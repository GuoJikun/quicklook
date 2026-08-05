use quicklook_docs as docs;
use tauri::command;

use crate::error::QuickLookError;

#[command]
pub async fn document(path: String, mode: String) -> Result<docs::Docs, QuickLookError> {
    tokio::task::spawn_blocking(move || match mode.as_str() {
        "csv" => docs::Docs::csv(&path).map_err(|e| QuickLookError::DocumentParse(e.to_string())),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            docs::Docs::excel(&path).map_err(|e| QuickLookError::DocumentParse(e.to_string()))
        },
        "docx" => docs::Docs::docx(&path).map_err(|e| QuickLookError::DocumentParse(e.to_string())),
        _ => Err(QuickLookError::UnsupportedDocumentFormat(mode)),
    })
    .await
    .map_err(|e| QuickLookError::DocumentParse(format!("文档解析任务执行失败: {}", e)))?
}
