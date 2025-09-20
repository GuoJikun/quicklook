use lofty::prelude::{AudioFile, ItemKey, TaggedFileExt};
use lofty::read_from_path;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct MusicInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u64>,  // 秒
    pub bitrate: Option<u32>,   // kbps
    pub cover: Option<Vec<u8>>, // 图片二进制
}

pub fn read_music_info<P: AsRef<Path>>(path: P) -> Option<MusicInfo> {
    let tagged_file = read_from_path(&path).ok()?;

    // 标签
    let tag = TaggedFileExt::primary_tag(&tagged_file);

    // 音频属性
    let props = AudioFile::properties(&tagged_file);

    // 提取封面（APIC / covr 等）
    let cover = tag.and_then(|t| t.pictures().get(0).map(|pic| pic.data().to_vec()));
    let title = tag.and_then(|t| t.get_string(&ItemKey::TrackTitle).map(|s| s.to_string()));
    let artist = tag.and_then(|t| t.get_string(&ItemKey::TrackArtist).map(|s| s.to_string()));
    let album = tag.and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(|s| s.to_string()));
    let music_info = MusicInfo {
        title: title,
        artist: artist,
        album: album,
        duration: Some(props.duration().as_secs()),
        bitrate: props.audio_bitrate().map(|b| b / 1000), // 转换为 kbps
        cover,
    };
    log::info!("audio metadata: {:?}", music_info);
    Some(music_info)
}
