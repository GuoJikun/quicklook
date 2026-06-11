use tauri::command;

use crate::error::QuickLookError;
pub use quicklook_model::ModelInfo;

/// 读取 3D 模型文件并返回基础统计信息。
#[command]
pub fn load_model(path: &str, extension: &str) -> Result<ModelInfo, QuickLookError> {
    quicklook_model::load_model(path, extension)
}
