use crate::error::QuickLookError;
use crate::helper::{ffmp, monitor, win};
use log::LevelFilter;
use quicklook_docs::pdf as pdf_helper;
use tauri::{command, AppHandle, Manager};
use windows::Win32::Foundation::HWND;

#[command]
pub fn set_log_level(level: usize) -> Result<(), QuickLookError> {
    let level_filter = match level {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };
    log::set_max_level(level_filter);
    Ok(())
}

#[command]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().map_or(HWND::default(), |hwnd| hwnd);
        let _ = win::show_open_with_dialog(path, hwnd);
    }
}

#[command]
pub fn get_monitor_info() -> monitor::MonitorInfo {
    monitor::get_monitor_info()
}

#[command]
pub fn get_default_program_name(path: &str) -> Result<String, QuickLookError> {
    win::get_default_program_name(path)
}

/// 汇总清理所有 quicklook 产生的缓存，包含 ffmpeg HLS 转码缓存、图片转码缓存和 PDF 渲染缓存。
/// 返回被删除的目录/文件总数量。
#[command]
pub fn clear_cache() -> Result<u32, QuickLookError> {
    let mut total = 0u32;
    total += ffmp::clear_ffmpeg_cache()?;
    total += crate::commands::image::clear_image_cache()?;
    total += pdf_helper::clear_pdf_cache()?;
    log::info!("缓存清理完成，共删除 {} 个目录/文件", total);
    Ok(total)
}
