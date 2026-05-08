use log::{set_max_level, LevelFilter};
use quicklook_archive::{extractors, Extract};
use quicklook_docs as docs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tauri::{command, AppHandle, Manager};
use windows::Win32::Foundation::HWND;

#[path = "helper/mod.rs"]
mod helper;
use helper::{audio, monitor, win};
// use helper::{archives, docs, ffmp, monitor, win};

#[command]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().map_or(HWND::default(), |hwnd| hwnd);
        let _ = win::show_open_with_dialog(path, hwnd);
    }
}

#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<Extract>, String> {
    log::info!("开始处理压缩文件: {}, 扩展名: {}", path, mode);
    let result = match mode {
        "zip" => extractors::zip::zip_extract(path).map_err(|e| e.to_string()),
        "tar" => extractors::tar::list_tar_entries(path).map_err(|e| e.to_string()),
        "gz" | "tgz" => extractors::tar::list_tar_gz_entries(path).map_err(|e| e.to_string()),
        "bz2" | "tbz2" => extractors::tar::list_tar_bz2_entries(path).map_err(|e| e.to_string()),
        "xz" | "txz" => extractors::tar::list_tar_xz_entries(path).map_err(|e| e.to_string()),
        "7z" => extractors::sevenz::list_7z_entries(path).map_err(|e| e.to_string()),
        _ => Err("不支持的压缩格式".to_string()),
    };

    match &result {
        Ok(entries) => {
            log::info!("成功处理压缩文件，共{}个条目", entries.len());
        },
        Err(e) => {
            log::error!("压缩文件处理失败: {}", e);
        },
    }

    result
}

#[command]
pub fn document(path: &str, mode: &str) -> Result<docs::Docs, String> {
    match mode {
        "csv" => docs::Docs::csv(path).map_err(|e| e.to_string()),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            docs::Docs::excel(path).map_err(|e| e.to_string())
        },
        "docx" => docs::Docs::docx(path).map_err(|e| e.to_string()),
        _ => Err("Not Support".to_string()),
    }
}

#[command]
pub fn get_monitor_info() -> monitor::MonitorInfo {
    monitor::get_monitor_info()
}

#[command]
pub fn get_default_program_name(path: &str) -> Result<String, String> {
    win::get_default_program_name(path)
}

#[command]
pub fn set_log_level(level: usize) -> Result<(), String> {
    let level_filter = match level {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };
    set_max_level(level_filter);
    Ok(())
}

#[command]
pub fn psd_to_png(path: &str) -> Result<String, String> {
    let file_bytes = std::fs::read(path);

    if file_bytes.is_err() {
        log::info!("psd:: 读取文件失败")
    }

    let psd_obj = psd::Psd::from_bytes(&*file_bytes.unwrap());
    if psd_obj.is_err() {
        log::info!("psd:: 从 bytes 解析 错误")
    }
    let psd_obj = psd_obj.unwrap();

    let rgba = psd_obj.rgba();
    // 封装成 RgbaImage
    let width = psd_obj.width();
    let height = psd_obj.height();
    let img = image::RgbaImage::from_raw(width, height, rgba);

    // Windows 临时目录
    let mut temp_path: PathBuf = std::env::temp_dir();
    temp_path.push("quicklook_psd_preview.png"); // 固定文件名

    // 保存为 PNG
    img.unwrap()
        .save_with_format(&temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    // 返回文件路径给前端
    Ok(temp_path.to_string_lossy().to_string())
}

#[command]
pub fn read_audio_info(path: &str) -> Option<audio::MusicInfo> {
    audio::read_music_info(path)
}

#[command]
pub fn parse_lrc(path: &str) -> Result<audio::Lrc, String> {
    audio::parse_lrc(path)
}

/// 全局记录正在运行的 ffmpeg 进程 PID 及其对应的临时目录，用于取消时终止进程并清理。
static FFMPEG_PROCESS: LazyLock<Mutex<Option<(u32, PathBuf)>>> = LazyLock::new(|| Mutex::new(None));

/// 检测本机是否安装了 ffmpeg
#[command]
pub fn check_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 将视频转换为 HLS (m3u8) 格式以供播放。
/// 如果视频已经是 h264 编码，则直接封装为 HLS；否则先转码为 h264 再封装。
/// 返回生成的 m3u8 文件路径。
#[command]
pub fn convert_video_to_hls(path: &str) -> Result<String, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // 确认 ffmpeg 可用
    if !check_ffmpeg() {
        return Err("ffmpeg 未找到，请确保 ffmpeg 已安装并添加到 PATH 中".to_string());
    }

    // 根据文件路径生成唯一临时目录（同一文件复用缓存）
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    let mut temp_dir = std::env::temp_dir();
    temp_dir.push(format!("quicklook_hls_{:x}", hash));

    let m3u8_path = temp_dir.join("index.m3u8");
    let m3u8_result = m3u8_path.to_string_lossy().to_string();

    // 同一文件若已在转换中，直接等待播放列表就绪并返回，避免重复启动 ffmpeg。
    {
        let guard = FFMPEG_PROCESS.lock().unwrap();
        if let Some((pid, running_dir)) = guard.as_ref() {
            if *running_dir == temp_dir {
                log::info!(
                    "检测到同一路径已有 ffmpeg 正在运行 (PID: {})，等待 m3u8 就绪后复用",
                    pid
                );
                drop(guard);

                for _ in 0..120 {
                    if m3u8_path.exists() {
                        return Ok(m3u8_result);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                return Err("ffmpeg 正在运行，但 m3u8 尚未就绪，请稍后重试".to_string());
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

    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // 用 ffprobe 检测视频流编解码器
    let codec_result = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokeys=1",
            path,
        ])
        .output();
    let codec = match codec_result {
        Ok(out) => match String::from_utf8(out.stdout) {
            Ok(s) => s.trim().to_lowercase(),
            Err(e) => {
                log::warn!("ffprobe 输出解析失败: {}，将使用转码模式", e);
                String::new()
            },
        },
        Err(e) => {
            log::warn!("ffprobe 执行失败: {}，将使用转码模式", e);
            String::new()
        },
    };
    log::info!("检测到视频编解码器: {}", codec);

    // 如果已是 h264，直接复制视频流；否则转码为 libx264
    let video_codec = if codec == "h264" { "copy" } else { "libx264" };

    let seg_filename = "seg_%03d.ts".to_string();
    let m3u8_file_name = "index.m3u8".to_string();
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
            &seg_filename,
            "-f",
            "hls",
            &m3u8_file_name,
        ])
        .spawn()
        .map_err(|e| e.to_string())?;

    // 记录 PID 和临时目录，以便在取消时终止进程
    {
        let mut guard = FFMPEG_PROCESS
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = Some((child.id(), temp_dir.clone()));
    }

    let child_pid = child.id();
    let temp_dir_for_wait = temp_dir.clone();
    std::thread::spawn(move || {
        let status = child.wait();

        {
            let mut guard = FFMPEG_PROCESS
                .lock()
                .unwrap_or_else(|e| e.into_inner());
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
    Err("ffmpeg 已启动，但 m3u8 生成超时".to_string())
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
#[command]
pub fn cancel_video_conversion() {
    kill_ffmpeg_process();
}

/// 清理所有由 quicklook 生成的 ffmpeg HLS 转码缓存目录。
/// 返回被删除的目录数量。
#[command]
pub fn clear_ffmpeg_cache() -> Result<u32, String> {
    let temp_dir = std::env::temp_dir();
    let entries = std::fs::read_dir(&temp_dir).map_err(|e| e.to_string())?;

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
