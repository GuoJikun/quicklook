use quicklook_docs as docs;
use tauri::command;

use crate::error::QuickLookError;

#[command]
pub fn document(path: &str, mode: &str) -> Result<docs::Docs, QuickLookError> {
    match mode {
        "csv" => docs::Docs::csv(path).map_err(|e| QuickLookError::DocumentParse(e.to_string())),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => {
            docs::Docs::excel(path).map_err(|e| QuickLookError::DocumentParse(e.to_string()))
        },
        "docx" => docs::Docs::docx(path).map_err(|e| QuickLookError::DocumentParse(e.to_string())),
        _ => Err(QuickLookError::UnsupportedDocumentFormat(mode.to_string())),
    }
}
