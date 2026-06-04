use tauri::{
    webview::PageLoadEvent, AppHandle, Error as TauriError, Manager, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

use crate::helper::monitor;
use crate::preview::route::WebRoute;
use crate::utils::get_file_info;

#[derive(Debug, Clone)]
pub struct PreviewFile {
    pub app_handle: Option<AppHandle>,
}

impl PreviewFile {
    fn calc_window_size(file_type: &str) -> (f64, f64) {
        let monitor_info = monitor::get_monitor_info();

        let scale = monitor_info.scale;
        let mut width = 1000.0;
        let mut height = 600.0;

        if monitor_info.width > 0.0 {
            if file_type == "Audio" {
                width = 560.0;
                height = 200.0;
            } else {
                width = monitor_info.width * 0.8;
                height = monitor_info.height * 0.8;
            }
        }

        if monitor_info.scale > 1.0 {
            width = crate::helper::get_scaled_size(width, scale);
            height = crate::helper::get_scaled_size(height, scale);
        }

        log::info!(
            "Client Rect: width is {}, height is {}, scale is {}",
            width,
            height,
            scale
        );
        (width, height)
    }

    pub fn preview_file(app: AppHandle) -> Result<(), TauriError> {
        let file_path = match crate::helper::selected_file::Selected::new() {
            Ok(path) => path,
            Err(e) => {
                log::error!("获取选中文件失败: {:?}", e);
                return Ok(());
            },
        };

        // 从 store 读取用户自定义扩展名
        let store = match app.store("config.data") {
            Ok(store) => Some(store),
            Err(err) => {
                log::warn!("Failed to open config.data store: {:?}", err);
                None
            },
        };
        let custom_code_exts: Vec<String> = store
            .as_ref()
            .and_then(|s| s.get("customCodeExtensions"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let custom_video_exts: Vec<String> = store
            .as_ref()
            .and_then(|s| s.get("customVideoExtensions"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let file_info = match get_file_info(&file_path, &custom_code_exts, &custom_video_exts) {
            Some(info) => info,
            None => return Ok(()),
        };

        let preview_state = app.state::<crate::preview::PreviewState>();
        let mut preview_state = match preview_state.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("lock preview state failed: {}", e);
                return Ok(());
            },
        };
        preview_state.input_path = file_path;

        let type_str = file_info.get_file_type();
        let (width, height) = Self::calc_window_size(&type_str);
        let route = WebRoute::get_route(&type_str, file_info);

        match app.get_webview_window("preview") {
            Some(window) => {
                let url = route.to_url();
                let js_escaped = url.replace('\\', "\\\\").replace('\'', "\\'");
                let js = format!("window.location.href = '{}'", &js_escaped);
                let _ = window.eval(js.as_str());

                let _ = window.show();
                let _ = window.set_focus();
            },
            None => {
                let result =
                    WebviewWindowBuilder::new(&app, "preview", WebviewUrl::App("/preview".into()))
                        .title("Preview")
                        .center()
                        .devtools(cfg!(debug_assertions))
                        .decorations(false)
                        .skip_taskbar(false)
                        .auto_resize()
                        .inner_size(width, height)
                        .min_inner_size(300.0, 200.0)
                        .on_page_load(move |window, payload| {
                            let cur_path = payload.url().path();
                            if cur_path == "/preview" {
                                match payload.event() {
                                    PageLoadEvent::Finished => {
                                        let url = route.to_url();
                                        let js_escaped =
                                            url.replace('\\', "\\\\").replace('\'', "\\'");
                                        let js =
                                            format!("window.location.href = '{}'", &js_escaped);
                                        let _ = window.eval(js.as_str());

                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    },
                                    _ => {},
                                }
                            }
                        })
                        .focused(true)
                        .visible_on_all_workspaces(true)
                        .build();
                if let Ok(preview) = result {
                    let _ = preview.show();
                }
            },
        }

        Ok(())
    }
}

impl Drop for PreviewFile {
    fn drop(&mut self) {
        log::debug!("Dropping PreviewFile instance");
        super::hook::remove_keyboard_hook();
    }
}

impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile { app_handle: None }
    }
}
