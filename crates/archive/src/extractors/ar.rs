use crate::{ArchiveError, Extract};
use std::{
    fs::File,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

/// 列举 AR 文件条目（`.deb` 内层格式、`.a` 静态库等）
pub fn list_ar_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let mut archive = ar::Archive::new(file);
    let mut entries = Vec::new();

    while let Some(entry_result) = archive.next_entry() {
        let mut entry = entry_result?;
        let identifier = entry.header().identifier();
        let name = std::str::from_utf8(identifier)
            .map(str::to_string)
            .unwrap_or_else(|_| String::from_utf8_lossy(identifier).into_owned());
        let size = entry.header().size();
        let is_dir = name.ends_with('/');

        // AR 头部存储 mtime（UNIX 秒），转换为与其他格式一致的本地时区时间格式
        let mtime = entry.header().mtime();
        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = chrono::DateTime::<chrono::Local>::from(dt)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        entries.push(Extract::new(name, size, last_modified, is_dir));

        // 跳过当前条目的数据，使 reader 推进到下一个 header。
        use std::io::{copy, sink};
        let _ = copy(&mut entry, &mut sink());
    }

    Ok(entries)
}
