use lofty::prelude::{AudioFile, ItemKey, TaggedFileExt};
use lofty::read_from_path;
use std::path::Path;

#[allow(unused)]
#[derive(Debug, serde::Serialize)]
pub struct MusicInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u64>,  // 秒
    pub bitrate: Option<u32>,   // kbps
    pub cover: Option<Vec<u8>>, // 图片二进制
}
#[allow(unused)]
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

#[allow(unused)]
#[derive(Debug, serde::Serialize)]
pub struct LrcLine {
    timestamp: u64, // 毫秒
    text: String,
}
#[allow(unused)]
#[derive(Debug, serde::Serialize)]
pub struct Lrc {
    title: Option<String>,
    offset: Option<i64>,
    content: Vec<LrcLine>,
}
#[allow(unused)]
pub fn parse_lrc(path: &str) -> Result<Lrc, String> {
    let lrc_content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lrc = Lrc {
        title: None,
        offset: None,
        content: Vec::new(),
    };

    for line in lrc_content.lines() {
        // 匹配标题
        if line.trim().starts_with("[ti:") {
            let title = line
                .trim_start_matches("[ti:")
                .trim_end_matches(']')
                .trim()
                .to_string();
            lrc.title = Some(title);
            continue;
        }
        // 匹配偏移
        if line.trim().starts_with("[offset:") {
            if let Ok(offset) = line
                .trim_start_matches("[offset:")
                .trim_end_matches(']')
                .trim()
                .parse::<i64>()
            {
                lrc.offset = Some(offset);
            }
            continue;
        }

        let line_list = line.trim().split("]").collect::<Vec<&str>>();
        if line_list.len() != 2 {
            continue; // 忽略非歌词主体部分
        }

        let time_part = line_list[0].trim_start_matches('[');
        let lyric_text = line_list[1].trim().to_string();

        let mut lrc_line = LrcLine { timestamp: 0, text: lyric_text };
        if let Some((min_str, sec_str)) = time_part.split_once(':') {
            if let (Ok(min), Ok(sec)) = (min_str.parse::<u64>(), sec_str.parse::<f64>()) {
                let total_millis = min * 60_000 + (sec * 1000.0) as u64;
                lrc_line.timestamp = total_millis;
                lrc.content.push(lrc_line);
            }
        }
    }
    lrc.content.sort_by_key(|k| k.timestamp);
    Ok(lrc)
}
