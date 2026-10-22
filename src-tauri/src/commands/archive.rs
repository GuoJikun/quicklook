use quicklook_archive::Extract;
use tauri::command;

use crate::error::QuickLookError;

#[command]
pub async fn archive_is_password_protected(path: String) -> Result<bool, QuickLookError> {
    tauri::async_runtime::spawn_blocking(move || {
        let needed = Extract::is_password_protected(&path)
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string()))?;
        log::info!("检测压缩文件路径: {}, 是否需要密码: {}", path, needed);
        Ok(needed)
    })
    .await
    .map_err(|e| QuickLookError::ArchiveParse(format!("密码检测任务执行失败: {}", e)))?
}

#[command]
pub async fn archive(
    path: String,
    password: Option<String>,
) -> Result<Vec<Extract>, QuickLookError> {
    tauri::async_runtime::spawn_blocking(move || {
        log::info!("开始处理压缩文件: {}", path);
        let result = Extract::list_archive_tree(&path, password.as_deref())
            .map_err(|e| QuickLookError::ArchiveParse(e.to_string()));

        match &result {
            Ok(entries) => {
                log::info!("成功处理压缩文件，共{}个根条目", entries.len());
            },
            Err(e) => {
                log::error!("压缩文件处理失败: {}", e);
            },
        }

        result
    })
    .await
    .map_err(|e| QuickLookError::ArchiveParse(format!("压缩文件处理任务执行失败: {}", e)))?
}
