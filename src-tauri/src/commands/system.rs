use crate::error::QuickLookError;
use crate::helper::{ffmp, monitor, win};
use log::LevelFilter;
use quicklook_docs::pdf as pdf_helper;
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{command, AppHandle, Manager};
use windows::Win32::Foundation::HWND;

/// 当前生效的日志级别（LevelFilter 的枚举值）。
/// 供 tauri_plugin_log 的 filter 在运行时读取，覆盖 webview 日志绕过 `log::set_max_level` 的情况。
pub static CURRENT_LOG_LEVEL: AtomicU8 = AtomicU8::new(LevelFilter::Warn as u8);

/// 将全局日志级别数字（与前端 LogLevel 枚举一致）映射为 LevelFilter
fn level_filter_of(level: usize) -> LevelFilter {
    match level {
        1 => LevelFilter::Trace,
        2 => LevelFilter::Debug,
        3 => LevelFilter::Info,
        4 => LevelFilter::Warn,
        5 => LevelFilter::Error,
        _ => LevelFilter::Off,
    }
}

#[command]
pub fn set_log_level(level: usize) -> Result<(), QuickLookError> {
    // 与前端 @tauri-apps/plugin-log 的 LogLevel 枚举保持一致：Trace=1 ... Error=5
    let level_filter = level_filter_of(level);
    CURRENT_LOG_LEVEL.store(level_filter as u8, Ordering::SeqCst);
    log::set_max_level(level_filter);
    Ok(())
}

/// 重启应用（tauri_plugin_log 不支持运行时变更日志级别，需重启后按配置生效）
#[command]
pub fn restart_app(app: AppHandle) {
    app.restart();
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
pub async fn get_default_program_name(path: String) -> Result<String, QuickLookError> {
    tokio::task::spawn_blocking(move || win::get_default_program_name(&path))
        .await
        .map_err(|e| QuickLookError::WindowsApi(format!("获取默认程序任务执行失败: {}", e)))?
}

/// 汇总清理所有 quicklook 产生的缓存，包含 ffmpeg HLS 转码缓存、图片转码缓存和 PDF 渲染缓存。
/// 返回被删除的目录/文件总数量。
#[command]
pub async fn clear_cache() -> Result<u32, QuickLookError> {
    tokio::task::spawn_blocking(|| {
        let mut total = 0u32;
        total += ffmp::clear_ffmpeg_cache()?;
        total += crate::commands::image::clear_image_cache_sync()?;
        total += pdf_helper::clear_pdf_cache()?;
        log::info!("缓存清理完成，共删除 {} 个目录/文件", total);
        Ok(total)
    })
    .await
    .map_err(|e| QuickLookError::Other(format!("缓存清理任务执行失败: {}", e)))?
}
