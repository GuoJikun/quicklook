use crate::{ArchiveError, Extract};
use ruzstd::decoding::StreamingDecoder;
use std::{
    fs::File,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

fn parse_tar_zst_entries<R: std::io::Read>(reader: R) -> Result<Vec<Extract>, ArchiveError> {
    let decoder = StreamingDecoder::new(reader)
        .map_err(|e| ArchiveError::Other(format!("Zstd 解码失败: {e}")))?;
    let mut archive = tar::Archive::new(decoder);
    let mut entries = Vec::new();

    for entry_result in archive.entries()? {
        let entry = entry_result?;
        let header = entry.header();
        let name = entry.path()?.to_string_lossy().into_owned();
        let size = header.size()?;
        let mtime = header.mtime()?;
        let is_dir = header.entry_type().is_dir();

        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339();

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 列举 TAR.ZST / TZST 文件条目
pub fn list_tar_zst_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    parse_tar_zst_entries(File::open(path)?)
}
