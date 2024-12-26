use tauri::{
    webview::WebviewWindow, AppHandle, Error as TError, Manager, WebviewUrl, WebviewWindowBuilder
};

pub mod archives;
pub mod docs;
pub mod monitor;
pub mod win;
pub mod config;

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

pub fn get_scaled_size(size: f64, scale: f64) -> f64 {
    size * scale
}
