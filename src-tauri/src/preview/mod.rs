pub mod hook;
pub mod route;
pub mod window;

use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Manager};

pub use hook::{remove_keyboard_hook, set_keyboard_hook};
pub use route::WebRoute;
pub use window::PreviewFile;

/// 全局 AppHandle 存储，供键盘钩子回调使用。
static PREVIEW_INSTANCE: LazyLock<Mutex<Option<PreviewFile>>> = LazyLock::new(|| Mutex::new(None));

pub fn set_global_app(app: AppHandle) {
    if let Ok(mut guard) = PREVIEW_INSTANCE.lock() {
        *guard = Some(PreviewFile { app_handle: Some(app) });
    }
}

pub fn get_global_app() -> Option<AppHandle> {
    PREVIEW_INSTANCE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref()?.app_handle.clone())
}

#[derive(Debug, Clone, Default)]
pub struct PreviewStateInner {
    pub input_path: String,
}

pub type PreviewState = Mutex<PreviewStateInner>;

pub fn init_preview_file(handle: AppHandle) {
    set_keyboard_hook();
    set_global_app(handle.clone());

    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));
}
