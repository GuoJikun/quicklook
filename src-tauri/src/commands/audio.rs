use tauri::command;

use crate::error::QuickLookError;
use crate::helper::audio;

#[command]
pub fn read_audio_info(path: &str) -> Option<audio::MusicInfo> {
    audio::read_music_info(path)
}

#[command]
pub fn parse_lrc(path: &str) -> Result<audio::Lrc, QuickLookError> {
    audio::parse_lrc(path)
}
