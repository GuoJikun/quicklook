use serde::Serialize;
use std::path::Path;

pub mod error;
pub mod extractors;

pub use error::ArchiveError;
pub use extractors::*;

/// 压缩文件条目信息
#[derive(Debug, Clone, Serialize)]
pub struct Extract {
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 最后修改时间
    pub last_modified: String,
    /// 是否是目录
    pub dir: bool,
    /// 子目录/文件（用于构建树状结构）
    pub children: Option<Vec<Extract>>,
}

impl Extract {
    /// 创建新的条目
    pub fn new(name: String, size: u64, last_modified: String, dir: bool) -> Self {
        Self {
            name,
            size,
            last_modified,
            dir,
            children: None,
        }
    }

    /// 列举归档文件（不解压内容），并构建树结构
    pub fn list_archive_tree<P: AsRef<Path>>(
        archive_path: P,
    ) -> Result<Vec<Extract>, ArchiveError> {
        let path = archive_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let entries = match extension.as_str() {
            "zip" => extractors::zip::list_zip_entries(path)?,
            "tar" => extractors::tar::list_tar_entries(path)?,
            "gz" | "tgz" => extractors::tar::list_tar_gz_entries(path)?,
            "bz2" | "tbz2" => extractors::tar::list_tar_bz2_entries(path)?,
            "xz" | "txz" => extractors::tar::list_tar_xz_entries(path)?,
            "7z" => extractors::sevenz::list_7z_entries(path)?,
            // 对于其他格式，返回错误
            _ => return Err(ArchiveError::UnsupportedFormat(extension)),
        };

        let tree = Self::build_tree(entries);
        Ok(tree)
    }

    /// 将扁平的条目列表构建为嵌套的目录树
    pub fn build_tree(mut entries: Vec<Extract>) -> Vec<Extract> {
        // 按路径排序，确保父目录在子项之前
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut root_items = Vec::new();
        let mut dirs_map = std::collections::HashMap::new();

        for entry in entries {
            let path_parts: Vec<&str> = entry.name.trim_end_matches('/').split('/').collect();

            if path_parts.len() == 1 {
                // 根级别项目
                root_items.push(entry);
            } else {
                // 嵌套项目
                let parent_path = path_parts[..path_parts.len() - 1].join("/");
                dirs_map
                    .entry(parent_path)
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }

        // 递归构建目录树
        Self::build_tree_recursive(&mut root_items, &dirs_map);
        root_items
    }

    fn build_tree_recursive(
        items: &mut Vec<Extract>,
        dirs_map: &std::collections::HashMap<String, Vec<Extract>>,
    ) {
        for item in items.iter_mut() {
            if item.dir {
                if let Some(children) = dirs_map.get(&item.name.trim_end_matches('/').to_string()) {
                    let mut child_items = children.clone();
                    Self::build_tree_recursive(&mut child_items, dirs_map);
                    item.children = Some(child_items);
                }
            }
        }
    }
}

// 导出 C ABI 兼容的函数
#[no_mangle]
pub extern "C" fn archive_list_entries(
    path: *const std::os::raw::c_char,
    result: *mut *mut std::os::raw::c_char,
) -> i32 {
    use std::ffi::{CStr, CString};

    if path.is_null() || result.is_null() {
        return -1;
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match Extract::list_archive_tree(path_str) {
        Ok(entries) => match serde_json::to_string(&entries) {
            Ok(json) => match CString::new(json) {
                Ok(c_string) => {
                    unsafe {
                        *result = c_string.into_raw();
                    }
                    0
                }
                Err(_) => -1,
            },
            Err(_) => -1,
        },
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn archive_free_string(s: *mut std::os::raw::c_char) {
    if !s.is_null() {
        unsafe {
            drop(std::ffi::CString::from_raw(s));
        }
    }
}
