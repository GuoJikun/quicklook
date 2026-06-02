use crate::{ArchiveError, Extract};
use hadris_cpio::mode::FileType;
use hadris_cpio::sync::CpioReader;
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

/// 列举 CPIO 文件条目
pub fn list_cpio_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let mut reader = CpioReader::new(BufReader::new(file));
    let mut entries = Vec::new();

    loop {
        let entry = match reader.next_entry_alloc() {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(e) => return Err(ArchiveError::CpioError(e)),
        };

        let header = entry.header();
        let name = entry
            .name_str()
            .map(str::to_string)
            .unwrap_or_else(|_| "<invalid>".to_string());
        let size = entry.file_size() as u64;
        let mtime = header.mtime;
        let is_dir = matches!(entry.file_type(), FileType::Directory);

        let dt = UNIX_EPOCH + Duration::from_secs(mtime as u64);
        let last_modified = chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339();

        entries.push(Extract::new(name, size, last_modified, is_dir));

        reader.skip_entry_data_owned(&entry)?;
    }

    Ok(entries)
}
