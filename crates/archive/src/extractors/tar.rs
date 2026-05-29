use crate::{ArchiveError, Extract};
use std::{
    fs::File,
    io::Read,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

fn parse_tar_entries<R: Read>(reader: R) -> Result<Vec<Extract>, ArchiveError> {
    let mut archive = tar::Archive::new(reader);
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

/// 列举 TAR 文件条目
pub fn list_tar_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    parse_tar_entries(File::open(path)?)
}

/// 列举 TAR.GZ 文件条目
pub fn list_tar_gz_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    parse_tar_entries(flate2::read::GzDecoder::new(File::open(path)?))
}

/// 列举 TAR.BZ2 文件条目
pub fn list_tar_bz2_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    parse_tar_entries(bzip2::read::BzDecoder::new(File::open(path)?))
}

/// 列举 TAR.XZ 文件条目
pub fn list_tar_xz_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    parse_tar_entries(xz2::read::XzDecoder::new(File::open(path)?))
}
