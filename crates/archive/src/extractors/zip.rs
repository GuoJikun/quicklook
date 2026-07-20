use crate::{ArchiveError, Extract};
use std::{fs::File, path::Path};
use zip::ZipArchive;

fn decode_entry_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_owned(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(raw);
            decoded.into_owned()
        },
    }
}

/// 列举 ZIP 文件条目
pub fn list_zip_entries<P: AsRef<Path>>(
    path: P,
    password: Option<&str>,
) -> Result<Vec<Extract>, ArchiveError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::with_capacity(archive.len());

    for i in 0..archive.len() {
        let file = match password {
            Some(pw) => archive.by_index_decrypt(i, pw.as_bytes())?,
            None => archive.by_index_raw(i)?,
        };
        let is_dir = file.is_dir();
        let name = decode_entry_name(file.name_raw());
        let size = file.size();
        let last_modified = file.last_modified().unwrap_or_default().to_string();

        entries.push(Extract::new(name, size, last_modified, is_dir));
    }

    Ok(entries)
}

/// 处理 zip 格式的压缩文件（兼容旧接口）
pub fn zip_extract(zip_path: &str, password: Option<&str>) -> Result<Vec<Extract>, ArchiveError> {
    list_zip_entries(zip_path, password)
}

/// 检测 ZIP 文件是否需要密码
/// ZIP 的元数据（文件名、大小、时间）始终是明文的，不需要密码即可列出
/// 此方法始终返回 false
pub fn is_zip_password_protected<P: AsRef<Path>>(_path: P) -> Result<bool, ArchiveError> {
    Ok(false)
}
