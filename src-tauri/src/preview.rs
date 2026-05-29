use std::sync::{LazyLock, Mutex};
use tauri::{
    webview::PageLoadEvent, AppHandle, Error as TauriError, Manager, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse, WindowsAndMessaging},
};

use crate::helper::{monitor, selected_file::Selected};

use crate::utils::{get_file_info, File as UFile};

// SAFETY: HookHandle 仅在主线程上设置与访问（键盘钩子回调），因此 Send+Sync 是安全的。
struct HookHandle(Option<WindowsAndMessaging::HHOOK>);
unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

static HOOK_HANDLE: LazyLock<Mutex<HookHandle>> =
    LazyLock::new(|| Mutex::new(HookHandle(None)));

fn set_keyboard_hook() {
    let hook_ex = unsafe {
        WindowsAndMessaging::SetWindowsHookExW(
            WindowsAndMessaging::WH_KEYBOARD_LL,
            Some(keyboard_proc),
            None,
            0,
        )
    };
    match hook_ex {
        Ok(hook) => {
            if let Ok(mut guard) = HOOK_HANDLE.lock() {
                guard.0 = Some(hook);
            }
        },
        Err(e) => {
            log::error!("设置键盘钩子失败: {:?}", e);
        },
    }
}

fn remove_keyboard_hook() {
    if let Ok(mut guard) = HOOK_HANDLE.lock() {
        if let Some(hook) = guard.0.take() {
            unsafe {
                let _ = WindowsAndMessaging::UnhookWindowsHookEx(hook);
            }
        }
    }
}

// 全局键盘钩子的回调函数
extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let next_hook_result =
        unsafe { WindowsAndMessaging::CallNextHookEx(None, ncode, wparam, lparam) };
    #[cfg(debug_assertions)]
    log::info!("Hook called - next_hook_result: {:?}", next_hook_result);

    if ncode >= 0
        && (wparam.0 == WindowsAndMessaging::WM_KEYDOWN as usize
            || wparam.0 == WindowsAndMessaging::WM_SYSKEYDOWN as usize)
    {
        let kb_struct = unsafe { *(lparam.0 as *const WindowsAndMessaging::KBDLLHOOKSTRUCT) };
        let vk_code = kb_struct.vkCode;

        if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32 {
            let type_str = Selected::get_focused_type();
            if type_str.is_none() {
                return next_hook_result;
            }

            if let Some(app) = get_global_app() {
                if let Err(e) = PreviewFile::preview_file(app) {
                    log::error!("Error: {:?}", e);
                }
            }
        }
    }

    next_hook_result
}

#[derive(Debug, Clone)]
pub struct PreviewFile {
    app_handle: Option<AppHandle>,
}

pub struct WebRoute {
    path: String,
    query: UFile,
}
impl WebRoute {
    pub fn to_url(&self) -> String {
        let query = [
            ("file_type", self.query.get_file_type()),
            ("path", self.query.get_path()),
            ("extension", self.query.get_extension()),
            ("size", self.query.get_size().to_string()),
            ("last_modified", self.query.get_last_modified().to_string()),
            ("name", self.query.get_name()),
        ];
        let encoded: Vec<String> = query
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect();
        format!("{}?{}", self.path, encoded.join("&"))
    }
    pub fn new(path: String, query: UFile) -> Self {
        Self { path, query }
    }
    pub fn get_route(type_str: &str, file_info: UFile) -> WebRoute {
        match type_str {
            "Markdown" => WebRoute::new("/preview/md".to_string(), file_info),
            "Image" => WebRoute::new("/preview/image".to_string(), file_info),
            "Audio" => WebRoute::new("/preview/audio".to_string(), file_info),
            "Video" => WebRoute::new("/preview/video".to_string(), file_info),
            "Font" => WebRoute::new("/preview/font".to_string(), file_info),
            "Code" => WebRoute::new("/preview/code".to_string(), file_info),
            "Book" => WebRoute::new("/preview/book".to_string(), file_info),
            "Archive" => WebRoute::new("/preview/archive".to_string(), file_info),
            "Doc" => WebRoute::new("/preview/document".to_string(), file_info),
            _ => WebRoute::new("/preview/not-support".to_string(), file_info),
        }
    }
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
        let file_path = match Selected::new() {
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

        let preview_state = app.state::<PreviewState>();
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
                let result = WebviewWindowBuilder::new(
                    &app,
                    "preview",
                    WebviewUrl::App("/preview".into()),
                )
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

static PREVIEW_INSTANCE: LazyLock<Mutex<Option<PreviewFile>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn set_global_app(app: AppHandle) {
    if let Ok(mut guard) = PREVIEW_INSTANCE.lock() {
        *guard = Some(PreviewFile { app_handle: Some(app) });
    }
}

fn get_global_app() -> Option<AppHandle> {
    PREVIEW_INSTANCE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref()?.app_handle.clone())
}

impl Drop for PreviewFile {
    fn drop(&mut self) {
        log::debug!("Dropping PreviewFile instance");
        remove_keyboard_hook();
    }
}

impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile { app_handle: None }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreviewStateInner {
    input_path: String,
}

pub type PreviewState = Mutex<PreviewStateInner>;

pub fn init_preview_file(handle: AppHandle) {
    set_keyboard_hook();
    set_global_app(handle.clone());

    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));
}
