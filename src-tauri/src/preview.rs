use std::sync::{Arc, LazyLock, Mutex};
use tauri::{
    webview::PageLoadEvent, AppHandle, Error as TauriError, Manager, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse, WindowsAndMessaging},
};

#[path = "./helper/mod.rs"]
mod helper;
use helper::{monitor, selected_file::Selected};

#[path = "./utils/mod.rs"]
mod utils;
use utils::{get_file_info, File as UFile};

#[derive(Debug, Clone)]
pub struct PreviewFile {
    hook_handle: Option<WindowsAndMessaging::HHOOK>, // 钩子的句柄
    app_handle: Option<AppHandle>,
}
// SAFETY: PreviewFile 的 hook_handle 仅在主线程设置与访问（通过 keyboard_proc 回调），
// app_handle（AppHandle）本身是 Send+Sync 的。全局实例通过 Mutex<Arc> 保护，
// 实际访问只发生在键盘钩子回调中（主线程），因此 Send+Sync 是合理的。
unsafe impl Send for PreviewFile {}
unsafe impl Sync for PreviewFile {}

pub struct WebRoute {
    path: String,
    query: UFile,
}
impl WebRoute {
    pub fn to_url(&self) -> String {
        let mut url = self.path.clone();
        url.push_str("?");
        url.push_str(
            format!(
                "file_type={}&path={}&extension={}&size={}&last_modified={}&name={}",
                self.query.get_file_type(),
                urlencoding::encode(&self.query.get_path()),
                self.query.get_extension(),
                self.query.get_size(),
                self.query.get_last_modified(),
                self.query.get_name()
            )
            .as_str(),
        );
        url
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

#[allow(dead_code)]
impl PreviewFile {
    // 注册键盘钩子
    pub fn set_keyboard_hook(&mut self) {
        let hook_ex = unsafe {
            WindowsAndMessaging::SetWindowsHookExW(
                WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(Self::keyboard_proc), // 使用结构体的键盘回调
                None,                      // 当前进程实例句柄
                0,
            )
        };
        match hook_ex {
            Ok(result) => {
                self.hook_handle = Some(result);
            },
            Err(_) => {
                self.hook_handle = None;
            },
        }
    }

    // 取消键盘钩子
    pub fn remove_keyboard_hook(&mut self) {
        if let Some(hook) = self.hook_handle {
            unsafe {
                let _ = WindowsAndMessaging::UnhookWindowsHookEx(hook);
            }
            self.hook_handle = None;
        }
    }

    // 按键处理逻辑
    pub fn handle_key_down(&self, vk_code: u32) {
        if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32 {
            let result = Self::preview_file(self.app_handle.clone().unwrap());
            if result.is_err() {
                log::error!("Error: {:?}", result.err().unwrap());
            }
        }
    }

    // 全局键盘钩子的回调函数
    extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // 确保消息被传递给其他应用程序
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

                // 获取 PreviewFile 实例并处理按键事件
                if let Some(app) = get_global_instance() {
                    app.handle_key_down(vk_code);
                }
            }
        }

        next_hook_result
    }
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
            width = helper::get_scaled_size(width, scale);
            height = helper::get_scaled_size(height, scale);
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
        let file_path = Selected::new();
        if file_path.is_ok() {
            let file_path = file_path.unwrap();

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

            let file_info = get_file_info(&file_path, &custom_code_exts, &custom_video_exts);

            let preview_state = app.state::<PreviewState>();
            let mut preview_state = preview_state.lock().unwrap();
            preview_state.input_path = file_path.clone();

            if file_info.is_none() {
                return Ok(());
            }

            let file_info = file_info.unwrap();
            let type_str = file_info.get_file_type();

            let (width, height) = Self::calc_window_size(&type_str);
            let route = WebRoute::get_route(&type_str, file_info);

            match app.get_webview_window("preview") {
                Some(window) => {
                    let url = route.to_url();
                    let js = format!("window.location.href = '{}'", &url);
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
                                    let js = format!("window.location.href = '{}'", &url);
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
        } else {
            log::error!("Error: {:?}", file_path.err().unwrap());
        }

        Ok(())
    }

    pub fn new() -> Self {
        Self { hook_handle: None, app_handle: None }
    }
}

static PREVIEW_INSTANCE: LazyLock<Mutex<Option<Arc<PreviewFile>>>> =
    LazyLock::new(|| Mutex::new(None));
// 函数用于设置全局 PreviewFile 实例
pub fn set_global_instance(instance: PreviewFile) {
    if let Ok(mut handle) = PREVIEW_INSTANCE.lock() {
        *handle = Some(Arc::new(instance));
    }
}
// 函数用于获取全局 PreviewFile 实例
fn get_global_instance() -> Option<Arc<PreviewFile>> {
    if let Ok(guard) = PREVIEW_INSTANCE.lock() {
        guard.clone()
    } else {
        None
    }
}

impl Drop for PreviewFile {
    fn drop(&mut self) {
        println!("Dropping PreviewFile instance");
        self.remove_keyboard_hook();
    }
}

impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreviewStateInner {
    input_path: String,
}

pub type PreviewState = Mutex<PreviewStateInner>;

//noinspection ALL
// 公开一个全局函数来初始化 PreviewFile
pub fn init_preview_file(handle: AppHandle) {
    let mut preview_file = PreviewFile::default();
    preview_file.set_keyboard_hook();
    preview_file.app_handle = Some(handle.clone());

    // 将实例存储在全局变量中
    set_global_instance(preview_file);

    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));
}
