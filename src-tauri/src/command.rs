use log::{set_max_level, LevelFilter};
use quicklook_archive::{extractors, Extract};
use quicklook_docs as docs;
use std::path::PathBuf;
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

    // 如果已经转换过，直接返回缓存结果
    if m3u8_path.exists() {
        log::info!("命中 HLS 缓存: {:?}", m3u8_path);
        return Ok(m3u8_path.to_string_lossy().to_string());
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

    let seg_filename = temp_dir
        .join("seg_%03d.ts")
        .to_str()
        .ok_or_else(|| "临时目录路径包含无效字符".to_string())?
        .to_string();
    let m3u8_str = m3u8_path
        .to_str()
        .ok_or_else(|| "m3u8 路径包含无效字符".to_string())?;

    let output = std::process::Command::new("ffmpeg")
        .args([
            "-i",
            path,
            "-c:v",
            video_codec,
            "-c:a",
            "aac",
            "-hls_time",
            "4",
            "-hls_list_size",
            "0",
            "-hls_segment_filename",
            &seg_filename,
            "-f",
            "hls",
            m3u8_str,
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("ffmpeg 转换失败: {}", stderr);
        // 清理不完整的临时文件
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("ffmpeg 转换失败: {}", stderr));
    }

    log::info!("HLS 转换完成: {:?}", m3u8_path);
    Ok(m3u8_path.to_string_lossy().to_string())
}
