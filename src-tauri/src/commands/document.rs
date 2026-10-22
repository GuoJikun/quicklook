use quicklook_docs as docs;
use tauri::command;

use crate::error::QuickLookError;

#[command]
pub async fn document(path: String, mode: String) -> Result<docs::Docs, QuickLookError> {
    tokio::task::spawn_blocking(move || match mode.as_str() {
        "csv" => docs::Docs::csv(&path),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => docs::Docs::excel(&path),
        "docx" => docs::Docs::docx(&path),
        _ => Err(QuickLookError::UnsupportedDocumentFormat(mode)),
    })
    .await
    .map_err(|e| QuickLookError::DocumentParse(format!("文档解析任务执行失败: {}", e)))?
}
