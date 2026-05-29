use crate::{ArchiveError, Extract};
use std::{fs::File, io::BufReader, path::Path};

/// 列举 7Z 文件条目
pub fn list_7z_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let archive = sevenz_rust::Archive::read(&mut reader, len, &[])?;

    let entries = archive
        .files
        .iter()
        .map(|entry| {
            let name = entry.name.clone();
            let size = if entry.has_stream { entry.size } else { 0 };
            let is_dir = entry.is_directory;
            // TODO: expose last_modified once sevenz-rust provides timestamp access
            let last_modified = "1970-01-01T00:00:00Z".to_string();
            Extract::new(name, size, last_modified, is_dir)
        })
        .collect();

    Ok(entries)
}
