use tauri::command;

use crate::error::QuickLookError;
use crate::helper::image as image_helper;

/// 将 PSD、HEIC/HEIF、JXL 等图片转换为 PNG 并缓存，返回转换后 PNG 文件的路径。
#[command]
pub async fn convert_to_png(path: String) -> Result<String, QuickLookError> {
    tokio::task::spawn_blocking(move || image_helper::convert_to_png(&path))
        .await
        .map_err(|e| QuickLookError::ImageProcessing(format!("图片转换任务执行失败: {}", e)))?
}

/// 清理图片转码缓存，返回删除的缓存条目数。
#[command]
pub async fn clear_image_cache() -> Result<u32, QuickLookError> {
    tokio::task::spawn_blocking(image_helper::clear_image_cache)
        .await
        .map_err(|e| QuickLookError::ImageProcessing(format!("图片缓存清理任务执行失败: {}", e)))?
}
