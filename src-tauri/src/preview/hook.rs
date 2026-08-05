use std::sync::atomic::{AtomicBool, Ordering};
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

// 防止重复按键导致并发创建/导航预览窗口。
static PREVIEW_RUNNING: AtomicBool = AtomicBool::new(false);

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

        // 低级键盘钩子回调必须在极短时间内返回（超时会被系统静默卸载，且阻塞会影响全局
        // 键盘输入），因此这里只做快速判断（空格键 + 防重入），完整的预览流程
        // （COM 查询选中文件、窗口创建、导航）移到 spawn_blocking 异步执行。
        if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32
            && PREVIEW_RUNNING
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            if let Some(app) = get_global_app() {
                tauri::async_runtime::spawn_blocking(move || {
                    let result = (|| {
                        let type_str =
                            crate::helper::selected_file::Selected::get_focused_type();
                        if type_str.is_none() {
                            return Ok(());
                        }
                        PreviewFile::preview_file(app)
                    })();
                    if let Err(e) = result {
                        log::error!("Error: {:?}", e);
                    }
                    PREVIEW_RUNNING.store(false, Ordering::SeqCst);
                });
            } else {
                PREVIEW_RUNNING.store(false, Ordering::SeqCst);
            }
        }
    }

    next_hook_result
}
