use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{command, ipc::Channel, AppHandle, Manager};
use windows::Win32::Foundation::HWND;
// use log::info;

#[path = "helper/mod.rs"]
mod helper;
use helper::{archives, docs, ffmpeg, monitor, win};

#[command]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().map_or(HWND::default(), |hwnd| hwnd);
        let _ = win::show_open_with_dialog(path, hwnd);
    }
}

#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<archives::Extract>, String> {
    log::info!("开始处理压缩文件: {}, 扩展名: {}", path, mode);
    let result = match mode {
        "zip" => archives::Extract::zip(path).map_err(|e| e.to_string()),
        "tar" => archives::Extract::list_tar_entries(path).map_err(|e| e.to_string()),
        "gz" | "tgz" => archives::Extract::list_tar_gz_entries(path).map_err(|e| e.to_string()),
        "bz2" | "tbz2" => archives::Extract::list_tar_bz2_entries(path).map_err(|e| e.to_string()),
        "xz" | "txz" => archives::Extract::list_tar_xz_entries(path).map_err(|e| e.to_string()),
        "7z" => archives::Extract::list_7z_entries(path).map_err(|e| e.to_string()),
        _ => Err("不支持的压缩格式".to_string()),
    };

    match &result {
        Ok(entries) => {
            log::info!("成功处理压缩文件，共{}个条目", entries.len());
        }
        Err(e) => {
            log::error!("压缩文件处理失败: {}", e);
        }
    }

    result
}

#[command]
pub fn document(path: &str, mode: &str) -> Result<docs::Docs, String> {
    match mode {
        "csv" => docs::Docs::csv(path).map_err(|e| e.to_string()),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            docs::Docs::excel(path).map_err(|e| e.to_string())
        }
        "docx" => docs::Docs::docx(path).map_err(|e| e.to_string()),
        _ => Err("Not Support".to_string()),
    }
}

#[command]
pub fn get_monitor_info() -> monitor::MonitorInfo {
    monitor::get_monitor_info()
}

#[command]
pub fn get_default_program_name(path: &str) -> Result<String, String> {
    win::get_default_program_name(path)
}

#[command]
pub async fn decode_video(path: String, on_chunk: Channel) -> Result<String, String> {
    // 生成唯一的任务 ID
    let task_id = format!(
        "video_stream_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    // 使用静态链接的视频流处理器开始转码
    ffmpeg::VideoStreamProcessor::start_stream(path, on_chunk, task_id.clone()).await?;

    // 返回任务 ID，供前端用于取消操作
    Ok(task_id)
}

/// Tauri 命令：取消视频流
#[command]
pub async fn cancel_task(task_id: String) -> Result<(), String> {
    ffmpeg::get_task_manager().cancel_task(&task_id)
}

/// Tauri 命令：获取正在运行的任务数量
#[command]
pub async fn get_active_tasks_count() -> Result<usize, String> {
    Ok(ffmpeg::get_task_manager().active_tasks_count())
}

// use windows::{
//     core::s,
//     Win32::{
//         Foundation::RECT,
//         UI::WindowsAndMessaging::{self},
//     },
// };
// #[tauri::command]
// pub fn set_into_taskbar(app: AppHandle, label: String) {
//     let w = app.get_webview_window(&label).unwrap();
//     let h = w.hwnd().unwrap();
//     set_taskbar(h);
// }

// fn set_taskbar(h: HWND) {
//     unsafe {
//         let shell_tray_wnd = WindowsAndMessaging::FindWindowA(s!("Shell_TrayWnd"), None).unwrap();
//         let tray =
//             WindowsAndMessaging::FindWindowExA(shell_tray_wnd, None, s!("TrayNotifyWnd"), None)
//                 .unwrap();
//         let rect = &mut RECT {
//             left: 0,
//             right: 0,
//             top: 0,
//             bottom: 0,
//         } as *mut RECT;
//         let _ = WindowsAndMessaging::GetWindowRect(tray, rect);
//         let r = *rect;
//         let _ = WindowsAndMessaging::SetParent(h, shell_tray_wnd);
//         let _ = WindowsAndMessaging::MoveWindow(h, r.left - 100, 0, 100, 60, false);
//     }
// }
