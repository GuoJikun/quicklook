use log::{set_max_level, LevelFilter};
use quicklook_archive::{extractors, Extract};
use quicklook_docs as docs;
use std::path::PathBuf;
use tauri::{command, AppHandle};
#[cfg(windows)]
use tauri::Manager;

#[cfg(windows)]
use windows::Win32::Foundation::HWND;

#[path = "helper/mod.rs"]
mod helper;
#[cfg(windows)]
use helper::{audio, monitor, win};
#[cfg(not(windows))]
use helper::{audio, monitor};

#[command]
#[cfg(windows)]
pub fn show_open_with_dialog(app: AppHandle, path: &str) {
    if let Some(preview_window) = app.get_webview_window("preview") {
        let hwnd = preview_window.hwnd().map_or(HWND::default(), |hwnd| hwnd);
        let _ = win::show_open_with_dialog(path, hwnd);
    }
}

#[command]
#[cfg(not(windows))]
pub fn show_open_with_dialog(_app: AppHandle, path: &str) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<Extract>, String> {
    log::info!("开始处理压缩文件: {}, 扩展名: {}", path, mode);
    let result = match mode {
        "zip" => extractors::zip::zip_extract(path).map_err(|e| e.to_string()),
        "tar" => extractors::tar::list_tar_entries(path).map_err(|e| e.to_string()),
        "gz" | "tgz" => extractors::tar::list_tar_gz_entries(path).map_err(|e| e.to_string()),
        "bz2" | "tbz2" => extractors::tar::list_tar_bz2_entries(path).map_err(|e| e.to_string()),
        "xz" | "txz" => extractors::tar::list_tar_xz_entries(path).map_err(|e| e.to_string()),
        "7z" => extractors::sevenz::list_7z_entries(path).map_err(|e| e.to_string()),
        _ => Err("不支持的压缩格式".to_string()),
    };

    match &result {
        Ok(entries) => {
            log::info!("成功处理压缩文件，共{}个条目", entries.len());
        },
        Err(e) => {
            log::error!("压缩文件处理失败: {}", e);
        },
    }

    result
}

#[command]
pub fn document(path: &str, mode: &str) -> Result<docs::Docs, String> {
    match mode {
        "csv" => docs::Docs::csv(path).map_err(|e| e.to_string()),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            docs::Docs::excel(path).map_err(|e| e.to_string())
        },
        "docx" => docs::Docs::docx(path).map_err(|e| e.to_string()),
        _ => Err("Not Support".to_string()),
    }
}

#[command]
pub fn get_monitor_info() -> monitor::MonitorInfo {
    monitor::get_monitor_info()
}

#[command]
#[cfg(windows)]
pub fn get_default_program_name(path: &str) -> Result<String, String> {
    win::get_default_program_name(path)
}

#[command]
#[cfg(not(windows))]
pub fn get_default_program_name(path: &str) -> Result<String, String> {
    // 先通过 xdg-mime 查询文件的 MIME 类型，再查询对应的默认程序
    let mime_output = std::process::Command::new("xdg-mime")
        .args(["query", "filetype", path])
        .output()
        .map_err(|e| format!("xdg-mime 执行失败: {}", e))?;

    if !mime_output.status.success() {
        return Err("xdg-mime query filetype 失败".to_string());
    }

    let mime_type = String::from_utf8_lossy(&mime_output.stdout).trim().to_string();
    if mime_type.is_empty() {
        return Err("未能确定文件 MIME 类型".to_string());
    }

    let app_output = std::process::Command::new("xdg-mime")
        .args(["query", "default", &mime_type])
        .output()
        .map_err(|e| format!("xdg-mime query default 执行失败: {}", e))?;

    let app_name = String::from_utf8_lossy(&app_output.stdout).trim().to_string();
    if app_name.is_empty() {
        return Err(format!("未找到 {} 的默认程序", mime_type));
    }

    // 去掉 .desktop 后缀并返回
    Ok(app_name.trim_end_matches(".desktop").to_string())
}

#[command]
pub fn set_log_level(level: usize) -> Result<(), String> {
    let level_filter = match level {
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };
    set_max_level(level_filter);
    Ok(())
}

#[command]
pub fn psd_to_png(path: &str) -> Result<String, String> {
    let file_bytes = std::fs::read(path);

    if file_bytes.is_err() {
        log::info!("psd:: 读取文件失败")
    }

    let psd_obj = psd::Psd::from_bytes(&*file_bytes.unwrap());
    if psd_obj.is_err() {
        log::info!("psd:: 从 bytes 解析 错误")
    }
    let psd_obj = psd_obj.unwrap();

    let rgba = psd_obj.rgba();
    // 封装成 RgbaImage
    let width = psd_obj.width();
    let height = psd_obj.height();
    let img = image::RgbaImage::from_raw(width, height, rgba);

    // 临时目录（跨平台）
    let mut temp_path: PathBuf = std::env::temp_dir();
    temp_path.push("quicklook_psd_preview.png"); // 固定文件名

    // 保存为 PNG
    img.unwrap()
        .save_with_format(&temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    // 返回文件路径给前端
    Ok(temp_path.to_string_lossy().to_string())
}

#[command]
pub fn read_audio_info(path: &str) -> Option<audio::MusicInfo> {
    audio::read_music_info(path)
}

#[command]
pub fn parse_lrc(path: &str) -> Result<audio::Lrc, String> {
    audio::parse_lrc(path)
}
