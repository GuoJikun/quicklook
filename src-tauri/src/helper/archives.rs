use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};
use zip::{DateTime, ZipArchive};
// use libarchive::archive::{Archive, FileType};

#[allow(unused)]
#[derive(Debug, Clone, Serialize)]
pub struct Extract {
    name: String,
    size: u64,
    last_modified: String,
    dir: bool,
    children: Option<Vec<Extract>>,
}

#[allow(unused)]
impl Extract {
    /// 处理zip格式的压缩文件
    pub fn zip(zip_path: &str) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        // 打开压缩文件
        let file = File::open(zip_path)?;
        let mut archive: ZipArchive<File> = ZipArchive::new(file)?;
        let mut target: Vec<Extract> = vec![];
        // 遍历压缩包中的每一个文件
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let dir = file.is_dir();
            let file_name = file.name().to_string();
            let size = file.size();
            let last_modified = file
                .last_modified()
                .unwrap_or(DateTime::default())
                .to_string();

            target.push(Extract {
                name: file_name,
                size,
                last_modified,
                dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        // let archives = Extract::new(zip_path);
        // println!("{:?}", archives);

        Ok(target)
    }

    /// 处理7z格式的压缩文件
    pub fn seven_zip(path: &str) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        let entries = Self::list_7z_entries(path)?;
        let tree = Self::build_tree(entries);
        Ok(tree)
    }

    /// 将扁平的条目列表构建为嵌套的目录树
    fn build_tree(entries: Vec<Extract>) -> Vec<Extract> {
        use std::collections::HashMap;

        let mut tree = Vec::new();
        let mut path_map: HashMap<String, Extract> = HashMap::new();

        // 首先创建所有节点
        for entry in entries {
            let path = Path::new(&entry.name);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| entry.name.clone());

            let extract = Extract {
                name,
                size: entry.size,
                last_modified: entry.last_modified,
                dir: entry.dir,
                children: if entry.dir { Some(Vec::new()) } else { None },
            };

            path_map.insert(entry.name, extract);
        }

        // 构建树结构
        let mut paths: Vec<String> = path_map.keys().cloned().collect();
        paths.sort();

        for path_str in paths {
            let path = Path::new(&path_str);

            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if parent_str != "" && path_map.contains_key(&parent_str) {
                    // 有父目录，将当前节点移动到父目录的children中
                    if let Some(current_node) = path_map.remove(&path_str) {
                        if let Some(parent_node) = path_map.get_mut(&parent_str) {
                            if let Some(ref mut children) = parent_node.children {
                                children.push(current_node);
                            }
                        }
                    }
                    continue;
                }
            }

            // 没有父目录或父目录不存在，作为根节点
            if let Some(node) = path_map.remove(&path_str) {
                tree.push(node);
            }
        }

        tree
    }

    /// 列举归档文件（不解压内容），并构建树结构
    pub fn list_archive_tree<P: AsRef<Path>>(
        archive_path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        let path = archive_path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let entries = match extension.as_str() {
            "zip" => Self::list_zip_entries(path)?,
            "tar" => Self::list_tar_entries(path)?,
            "gz" | "tgz" => Self::list_tar_gz_entries(path)?,
            "bz2" | "tbz2" => Self::list_tar_bz2_entries(path)?,
            "xz" | "txz" => Self::list_tar_xz_entries(path)?,
            "7z" => Self::list_7z_entries(path)?,
            // 对于其他格式，尝试使用 libarchive
            _ => Self::list_libarchive_entries(path)?,
        };

        let tree = Self::build_tree(entries);
        Ok(tree)
    }

    /// 列举 ZIP 文件条目
    fn list_zip_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut entries = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let is_dir = file.is_dir();
            let name = file.name().to_string();
            let size = file.size();
            let last_modified = file
                .last_modified()
                .unwrap_or(DateTime::default())
                .to_string();

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        Ok(entries)
    }

    /// 列举 TAR 文件条目
    pub fn list_tar_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
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

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        Ok(entries)
    }

    /// 列举 TAR.GZ 文件条目
    pub fn list_tar_gz_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
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

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        Ok(entries)
    }

    /// 列举 TAR.BZ2 文件条目
    pub fn list_tar_bz2_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
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

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        Ok(entries)
    }

    /// 列举 TAR.XZ 文件条目
    pub fn list_tar_xz_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
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

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None, // ZIP 文件条目通常不包含子目录
            });
        }

        Ok(entries)
    }

    /// 列举 7Z 文件条目
    pub fn list_7z_entries<P: AsRef<Path>>(
        path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        use std::io::BufReader;

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

            entries.push(Extract {
                name,
                size,
                last_modified,
                dir: is_dir,
                children: None,
            });
        }

        Ok(entries)
    }

    /// 使用 libarchive 列举其他格式的压缩文件条目
    fn list_libarchive_entries<P: AsRef<Path>>(
        _path: P,
    ) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
        // TODO: 实现 libarchive 支持
        Err("libarchive support not yet implemented".into())
    }
}
