use tauri::{
    webview::WebviewWindow, AppHandle, Error as TError, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub mod audio;
pub mod config;
pub mod monitor;
pub mod selected_file;
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
