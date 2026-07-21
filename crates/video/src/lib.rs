use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use quicklook_error::QuickLookError;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPreviewDecision {
    pub is_direct_playback: bool,
    pub preview_path: String,
}

/// 全局记录正在运行的 ffmpeg 进程 PID 及其对应的临时目录，用于取消时终止进程并清理。
static FFMPEG_PROCESS: LazyLock<Mutex<Option<(u32, PathBuf)>>> = LazyLock::new(|| Mutex::new(None));
static FFMPEG_CANCELLED: AtomicBool = AtomicBool::new(false);

/// 检测本机是否安装了 ffmpeg
pub fn check_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 预检查视频是否可直接原生播放。
/// 仅当文件格式/编码都满足兼容性要求时，返回原视频路径；否则返回需要转码的信号。
pub fn prepare_video_for_preview(path: &str) -> Result<VideoPreviewDecision, QuickLookError> {
    if !check_ffmpeg() {
        return Err(QuickLookError::FfmpegNotFound);
    }

    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let codec = probe_video_codec(path)?;
    log::info!("prepare_video_for_preview: path={:?}, ext={}, codec={}", path, extension, codec);
    let is_compatible = is_compatible_video(path, &extension, &codec);

    if is_compatible {
        return Ok(VideoPreviewDecision {
            is_direct_playback: true,
            preview_path: path.to_string_lossy().to_string(),
        });
    }

    Ok(VideoPreviewDecision {
        is_direct_playback: false,
        preview_path: String::new(),
    })
}

/// 将视频转换为 HLS (m3u8) 格式以供播放。
/// 如果视频已经是 h264 编码，则直接封装为 HLS；否则先转码为 h264 再封装。
/// 返回生成的 m3u8 文件路径。
pub fn convert_video_to_hls(path: &str) -> Result<String, QuickLookError> {
    FFMPEG_CANCELLED.store(false, Ordering::Relaxed);

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // 确认 ffmpeg 可用
    if !check_ffmpeg() {
        return Err(QuickLookError::FfmpegNotFound);
    }

    // 根据文件路径生成唯一临时目录（同一文件复用缓存）
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    let mut videos_dir = std::env::temp_dir();
    videos_dir.push("quicklook_videos");
    std::fs::create_dir_all(&videos_dir)?;

    let mut temp_dir = videos_dir;
    temp_dir.push(format!("quicklook_hls_{:x}", hash));

    let m3u8_path = temp_dir.join("index.m3u8");
    let m3u8_result = m3u8_path.to_string_lossy().to_string();

    // 同一文件若已在转换中，直接等待播放列表就绪并返回，避免重复启动 ffmpeg。
    {
        let guard = FFMPEG_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((pid, running_dir)) = guard.as_ref() {
            if *running_dir == temp_dir {
                log::info!(
                    "检测到同一路径已有 ffmpeg 正在运行 (PID: {})，等待 m3u8 就绪后复用",
                    pid
                );
                drop(guard);

                for _ in 0..120 {
                    if FFMPEG_CANCELLED.load(Ordering::Relaxed) {
                        return Err(QuickLookError::VideoConversionCancelled);
                    }
                    if m3u8_path.exists() {
                        return Ok(m3u8_result);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                return Err(QuickLookError::VideoConversion(
                    "ffmpeg 正在运行，但 m3u8 尚未就绪，请稍后重试".to_string(),
                ));
            }
        }
    }

    // 如果已经转换过，优先复用缓存；但要避免历史版本留下的错误分片路径，且必须是绝对分片 URI。
    if m3u8_path.exists() {
        let should_rebuild = std::fs::read_to_string(&m3u8_path)
            .map(|content| {
                let has_invalid_backslash = content.contains(":\\") || content.contains('\\');
                let has_absolute_segment_uri = content.lines().any(|line| {
                    let l = line.trim();
                    if l.is_empty() || l.starts_with('#') {
                        return false;
                    }
                    l.contains("://")
                        || (l.len() > 2
                            && l.as_bytes()[1] == b':'
                            && (l.as_bytes()[2] == b'/' || l.as_bytes()[2] == b'\\'))
                        || l.starts_with('/')
                });
                has_invalid_backslash || !has_absolute_segment_uri
            })
            .unwrap_or(true);

        if !should_rebuild {
            log::info!("命中 HLS 缓存: {:?}", m3u8_path);
            return Ok(m3u8_result);
        }

        log::warn!("检测到旧版 HLS 缓存路径格式异常，准备重建: {:?}", m3u8_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    std::fs::create_dir_all(&temp_dir)?;

    // 用 ffprobe 检测视频流编解码器
    let codec = probe_video_codec(Path::new(path))?;
    log::info!("检测到视频编解码器: {}", codec);

    // 如果已是 h264，直接复制视频流；否则转码为 libx264
    let video_codec = if codec == "h264" { "copy" } else { "libx264" };

    let mut hls_base_url = format!("{}", temp_dir.to_string_lossy().replace('\\', "/"));
    if !hls_base_url.ends_with('/') {
        hls_base_url.push('/');
    }

    let mut ffmpeg = std::process::Command::new("ffmpeg");
    ffmpeg
        .current_dir(&temp_dir)
        .arg("-i")
        .arg(path)
        .arg("-c:v")
        .arg(video_codec);

    // 转码时固定像素格式和 profile，提升浏览器/MSE 兼容性；copy 模式保持原编码。
    if video_codec != "copy" {
        ffmpeg
            .arg("-preset")
            .arg("veryfast")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-profile:v")
            .arg("main")
            .arg("-level")
            .arg("4.0");
    }

    let mut child = ffmpeg
        .args([
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-ac",
            "2",
            "-ar",
            "48000",
            "-hls_time",
            "2",
            "-hls_list_size",
            "0",
            "-hls_flags",
            "independent_segments+append_list",
            "-hls_base_url",
            &hls_base_url,
            "-hls_segment_filename",
            "seg_%03d.ts",
            "-f",
            "hls",
            "index.m3u8",
        ])
        .spawn()
        .map_err(|e| QuickLookError::VideoConversion(format!("启动 ffmpeg 失败: {}", e)))?;

    // 记录 PID 和临时目录，以便在取消时终止进程
    {
        let mut guard = FFMPEG_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some((child.id(), temp_dir.clone()));
    }

    let child_pid = child.id();
    let temp_dir_for_wait = temp_dir.clone();
    std::thread::spawn(move || {
        let status = child.wait();

        {
            let mut guard = FFMPEG_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((pid, _)) = guard.as_ref() {
                if *pid == child_pid {
                    *guard = None;
                }
            }
        }

        match status {
            Ok(exit) if exit.success() => {
                log::info!("HLS 转换完成，PID: {}", child_pid);
            },
            Ok(exit) => {
                let code = exit.code().unwrap_or(-1);
                log::error!("ffmpeg 转换失败，PID: {}, 退出码: {}", child_pid, code);
                let _ = std::fs::remove_dir_all(&temp_dir_for_wait);
            },
            Err(e) => {
                log::error!("等待 ffmpeg 进程失败，PID: {}, 错误: {}", child_pid, e);
                let _ = std::fs::remove_dir_all(&temp_dir_for_wait);
            },
        }
    });

    // 边转边播：等待首个播放列表文件生成后立即返回给前端。
    for _ in 0..120 {
        if FFMPEG_CANCELLED.load(Ordering::Relaxed) {
            log::info!("ffmpeg 转换已取消，提前结束等待");
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(QuickLookError::VideoConversionCancelled);
        }
        if m3u8_path.exists() {
            log::info!("m3u8 已就绪，开始边转边播: {:?}", m3u8_path);
            return Ok(m3u8_result);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // m3u8 在 12 秒内未生成：终止进程、清理临时目录并向前端报错。
    log::error!("ffmpeg 已启动，但 m3u8 生成超时，正在终止进程并清理临时文件");
    kill_ffmpeg_process();
    let _ = std::fs::remove_dir_all(&temp_dir);
    Err(QuickLookError::VideoConversion(
        "ffmpeg 已启动，但 m3u8 生成超时".to_string(),
    ))
}

fn probe_video_codec(path: &Path) -> Result<String, QuickLookError> {
    let output = match std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "csv=p=0",
            path.to_string_lossy().as_ref(),
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            log::warn!("ffprobe 执行失败: {}，将使用转码模式", e);
            return Ok(String::new());
        },
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("ffprobe 退出异常: {}，将使用转码模式", stderr.trim());
        return Ok(String::new());
    }

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(e) => {
            log::warn!("ffprobe 输出解析失败: {}，将使用转码模式", e);
            return Ok(String::new());
        },
    };

    let codec = stdout.trim().to_lowercase();
    log::info!("ffprobe stdout='{}'", codec);
    Ok(codec)
}

fn is_compatible_video(_path: &Path, extension: &str, codec: &str) -> bool {
    if extension.is_empty() || codec.is_empty() {
        return false;
    }

    extension == "mp4" && codec == "h264"
}

/// 从全局取出正在运行的 ffmpeg 进程记录，终止该进程并删除临时目录。
/// 如果当前没有记录则直接返回。
fn kill_ffmpeg_process() {
    let entry = FFMPEG_PROCESS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();

    if let Some((pid, temp_dir)) = entry {
        log::info!("正在终止 ffmpeg 进程 (PID: {})", pid);
        // 使用 taskkill 强制结束进程（Windows 平台）
        let result = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
        match result {
            Ok(out) if out.status.success() => {
                log::info!("ffmpeg 进程 (PID: {}) 已终止", pid);
            },
            Ok(out) => {
                log::warn!(
                    "终止 ffmpeg 进程时出现警告: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            },
            Err(e) => {
                log::error!("终止 ffmpeg 进程失败: {}", e);
            },
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
        log::info!("已清理临时目录: {:?}", temp_dir);
    }
}

/// 取消正在进行的 ffmpeg 视频转换，清理临时文件。
pub fn cancel_video_conversion() {
    FFMPEG_CANCELLED.store(true, Ordering::Relaxed);
    kill_ffmpeg_process();
}

/// 清理所有由 quicklook 生成的 ffmpeg HLS 转码缓存目录。
/// 返回被删除的目录数量。
pub fn clear_ffmpeg_cache() -> Result<u32, QuickLookError> {
    let videos_dir = std::env::temp_dir().join("quicklook_videos");
    if !videos_dir.exists() {
        log::info!("quicklook_videos 目录不存在，无需清理");
        return Ok(0);
    }
    let entries = std::fs::read_dir(&videos_dir)?;

    let mut removed = 0u32;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("quicklook_hls_") && entry.path().is_dir() {
            match std::fs::remove_dir_all(entry.path()) {
                Ok(_) => {
                    removed += 1;
                    log::info!("已清理缓存目录: {}", entry.path().display());
                },
                Err(e) => {
                    log::warn!("清理缓存目录失败: {}, 错误: {}", entry.path().display(), e);
                },
            }
        }
    }
    log::info!("共清理 {} 个 ffmpeg 缓存目录", removed);
    Ok(removed)
}
