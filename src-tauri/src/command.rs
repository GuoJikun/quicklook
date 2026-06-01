use log::{set_max_level, LevelFilter};
use quicklook_archive::{extractors, Extract};
use quicklook_docs as docs;
use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};
use windows::Win32::Foundation::HWND;

use crate::helper::{audio, ffmp, monitor, win};

#[command]
pub fn check_ffmpeg() -> bool {
    ffmp::check_ffmpeg()
}

#[command]
pub async fn convert_video_to_hls(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ffmp::convert_video_to_hls(&path))
        .await
        .map_err(|e| format!("转码任务执行失败: {}", e))?
}

#[command]
pub fn cancel_video_conversion() {
    ffmp::cancel_video_conversion()
}

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
    let file_bytes =
        std::fs::read(path).map_err(|e| format!("psd:: 读取文件失败: {}", e))?;

    let psd_obj = psd::Psd::from_bytes(&file_bytes)
        .map_err(|e| format!("psd:: 从 bytes 解析错误: {}", e))?;

    let rgba = psd_obj.rgba();
    let width = psd_obj.width();
    let height = psd_obj.height();
    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "psd:: 构建 RgbaImage 失败".to_string())?;

    let mut temp_path: PathBuf = std::env::temp_dir();
    temp_path.push("quicklook_psd_preview.png");

    img.save_with_format(&temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(temp_path.to_string_lossy().to_string())
}

#[command]
pub fn image_to_png(path: &str) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("image:: 读取图片失败: {}", e))?;

    let img = img.to_rgba8();

    let mut temp_path: PathBuf = std::env::temp_dir();
    temp_path.push("quicklook_image_preview.png");

    img.save_with_format(&temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

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

/// 汇总清理所有 quicklook 产生的缓存，目前包含 ffmpeg HLS 转码缓存。
/// 返回被删除的目录/文件总数量。
#[command]
pub fn clear_cache() -> Result<u32, String> {
    let mut total = 0u32;
    total += ffmp::clear_ffmpeg_cache()?;
    log::info!("缓存清理完成，共删除 {} 个目录/文件", total);
    Ok(total)
}
