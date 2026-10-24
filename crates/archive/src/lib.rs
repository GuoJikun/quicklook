use quicklook_error::{QuickLookError, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

pub mod extractors;

/// 统一条目路径：Windows 分隔符 `\` 归一为 `/`，去掉重复斜杠
pub fn normalize_entry_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_slash = false;
    for ch in name.chars() {
        if ch == '\\' || ch == '/' {
            if !prev_slash {
                out.push('/');
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

pub use extractors::ar::list_ar_entries;
pub use extractors::cpio::list_cpio_entries;
pub use extractors::rar::{is_rar_password_protected, list_rar_entries};
pub use extractors::sevenz::{is_7z_password_protected, list_7z_entries};
pub use extractors::tar::{
    list_tar_bz2_entries, list_tar_entries, list_tar_gz_entries, list_tar_xz_entries,
};
pub use extractors::zip::{is_zip_password_protected, list_zip_entries, zip_extract};
pub use extractors::zst::list_tar_zst_entries;

/// 压缩文件条目信息
#[derive(Debug, Clone, Serialize)]
pub struct Extract {
    /// 文件名（树节点为归一化完整路径）
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 最后修改时间
    pub last_modified: String,
    /// 是否是目录
    pub dir: bool,
    /// 子目录/文件（用于构建树状结构）
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// 检测归档文件是否需要密码
    pub fn is_password_protected<P: AsRef<Path>>(archive_path: P) -> Result<bool> {
        let path = archive_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => extractors::zip::is_zip_password_protected(path),
            "7z" => extractors::sevenz::is_7z_password_protected(path),
            "rar" => extractors::rar::is_rar_password_protected(path),
            "jar" | "war" | "ear" | "apk" | "aar" | "whl" | "vsix" | "nupkg" | "crx" | "xpi"
            | "egg" | "kra" | "xps" | "oxps" => extractors::zip::is_zip_password_protected(path),
            // TAR/CPIO/AR 等格式不支持加密
            _ => Ok(false),
        }
    }

    /// 列举归档文件（不解压内容），并构建树结构
    pub fn list_archive_tree<P: AsRef<Path>>(
        archive_path: P,
        password: Option<&str>,
    ) -> Result<Vec<Extract>> {
        let path = archive_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let entries = match extension.as_str() {
            "zip" => extractors::zip::list_zip_entries(path, password)?,
            "tar" => extractors::tar::list_tar_entries(path)?,
            "gz" | "tgz" => extractors::tar::list_tar_gz_entries(path)?,
            "bz2" | "tbz2" => extractors::tar::list_tar_bz2_entries(path)?,
            "xz" | "txz" => extractors::tar::list_tar_xz_entries(path)?,
            "zst" | "tzst" => extractors::zst::list_tar_zst_entries(path)?,
            "7z" => extractors::sevenz::list_7z_entries(path, password)?,
            "rar" => extractors::rar::list_rar_entries(path, password)?,
            "cpio" => extractors::cpio::list_cpio_entries(path)?,
            "ar" | "deb" | "a" => extractors::ar::list_ar_entries(path)?,
            // ZIP 本质但带特殊扩展名的格式
            "jar" | "war" | "ear" | "apk" | "aar" | "whl" | "vsix" | "nupkg" | "crx" | "xpi"
            | "egg" | "kra" | "xps" | "oxps" => extractors::zip::list_zip_entries(path, password)?,
            // 对于其他格式，返回错误
            _ => return Err(QuickLookError::UnsupportedArchiveFormat(extension)),
        };

        let tree = Self::build_tree(entries);
        Ok(tree)
    }

    /// 将扁平的条目列表构建为嵌套的目录树。
    ///
    /// - 自动合成缺失的中间目录（与前端预览语义一致，避免丢条目）
    /// - 若归档内存在目录条目，保留其 `size` / `last_modified`
    /// - 节点 `name` 为归一化后的完整路径（不含目录尾部 `/`）
    pub fn build_tree(entries: Vec<Extract>) -> Vec<Extract> {
        let mut nodes: Vec<Extract> = Vec::new();
        let mut children: Vec<Vec<usize>> = Vec::new();
        let mut path_to_idx: HashMap<String, usize> = HashMap::new();
        let mut roots: Vec<usize> = Vec::new();

        for mut entry in entries {
            entry.name = normalize_entry_name(&entry.name);
            let trimmed = entry.name.trim_end_matches('/').trim_matches('/');
            if trimmed.is_empty() {
                continue;
            }

            let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
            if segments.is_empty() {
                continue;
            }

            let mut parent: Option<usize> = None;
            let mut path_so_far = String::new();

            for (i, seg) in segments.iter().enumerate() {
                if !path_so_far.is_empty() {
                    path_so_far.push('/');
                }
                path_so_far.push_str(seg);
                let is_last = i + 1 == segments.len();
                let path_key = path_so_far.clone();

                if is_last && !entry.dir {
                    // 文件节点：直接挂到父目录
                    let mut node = entry.clone();
                    node.name = path_key.clone();
                    node.children = None;
                    let idx = nodes.len();
                    nodes.push(node);
                    children.push(Vec::new());
                    path_to_idx.insert(path_key, idx);
                    match parent {
                        Some(p) => children[p].push(idx),
                        None => roots.push(idx),
                    }
                } else {
                    // 目录节点（中间目录或显式目录条目）
                    let idx = if let Some(&idx) = path_to_idx.get(&path_key) {
                        idx
                    } else {
                        let node = Extract::new(path_key.clone(), 0, String::new(), true);
                        let idx = nodes.len();
                        nodes.push(node);
                        children.push(Vec::new());
                        path_to_idx.insert(path_key.clone(), idx);
                        match parent {
                            Some(p) => children[p].push(idx),
                            None => roots.push(idx),
                        }
                        idx
                    };

                    // 显式目录条目：合并元数据
                    if is_last && entry.dir {
                        nodes[idx].size = entry.size;
                        if !entry.last_modified.is_empty() {
                            nodes[idx].last_modified = entry.last_modified.clone();
                        }
                        nodes[idx].dir = true;
                    }
                    parent = Some(idx);
                }
            }
        }

        // 组装为嵌套结构，子节点按名称排序保证展示稳定
        fn assemble(
            nodes: &mut Vec<Extract>,
            children: &mut Vec<Vec<usize>>,
            idx: usize,
        ) -> Extract {
            let mut node = Extract::new(
                std::mem::take(&mut nodes[idx].name),
                nodes[idx].size,
                std::mem::take(&mut nodes[idx].last_modified),
                nodes[idx].dir,
            );
            let kids = std::mem::take(&mut children[idx]);
            if kids.is_empty() {
                return node;
            }
            let mut child_items: Vec<Extract> = kids
                .into_iter()
                .map(|c| assemble(nodes, children, c))
                .collect();
            child_items.sort_by(|a, b| a.name.cmp(&b.name));
            node.children = Some(child_items);
            node
        }

        roots.sort_by(|&a, &b| nodes[a].name.cmp(&nodes[b].name));
        roots
            .into_iter()
            .map(|idx| assemble(&mut nodes, &mut children, idx))
            .collect()
    }
}
