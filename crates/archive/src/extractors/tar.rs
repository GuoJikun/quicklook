use crate::{Extract, ArchiveError};
use std::{
    fs::File,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

/// 列举 TAR 文件条目
pub fn list_tar_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let mut archive = tar::Archive::new(file);
    let mut entries = Vec::new();

    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;
        let name = path.to_string_lossy().to_string();
        let size = header.size()?;
        let mtime = header.mtime()?;
        let is_dir = header.entry_type().is_dir();

        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = format!(
            "{}",
            chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339()
        );

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 列举 TAR.GZ 文件条目
pub fn list_tar_gz_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    let mut entries = Vec::new();

    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;
        let name = path.to_string_lossy().to_string();
        let size = header.size()?;
        let mtime = header.mtime()?;
        let is_dir = header.entry_type().is_dir();

        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = format!(
            "{}",
            chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339()
        );

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 列举 TAR.BZ2 文件条目
pub fn list_tar_bz2_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let bz2 = bzip2::read::BzDecoder::new(file);
    let mut archive = tar::Archive::new(bz2);
    let mut entries = Vec::new();

    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;
        let name = path.to_string_lossy().to_string();
        let size = header.size()?;
        let mtime = header.mtime()?;
        let is_dir = header.entry_type().is_dir();

        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = format!(
            "{}",
            chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339()
        );

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 列举 TAR.XZ 文件条目
pub fn list_tar_xz_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let xz = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(xz);
    let mut entries = Vec::new();

    for entry in archive.entries()? {
        let entry = entry?;
        let header = entry.header();
        let path = entry.path()?;
        let name = path.to_string_lossy().to_string();
        let size = header.size()?;
        let mtime = header.mtime()?;
        let is_dir = header.entry_type().is_dir();

        let dt = UNIX_EPOCH + Duration::from_secs(mtime);
        let last_modified = format!(
            "{}",
            chrono::DateTime::<chrono::Local>::from(dt).to_rfc3339()
        );

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}
