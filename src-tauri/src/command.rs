use tauri::{command, AppHandle, Manager};
use windows::Win32::Foundation::HWND;

mod helper;

#[command]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().map_or(HWND::default(), |hwnd| hwnd);
        let _ = helper::show_open_with_dialog(path, hwnd);
    }
}

#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<helper::Extract>, String> {
    match mode {
        "zip" => helper::Extract::zip(path).map_err(|e| e.to_string()),
        _ => Err("Not Support".to_string()),
    }
}

#[command]
pub fn document(path: &str, mode: &str) -> Result<Vec<helper::DSheet>, String> {
    match mode {
        "csv" => helper::Document::csv(path).map_err(|e| e.to_string()),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            helper::Document::excel(path).map_err(|e| e.to_string())
        }
        _ => Err("Not Support".to_string()),
    }
}

use windows::{
    core::s,
    Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{self},
    },
};
#[tauri::command]
pub fn set_into_taskbar(app: AppHandle, label: String) {
    let w = app.get_webview_window(&label).unwrap();
    let h = w.hwnd().unwrap();
    set_taskbar(h);
}

fn set_taskbar(h: HWND) {
    unsafe {
        let shell_tray_wnd = WindowsAndMessaging::FindWindowA(s!("Shell_TrayWnd"), None).unwrap();
        let tray =
            WindowsAndMessaging::FindWindowExA(shell_tray_wnd, None, s!("TrayNotifyWnd"), None)
                .unwrap();
        let rect = &mut RECT {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        } as *mut RECT;
        let _ = WindowsAndMessaging::GetWindowRect(tray, rect);
        let r = *rect;
        let _ = WindowsAndMessaging::SetParent(h, shell_tray_wnd);
        let _ = WindowsAndMessaging::MoveWindow(h, r.left - 100, 0, 100, 60, false);
    }
}