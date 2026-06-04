use std::path::PathBuf;
use tauri::command;

use crate::error::QuickLookError;
use crate::helper::image as image_helper;

/// 将 PSD、HEIC/HEIF 等图片格式转换为 PNG 并缓存。
/// 返回转换后 PNG 文件的路径。
#[command]
pub fn convert_to_png(path: &str) -> Result<String, QuickLookError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    let hash = hasher.finish();

    let mut images_dir: PathBuf = std::env::temp_dir();
    images_dir.push("quicklook_images");
    std::fs::create_dir_all(&images_dir)?;

    let mut temp_path = images_dir;
    temp_path.push(format!("quicklook_{:x}.png", hash));

    if temp_path.exists() {
        log::info!("命中图片缓存: {:?}", temp_path);
        return Ok(temp_path.to_string_lossy().to_string());
    }

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "psd" => image_helper::psd_to_png(path, &temp_path)?,
        "heic" | "heif" => image_helper::heic_to_png(path, &temp_path)?,
        _ => image_helper::image_to_png(path, &temp_path)?,
    }

    Ok(temp_path.to_string_lossy().to_string())
}

/// 清理图片转码缓存目录。
/// 返回被删除的文件数量。
#[command]
pub fn clear_image_cache() -> Result<u32, QuickLookError> {
    let images_dir = std::env::temp_dir().join("quicklook_images");
    if !images_dir.exists() {
        log::info!("quicklook_images 目录不存在，无需清理");
        return Ok(0);
    }
    let entries = std::fs::read_dir(&images_dir)?;

    let mut removed = 0u32;
    for entry in entries.flatten() {
        if entry.path().is_file() {
            match std::fs::remove_file(entry.path()) {
                Ok(_) => {
                    removed += 1;
                    log::info!("已清理图片缓存: {}", entry.path().display());
                },
                Err(e) => {
                    log::warn!("清理图片缓存失败: {}, 错误: {}", entry.path().display(), e);
                },
            }
        }
    }
    log::info!("共清理 {} 个图片缓存文件", removed);
    Ok(removed)
}
