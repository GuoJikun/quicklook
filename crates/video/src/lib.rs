use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};

use quicklook_cache::{CacheDir, file_key};
use quicklook_error::QuickLookError;
use serde::Serialize;

const CACHE_DIR: &str = "quicklook_videos";

/// HLS 分片时长（秒），与 `-hls_time` 保持一致。
const HLS_SEGMENT_SECONDS: u32 = 2;
/// GOP 帧数：按 30fps 折算为 2 秒，保证分片边界落在关键帧上。
const HLS_GOP_FRAMES: u32 = 30 * HLS_SEGMENT_SECONDS;

/// 随包分发的二进制路径，由宿主在启动时注入。
/// 未注入时回退到 PATH 查找，兼容开发环境与用户自装 ffmpeg 的场景。
static FFMPEG_BIN: OnceLock<PathBuf> = OnceLock::new();
static FFPROBE_BIN: OnceLock<PathBuf> = OnceLock::new();

/// 注入随包分发的 ffmpeg / ffprobe 绝对路径。
/// `ffprobe` 缺失时保持 PATH 回退；仅在首次调用时生效，重复调用会被忽略。
pub fn set_bundled_binaries(ffmpeg: PathBuf, ffprobe: Option<PathBuf>) {
    let _ = FFMPEG_BIN.set(ffmpeg);
    if let Some(ffprobe) = ffprobe {
        let _ = FFPROBE_BIN.set(ffprobe);
    }
}

/// 当前实际使用的 ffmpeg 程序，仅用于日志与错误提示。
pub fn ffmpeg_program() -> String {
    match FFMPEG_BIN.get() {
        Some(path) => path.to_string_lossy().to_string(),
        None => "ffmpeg".to_string(),
    }
}

/// CREATE_NO_WINDOW：启动控制台子进程时不弹出终端窗口。
fn silent_command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.creation_flags(0x0800_0000);
    cmd
}

/// 优先使用随包分发的 ffmpeg，其次回退 PATH 中的同名可执行文件。
fn ffmpeg_command() -> Command {
    match FFMPEG_BIN.get() {
        Some(path) => silent_command(path),
        None => silent_command("ffmpeg"),
    }
}

fn ffprobe_command() -> Command {
    match FFPROBE_BIN.get() {
        Some(path) => silent_command(path),
        None => silent_command("ffprobe"),
    }
}

/// 视频编码器：独显硬件编码优先，其次核显，最后系统媒体组件软编。
///
/// 末位的 `MediaFoundation` 使用系统自带的 Media Foundation H.264 编码器（`h264_mf`），
/// 不依赖 libx264，因此整个分发包不引入 GPL 组件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoEncoder {
    Nvenc,
    Amf,
    Qsv,
    MediaFoundation,
}

impl VideoEncoder {
    pub fn codec_name(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Amf => "h264_amf",
            Self::Qsv => "h264_qsv",
            Self::MediaFoundation => "h264_mf",
        }
    }

    /// 失败后可回退到的通用编码器；`MediaFoundation` 已是最后一环，无更软的兜底。
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::MediaFoundation => None,
            _ => Some(Self::MediaFoundation),
        }
    }
}

/// 进程内缓存已选定的编码器，避免重复探测。
static SELECTED_ENCODER: LazyLock<Mutex<Option<VideoEncoder>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPreviewDecision {
    pub is_direct_playback: bool,
    pub preview_path: String,
}

/// ffprobe 探测结果。任一字段缺失时保持默认值，由调用方走保守转码路径。
#[derive(Debug, Default, Clone)]
struct CodecProbe {
    video: String,
    audio: String,
    width: u32,
    height: u32,
}

/// 全局记录正在运行的 ffmpeg 进程 PID 及其对应的临时目录，用于取消时终止进程并清理。
static FFMPEG_PROCESS: LazyLock<Mutex<Option<(u32, PathBuf)>>> = LazyLock::new(|| Mutex::new(None));
static FFMPEG_CANCELLED: AtomicBool = AtomicBool::new(false);

/// 检测 ffmpeg 是否可用：优先随包分发的二进制，其次 PATH。
pub fn check_ffmpeg() -> bool {
    let available = ffmpeg_command()
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    log::info!(
        "ffmpeg 检测: program={}, available={}",
        ffmpeg_program(),
        available
    );
    available
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

    let probe = probe_codecs(path)?;
    log::info!(
        "prepare_video_for_preview: path={:?}, ext={}, codec={}, size={}x{}",
        path,
        extension,
        probe.video,
        probe.width,
        probe.height
    );
    let is_compatible = is_compatible_video(path, &extension, &probe.video);

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

    // 确认 ffmpeg 可用
    if !check_ffmpeg() {
        return Err(QuickLookError::FfmpegNotFound);
    }

    // 缓存 key：路径 + size + mtime，文件被覆盖修改后不会误命中旧 HLS
    let cache = CacheDir::new(CACHE_DIR);
    cache.ensure()?;
    let temp_dir = cache
        .path()
        .join(format!("quicklook_hls_{:x}", file_key(path)));

    let m3u8_path = temp_dir.join("index.m3u8");
    let m3u8_result = m3u8_path.to_string_lossy().to_string();

    // 同一文件若已在转换中，直接等待播放列表就绪并返回，避免重复启动 ffmpeg。
    {
        let guard = FFMPEG_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((pid, running_dir)) = guard.as_ref()
            && *running_dir == temp_dir
        {
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

    // 用 ffprobe 检测视频/音频流编解码器与分辨率
    let probe = probe_codecs(Path::new(path))?;
    log::info!(
        "检测到编解码器: video={}, audio={}, size={}x{}",
        probe.video,
        probe.audio,
        probe.width,
        probe.height
    );

    // 如果已是 h264，直接复制视频流；否则按 独显 → 核显 → 系统媒体组件 选编码器转码。
    let video_encoder = if probe.video == "h264" {
        None
    } else {
        Some(select_encoder()?)
    };

    let mut hls_base_url = temp_dir.to_string_lossy().replace('\\', "/").to_string();
    if !hls_base_url.ends_with('/') {
        hls_base_url.push('/');
    }

    let mut ffmpeg = ffmpeg_command();
    ffmpeg.current_dir(&temp_dir).arg("-i").arg(path);

    // 转码时固定像素格式和 profile，提升浏览器/MSE 兼容性；copy 模式保持原编码。
    if let Some(encoder) = video_encoder {
        apply_video_encode_args(&mut ffmpeg, encoder, probe.width, probe.height);
    } else {
        ffmpeg.arg("-c:v").arg("copy");
    }

    // 音频已是 aac 时直接复制，避免二次有损转码；否则转为 aac。
    if probe.audio == "aac" {
        ffmpeg.arg("-c:a").arg("copy");
    } else {
        ffmpeg.args(["-c:a", "aac", "-b:a", "128k", "-ac", "2", "-ar", "48000"]);
    }

    let mut child = match ffmpeg
        .args([
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
    {
        Ok(child) => child,
        Err(e) => {
            // 硬件编码器启动失败时回退到通用编码器再试一次
            if let Some(encoder) = video_encoder
                && let Some(next) = encoder.fallback()
            {
                log::warn!(
                    "编码器 {} 启动失败: {}，回退 {} 重试",
                    encoder.codec_name(),
                    e,
                    next.codec_name()
                );
                set_selected_encoder(next);
                let _ = std::fs::remove_dir_all(&temp_dir);
                return convert_video_to_hls(path);
            }
            return Err(QuickLookError::VideoConversion(format!(
                "启动 ffmpeg 失败: {}",
                e
            )));
        },
    };

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
            if let Some((pid, _)) = guard.as_ref()
                && *pid == child_pid
            {
                *guard = None;
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

    // m3u8 在 12 秒内未生成：终止进程、清理临时目录；硬件编码则回退通用编码重试。
    log::error!("ffmpeg 已启动，但 m3u8 生成超时，正在终止进程并清理临时文件");
    kill_ffmpeg_process();
    let _ = std::fs::remove_dir_all(&temp_dir);

    if let Some(encoder) = video_encoder
        && let Some(next) = encoder.fallback()
        && !FFMPEG_CANCELLED.load(Ordering::Relaxed)
    {
        log::warn!(
            "编码器 {} 未产出 m3u8，回退 {} 重试",
            encoder.codec_name(),
            next.codec_name()
        );
        set_selected_encoder(next);
        return convert_video_to_hls(path);
    }

    Err(QuickLookError::VideoConversion(
        "ffmpeg 已启动，但 m3u8 生成超时".to_string(),
    ))
}

/// 探测首条视频流与音频流的编码名及视频分辨率。
/// 失败时返回默认值，由调用方走保守转码路径。
fn probe_codecs(path: &Path) -> Result<CodecProbe, QuickLookError> {
    let output = match ffprobe_command()
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name,width,height",
            "-of",
            "csv=p=0",
            path.to_string_lossy().as_ref(),
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            log::warn!("ffprobe 执行失败: {}，将使用转码模式", e);
            return Ok(CodecProbe::default());
        },
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("ffprobe 退出异常: {}，将使用转码模式", stderr.trim());
        return Ok(CodecProbe::default());
    }

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(e) => {
            log::warn!("ffprobe 输出解析失败: {}，将使用转码模式", e);
            return Ok(CodecProbe::default());
        },
    };

    let mut probe = CodecProbe::default();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(4, ',');
        let kind = parts.next().unwrap_or_default().trim().to_lowercase();
        let name = parts.next().unwrap_or_default().trim().to_lowercase();
        let width = parts
            .next()
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);
        let height = parts
            .next()
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

        match kind.as_str() {
            "video" if probe.video.is_empty() => {
                probe.video = name;
                probe.width = width;
                probe.height = height;
            },
            "audio" if probe.audio.is_empty() => probe.audio = name,
            _ => {},
        }
    }

    log::info!(
        "ffprobe 探测结果: video='{}', audio='{}', size={}x{}",
        probe.video,
        probe.audio,
        probe.width,
        probe.height
    );
    Ok(probe)
}

/// 通过 WMI 读取显卡名称列表，用于决定硬件编码器试探顺序。
fn detect_gpu_names() -> Vec<String> {
    let output = match silent_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }",
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            log::warn!("读取显卡信息失败: {}", e);
            return Vec::new();
        },
    };

    if !output.status.success() {
        log::warn!(
            "读取显卡信息失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn gpu_name_contains(names: &[String], keyword: &str) -> bool {
    let key = keyword.to_ascii_uppercase();
    names.iter().any(|n| n.to_ascii_uppercase().contains(&key))
}

/// 编码器候选顺序：独显（NVIDIA/AMD）→ 核显（Intel）→ 系统媒体组件。
fn encoder_candidates() -> Vec<VideoEncoder> {
    let names = detect_gpu_names();
    log::info!("检测到显卡: {:?}", names);

    let mut candidates = Vec::new();
    // 独显优先
    if gpu_name_contains(&names, "NVIDIA") {
        candidates.push(VideoEncoder::Nvenc);
    }
    if gpu_name_contains(&names, "AMD") || gpu_name_contains(&names, "ATI") {
        candidates.push(VideoEncoder::Amf);
    }
    // 核显
    if gpu_name_contains(&names, "INTEL") {
        candidates.push(VideoEncoder::Qsv);
    }
    // 显卡识别失败时仍按常见独显顺序试探，由试编过滤
    if candidates.is_empty() {
        candidates.extend([VideoEncoder::Nvenc, VideoEncoder::Amf, VideoEncoder::Qsv]);
    }
    // 最终兜底：系统自带的 Media Foundation 编码器
    candidates.push(VideoEncoder::MediaFoundation);
    candidates
}

/// 试编 1 帧，确认该编码器当前真能工作。
fn probe_encoder(encoder: VideoEncoder) -> bool {
    let ok = ffmpeg_command()
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=256x256:d=0.1",
            "-frames:v",
            "1",
            "-c:v",
            encoder.codec_name(),
            "-f",
            "null",
            "-",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        log::info!("编码器 {} 试编通过", encoder.codec_name());
    } else {
        log::warn!("编码器 {} 试编失败", encoder.codec_name());
    }
    ok
}

fn set_selected_encoder(encoder: VideoEncoder) {
    *SELECTED_ENCODER.lock().unwrap_or_else(|e| e.into_inner()) = Some(encoder);
}

/// 选定编码器并缓存：独显 → 核显 → 系统媒体组件，每个候选用试编确认。
/// 全部候选均不可用时返回 `VideoEncoderUnavailable`。
fn select_encoder() -> Result<VideoEncoder, QuickLookError> {
    {
        let guard = SELECTED_ENCODER.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(encoder) = *guard {
            return Ok(encoder);
        }
    }

    for candidate in encoder_candidates() {
        if probe_encoder(candidate) {
            log::info!("选用视频编码器: {}", candidate.codec_name());
            set_selected_encoder(candidate);
            return Ok(candidate);
        }
    }

    log::error!("所有候选编码器均试编失败，无法进行视频转码");
    Err(QuickLookError::VideoEncoderUnavailable)
}

/// 按分辨率给出目标码率。`h264_mf` 没有 CRF 等效项，必须显式指定码率。
pub fn target_bitrate(width: u32, height: u32) -> &'static str {
    match width.saturating_mul(height) {
        pixels if pixels >= 3840 * 2160 => "16M",
        pixels if pixels >= 2560 * 1440 => "8M",
        pixels if pixels >= 1920 * 1080 => "4M",
        pixels if pixels >= 1280 * 720 => "2M",
        _ => "1M",
    }
}

/// 按分辨率给出 H.264 level，1080p 以上需要 5.1。
fn target_level(width: u32, height: u32) -> &'static str {
    match width.saturating_mul(height) {
        pixels if pixels > 1920 * 1080 => "5.1",
        _ => "4.0",
    }
}

/// 为转码命令追加编码器及其参数（像素格式/profile 固定以保证浏览器兼容）。
fn apply_video_encode_args(ffmpeg: &mut Command, encoder: VideoEncoder, width: u32, height: u32) {
    ffmpeg.arg("-c:v").arg(encoder.codec_name());
    match encoder {
        VideoEncoder::MediaFoundation => {
            // mfenc 的 profile 只接受数字枚举值，传 "main" 会报 Invalid argument：
            // 66 = Baseline，77 = Main，100 = High。默认不指定时输出 Constrained Baseline。
            ffmpeg
                .args(["-pix_fmt", "nv12"])
                .args(["-profile:v", "77"])
                .arg("-level")
                .arg(target_level(width, height))
                .arg("-g")
                .arg(HLS_GOP_FRAMES.to_string())
                .args(["-rate_control", "u_vbr"])
                .args(["-quality", "70"])
                .arg("-b:v")
                .arg(target_bitrate(width, height));
        },
        VideoEncoder::Nvenc => {
            ffmpeg
                .arg("-preset")
                .arg("p4")
                .arg("-cq")
                .arg("23")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-profile:v")
                .arg("main")
                .arg("-level")
                .arg("4.0");
        },
        VideoEncoder::Qsv => {
            ffmpeg
                .arg("-preset")
                .arg("fast")
                .arg("-global_quality")
                .arg("23")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-profile:v")
                .arg("main")
                .arg("-level")
                .arg("4.0");
        },
        VideoEncoder::Amf => {
            ffmpeg
                .arg("-quality")
                .arg("speed")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-profile:v")
                .arg("main")
                .arg("-level")
                .arg("4.0");
        },
    }
}

pub fn is_compatible_video(_path: &Path, extension: &str, codec: &str) -> bool {
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
        let result = silent_command("taskkill")
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
    let removed = CacheDir::new(CACHE_DIR).clear_dirs_with_prefix("quicklook_hls_")?;
    log::info!("共清理 {} 个 ffmpeg 缓存目录", removed);
    Ok(removed)
}
