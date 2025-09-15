use crate::{ArchiveError, Extract};
use std::{fs::File, io::BufReader, path::Path};

/// 列举 7Z 文件条目
pub fn list_7z_entries<P: AsRef<Path>>(path: P) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let archive = sevenz_rust::Archive::read(&mut reader, len, &[])?;
    let mut entries = Vec::new();

    for entry in &archive.files {
        let name = entry.name.clone();
        let size = if entry.has_stream { entry.size } else { 0 };
        let is_dir = entry.is_directory;

        // 暂时使用默认时间，后续可以根据sevenz-rust库的更新进行改进
        let last_modified = "1970-01-01T00:00:00Z".to_string();

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 处理7z格式的压缩文件（兼容旧接口）
pub fn seven_zip_extract(path: &str) -> Result<Vec<Extract>, ArchiveError> {
    let entries = list_7z_entries(path)?;
    let tree = Extract::build_tree(entries);
    Ok(tree)
}
