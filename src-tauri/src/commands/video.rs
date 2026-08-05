use tauri::{command, AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::error::QuickLookError;
use crate::helper::ffmp;

/// 从 store 缓存读取 ffmpeg 检测结果：1=可用，0=不可用，-1=未检测（每次启动时重置）。
/// 缓存不存在时实际检测一次并写回 store。
fn check_ffmpeg_with_cache(app: &AppHandle) -> bool {
    let store = match app.store("config.data") {
        Ok(s) => s,
        Err(e) => {
            log::warn!("读取 config.data 失败，直接检测 ffmpeg: {:?}", e);
            return ffmp::check_ffmpeg();
        }
    };

    match store.get("ffmpeg").and_then(|v| v.as_i64()) {
        Some(1) => {
            log::info!("ffmpeg 检测命中缓存: 可用");
            return true;
        }
        Some(0) => {
            log::info!("ffmpeg 检测命中缓存: 不可用");
            return false;
        }
        _ => {}
    }

    let available = ffmp::check_ffmpeg();
    log::info!("ffmpeg 实际检测结果: 可用={}", available);
    store.set("ffmpeg", serde_json::Value::from(if available { 1 } else { 0 }));
    available
}

#[command]
pub fn check_ffmpeg(app: AppHandle) -> bool {
    check_ffmpeg_with_cache(&app)
}

#[command]
pub async fn prepare_video_for_preview(path: String) -> Result<ffmp::VideoPreviewDecision, QuickLookError> {
    tauri::async_runtime::spawn_blocking(move || ffmp::prepare_video_for_preview(&path))
        .await
        .map_err(|e| QuickLookError::VideoConversion(format!("视频预检查执行失败: {}", e)))?
}

#[command]
pub async fn convert_video_to_hls(path: String) -> Result<String, QuickLookError> {
    tauri::async_runtime::spawn_blocking(move || ffmp::convert_video_to_hls(&path))
        .await
        .map_err(|e| QuickLookError::VideoConversion(format!("转码任务执行失败: {}", e)))?
}

#[command]
pub fn cancel_video_conversion() {
    ffmp::cancel_video_conversion()
}
