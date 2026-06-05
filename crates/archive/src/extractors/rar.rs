use crate::{ArchiveError, Extract};
use chrono::NaiveDate;
use std::path::Path;

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

/// 将 RAR 文件时间转换为 yyyy-MM-dd HH:mm:ss 格式
/// RAR header.file_time 是 DOS date/time 打包格式:
///   高 16 位: 日期 (bit 15-9: 年-1980, bit 8-5: 月, bit 4-0: 日)
///   低 16 位: 时间 (bit 15-11: 时, bit 10-5: 分, bit 4-0: 秒/2)
fn rar_time_to_string(file_time: u32) -> String {
    if file_time == 0 {
        return "1970-01-01 00:00:00".to_string();
    }

    let date = (file_time >> 16) as u16;
    let time = (file_time & 0xFFFF) as u16;

    let year = ((date >> 9) + 1980) as i32;
    let month = ((date >> 5) & 0x0F) as u32;
    let day = (date & 0x1F) as u32;
    let hour = (time >> 11) as u32;
    let minute = ((time >> 5) & 0x3F) as u32;
    let second = ((time & 0x1F) * 2) as u32;

    match NaiveDate::from_ymd_opt(year, month, day) {
        Some(date) => match date.and_hms_opt(hour, minute, second) {
            Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            None => "1970-01-01 00:00:00".to_string(),
        },
        None => "1970-01-01 00:00:00".to_string(),
    }
}
