use quicklook_pdf as pdf;
use tauri::command;

use crate::error::QuickLookError;

#[derive(serde::Serialize)]
pub struct PdfPageResult {
    pub base64: String,
    pub page: u32,
}

/// 获取 PDF 元信息（页数）
#[command]
pub fn pdf_meta(path: &str) -> Result<pdf::PdfMeta, QuickLookError> {
    pdf::get_meta(path)
}

/// 渲染 PDF 指定页为 base64 PNG
#[command]
pub fn pdf_render_page(
    path: &str,
    page: u32,
    scale: f32,
) -> Result<PdfPageResult, QuickLookError> {
    let base64 = pdf::render_page_base64(path, page, scale)?;
    Ok(PdfPageResult { base64, page })
}
