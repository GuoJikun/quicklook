use crate::error::QuickLookError;
use crate::helper::win;
use crate::i18n::{self, Locale};
use log::LevelFilter;
use quicklook_docs::pdf as pdf_helper;
use quicklook_image::clear_image_cache;
use quicklook_video::clear_ffmpeg_cache;
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{AppHandle, Emitter, Manager, command};
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

#[command]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().unwrap_or(HWND::default());
        let _ = win::show_open_with_dialog(path, hwnd);
    }
}

/// 切换界面语言：更新托盘菜单与窗口标题，并广播给所有窗口。
///
/// 语言本身由前端写入 `config.data`，这里再写一次是为了保证托盘在下次启动时
/// 读到与界面一致的值（前端 store 写入失败时不至于两边不一致）。
#[command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), QuickLookError> {
    let locale = Locale::from_tag(Some(&locale));

    if let Err(e) = i18n::save_locale(&app, locale) {
        log::warn!("保存语言失败: {e}");
    }

    let texts = locale.tray_texts();
    crate::tray::refresh_tray(&app);

    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_title(texts.settings);
    }
    if let Some(window) = app.get_webview_window("upgrade") {
        let _ = window.set_title(texts.update_window_title);
    }

    app.emit("locale-changed", locale.tag())
        .map_err(|e| QuickLookError::Other(format!("广播语言变更失败: {e}")))?;

    log::debug!("界面语言已切换为 {}", locale.tag());
    Ok(())
}

#[command]
pub async fn get_default_program_name(path: String) -> Result<String, QuickLookError> {
    tauri::async_runtime::spawn_blocking(move || win::get_default_program_name(&path))
        .await
        .map_err(|e| QuickLookError::WindowsApi(format!("获取默认程序任务执行失败: {}", e)))?
}

/// 汇总清理所有 quicklook 产生的缓存，包含 ffmpeg HLS 转码缓存、图片转码缓存和 PDF 渲染缓存。
/// 返回被删除的目录/文件总数量。
#[command]
pub async fn clear_cache() -> Result<u32, QuickLookError> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut total = 0u32;
        total += clear_ffmpeg_cache()?;
        total += clear_image_cache()?;
        total += pdf_helper::clear_pdf_cache()?;
        log::info!("缓存清理完成，共删除 {} 个目录/文件", total);
        Ok(total)
    })
    .await
    .map_err(|e| QuickLookError::Other(format!("缓存清理任务执行失败: {}", e)))?
}
