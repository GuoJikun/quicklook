use crate::{ArchiveError, Extract};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// 列举 RAR 文件条目
pub fn list_rar_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy().to_string();

    let archive = unrar::Archive::new(&path_str)
        .open_for_listing()
        .map_err(|e| ArchiveError::Other(format!("Failed to open RAR archive: {}", e)))?;

    let mut entries = Vec::new();

    for entry in archive {
        match entry {
            Ok(header) => {
                let raw_name = header.filename.to_string_lossy().to_string();
                // 统一路径分隔符为 '/'
                let name = raw_name.replace('\\', "/");
                let size = header.unpacked_size;
                let is_dir = header.is_directory();

                // 目录名称需要以 '/' 结尾
                let name = if is_dir && !name.ends_with('/') {
                    format!("{}/", name)
                } else {
                    name
                };

                // 将 RAR 文件时间转换为 ISO 8601 格式
                let last_modified = rar_time_to_string(header.file_time);

                entries.push(Extract::new(name, size, last_modified, is_dir));
            }
            Err(e) => {
                log::warn!("Failed to read RAR entry: {}", e);
                continue;
            }
        }
    }

    Ok(entries)
}

/// 将 RAR 文件时间戳转换为 ISO 8601 字符串
/// RAR 时间格式: 自 1970-01-01 以来的秒数
fn rar_time_to_string(file_time: u32) -> String {
    let seconds = file_time as u64;
    let duration = std::time::Duration::from_secs(seconds);
    let datetime = UNIX_EPOCH + duration;

    // 转换为 chrono DateTime
    match datetime.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs() as i64;
            let naive = chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.naive_utc())
                .unwrap_or_default();
            naive.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }
        Err(_) => "1970-01-01T00:00:00Z".to_string(),
    }
}
