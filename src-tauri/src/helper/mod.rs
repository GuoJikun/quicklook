use tauri::{
    webview::WebviewWindow, AppHandle, Error as TError, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub mod audio;
pub mod config;
pub mod monitor;
pub mod selected_file;
#[cfg(windows)]
pub mod win;

#[allow(unused)]
pub fn get_webview_window(
    app: &AppHandle,
    label: &str,
    url: &str,
) -> Result<WebviewWindow, TError> {
    match app.get_webview_window(label) {
        Some(window) => Ok(window),
        None => WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
            .center()
            .auto_resize()
            .build(),
    }
}
#[allow(unused)]
pub fn get_scaled_size(size: f64, scale: f64) -> f64 {
    size / scale
}

/// 检测当前会话是否为 Wayland
#[cfg(target_os = "linux")]
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}
