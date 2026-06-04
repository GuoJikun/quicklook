use std::{fs::File, io::BufReader};

use serde_json::Value;
use tauri::{path::BaseDirectory, AppHandle, Manager};

use crate::error::QuickLookError;

// 读取resources下的config.json文件
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub markdown: Vec<String>,
    pub markdown_checked: Vec<String>,
    pub image: Vec<String>,
    pub image_checked: Vec<String>,
    pub video: Vec<String>,
    pub video_checked: Vec<String>,
    pub doc: Vec<String>,
    pub doc_checked: Vec<String>,
    pub code: Vec<String>,
    pub code_checked: Vec<String>,
    pub font: Vec<String>,
    pub font_checked: Vec<String>,
    pub archive: Vec<String>,
    pub archive_checked: Vec<String>,
    pub book: Vec<String>,
    pub book_checked: Vec<String>,
}

fn extract_str_vec(
    config: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, QuickLookError> {
    config
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .ok_or_else(|| QuickLookError::ConfigField(format!("缺少或类型错误的键 \"{}\"", key)))
}

#[allow(unused)]
pub fn read_config(app: &AppHandle) -> Result<Config, QuickLookError> {
    let config_path = app
        .path()
        .resolve("config.json", BaseDirectory::Resource)
        .map_err(|e| QuickLookError::ConfigRead(e.to_string()))?;

    let file = File::open(config_path).map_err(|e| QuickLookError::Io(e.to_string()))?;
    let reader = BufReader::new(file);
    let config: Value = serde_json::from_reader(reader)
        .map_err(|e| QuickLookError::ConfigRead(e.to_string()))?;
    let config = config
        .as_object()
        .ok_or_else(|| QuickLookError::ConfigRead("config.json is not an object".to_string()))?;
    Ok(Config {
        markdown: extract_str_vec(config, "preview.markdown")?,
        markdown_checked: extract_str_vec(config, "preview.markdown.checked")?,
        image: extract_str_vec(config, "preview.image")?,
        image_checked: extract_str_vec(config, "preview.image.checked")?,
        video: extract_str_vec(config, "preview.video")?,
        video_checked: extract_str_vec(config, "preview.video.checked")?,
        doc: extract_str_vec(config, "preview.doc")?,
        doc_checked: extract_str_vec(config, "preview.doc.checked")?,
        code: extract_str_vec(config, "preview.code")?,
        code_checked: extract_str_vec(config, "preview.code.checked")?,
        font: extract_str_vec(config, "preview.font")?,
        font_checked: extract_str_vec(config, "preview.font.checked")?,
        archive: extract_str_vec(config, "preview.archive")?,
        archive_checked: extract_str_vec(config, "preview.archive.checked")?,
        book: extract_str_vec(config, "preview.book")?,
        book_checked: extract_str_vec(config, "preview.book.checked")?,
    })
}
