use tauri::command;

use crate::error::QuickLookError;
use crate::helper::audio;

#[command]
pub async fn read_audio_info(path: String) -> Option<audio::MusicInfo> {
    tokio::task::spawn_blocking(move || audio::read_music_info(&path))
        .await
        .ok()
        .flatten()
}

#[command]
pub async fn parse_lrc(path: String) -> Result<audio::Lrc, QuickLookError> {
    tokio::task::spawn_blocking(move || audio::parse_lrc(&path))
        .await
        .map_err(|e| QuickLookError::LrcParse(format!("LRC 解析任务执行失败: {}", e)))?
}
