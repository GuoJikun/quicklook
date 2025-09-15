use crate::{ArchiveError, Extract};
use std::{fs::File, path::Path};
use zip::{DateTime, ZipArchive};

/// 列举 ZIP 文件条目
pub fn list_zip_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let is_dir = file.is_dir();
        let name = file.name().to_string();
        let size = file.size();
        let last_modified = file
            .last_modified()
            .unwrap_or(DateTime::default())
            .to_string();

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 处理zip格式的压缩文件（兼容旧接口）
pub fn zip_extract(zip_path: &str) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut target = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let dir = file.is_dir();
        let file_name = file.name().to_string();
        let size = file.size();
        let last_modified = file
            .last_modified()
            .unwrap_or(DateTime::default())
            .to_string();

        target.push(Extract::new(file_name, size, last_modified, dir));
    }

    Ok(target)
}
