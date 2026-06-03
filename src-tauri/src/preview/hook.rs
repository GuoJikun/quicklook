use std::sync::{LazyLock, Mutex};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse, WindowsAndMessaging},
};

use crate::preview::{get_global_app, window::PreviewFile};

// SAFETY: HookHandle 仅在主线程上设置与访问（键盘钩子回调），因此 Send+Sync 是安全的。
struct HookHandle(Option<WindowsAndMessaging::HHOOK>);
unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

static HOOK_HANDLE: LazyLock<Mutex<HookHandle>> = LazyLock::new(|| Mutex::new(HookHandle(None)));

pub fn set_keyboard_hook() {
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

pub fn remove_keyboard_hook() {
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
            let type_str = crate::helper::selected_file::Selected::get_focused_type();
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
