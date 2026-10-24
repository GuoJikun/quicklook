use std::path::{Path, PathBuf};

use quicklook_error::QuickLookError;

/// 返回缓存根目录。
///
/// 默认使用系统临时目录；设置 `QUICKLOOK_CACHE_ROOT` 可覆盖（主要用于测试）。
pub fn root() -> PathBuf {
    std::env::var_os("QUICKLOOK_CACHE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// 计算文件缓存 key：路径 + 大小 + 修改时间。
///
/// 文件内容或元数据变化后会生成新的 key，避免命中过期缓存。
pub fn file_key(path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// 一个命名缓存目录，封装创建、路径拼接与清理逻辑。
#[derive(Debug, Clone)]
pub struct CacheDir {
    path: PathBuf,
}

impl CacheDir {
    /// 在默认缓存根目录下创建指定名称的缓存目录描述。
    pub fn new(name: &str) -> Self {
        Self::with_root(root(), name)
    }

    /// 在指定根目录下创建缓存目录描述（测试可注入独立根目录）。
    pub fn with_root(root: impl AsRef<Path>, name: &str) -> Self {
        Self { path: root.as_ref().join(name) }
    }

    /// 缓存目录路径（不保证已存在）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 拼接缓存文件路径。
    pub fn file_path(&self, file_name: impl AsRef<str>) -> PathBuf {
        self.path.join(file_name.as_ref())
    }

    /// 确保缓存目录存在。
    pub fn ensure(&self) -> Result<(), QuickLookError> {
        std::fs::create_dir_all(&self.path)?;
        Ok(())
    }

    /// 清空缓存目录下的所有条目（保留目录本身），返回删除的条目数。
    pub fn clear(&self) -> Result<u32, QuickLookError> {
        if !self.path.exists() {
            return Ok(0);
        }

        let mut removed = 0u32;
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            match result {
                Ok(()) => removed += 1,
                Err(e) => log::warn!("清理缓存失败: {}, 错误: {}", path.display(), e),
            }
        }
        Ok(removed)
    }

    /// 只删除名称以 `prefix` 开头的子目录，返回删除的目录数。
    pub fn clear_dirs_with_prefix(&self, prefix: &str) -> Result<u32, QuickLookError> {
        if !self.path.exists() {
            return Ok(0);
        }

        let mut removed = 0u32;
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && entry.file_name().to_string_lossy().starts_with(prefix) {
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => removed += 1,
                    Err(e) => log::warn!("清理缓存目录失败: {}, 错误: {}", path.display(), e),
                }
            }
        }
        Ok(removed)
    }
}
