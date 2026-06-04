use tauri::command;

use crate::error::QuickLookError;
use crate::helper::ffmp;

#[command]
pub fn check_ffmpeg() -> bool {
    ffmp::check_ffmpeg()
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
