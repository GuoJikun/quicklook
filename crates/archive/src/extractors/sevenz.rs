use crate::{ArchiveError, Extract};
use sevenz_rust::Password;
use std::{fs::File, io::BufReader, path::Path};

/// Windows FILETIME: 100-nanosecond intervals since 1601-01-01 00:00:00 UTC
const FILETIME_UNIX_EPOCH_DIFF: u64 = 116_444_736_000_000_000;
const FILETIMES_PER_SEC: u64 = 10_000_000;

fn filetime_to_string(raw: u64) -> String {
    if raw == 0 || raw < FILETIME_UNIX_EPOCH_DIFF {
        return "1970-01-01 00:00:00".to_string();
    }

    let secs = (raw - FILETIME_UNIX_EPOCH_DIFF) / FILETIMES_PER_SEC;
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0);
    match dt {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => "1970-01-01 00:00:00".to_string(),
    }
}

/// 列举 7Z 文件条目
pub fn list_7z_entries<P: AsRef<Path>>(
    path: P,
    password: Option<&str>,
) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let password_bytes = password.map(|p| Password::from(p).as_slice().to_vec()).unwrap_or_default();
    let archive = match sevenz_rust::Archive::read(&mut reader, len, &password_bytes) {
        Ok(archive) => archive,
        Err(sevenz_rust::Error::PasswordRequired) | Err(sevenz_rust::Error::MaybeBadPassword(_)) => {
            return Err(ArchiveError::Other("Password required to decrypt 7z archive".into()));
        }
        Err(e) => return Err(ArchiveError::Other(format!("Failed to read 7z archive: {}", e))),
    };

    let entries = archive
        .files
        .iter()
        .map(|entry| {
            let name = entry.name.clone();
            let size = if entry.has_stream { entry.size } else { 0 };
            let is_dir = entry.is_directory;
            let last_modified = if entry.has_last_modified_date {
                filetime_to_string(entry.last_modified_date.to_raw())
            } else {
                "1970-01-01 00:00:00".to_string()
            };
            Extract::new(name, size, last_modified, is_dir)
        })
        .collect();

    Ok(entries)
}

/// 检测 7Z 文件是否需要密码
pub fn is_7z_password_protected<P: AsRef<Path>>(path: P) -> Result<bool, ArchiveError> {
    match sevenz_rust::Archive::open(path.as_ref()) {
        Ok(_) => Ok(false),
        Err(sevenz_rust::Error::PasswordRequired) | Err(sevenz_rust::Error::MaybeBadPassword(_)) => {
            Ok(true)
        }
        Err(e) => {
            log::error!("Failed to open 7z archive: {}", e);
            Err(ArchiveError::Other(format!(
                "Failed to open 7z archive: {}",
                e
            )))
        },
    }
}
