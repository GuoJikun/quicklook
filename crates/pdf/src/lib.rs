use base64::{engine::general_purpose::STANDARD, Engine};
use pdfium_render::prelude::*;
use quicklook_error::QuickLookError;

/// PDF 元信息
#[derive(Debug, serde::Serialize)]
pub struct PdfMeta {
    pub page_count: u32,
}

/// 获取 PDF 元信息（页数等）
pub fn get_meta(path: &str) -> Result<PdfMeta, QuickLookError> {
    let pdfium = Pdfium::default();
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| QuickLookError::DocumentParse(e.to_string()))?;
    let page_count = doc.pages().len() as u32;
    Ok(PdfMeta { page_count })
}

/// 渲染指定页为 PNG base64 字符串
/// page_index: 0-based
/// scale: 渲染缩放倍率，1.0 ≈ 96dpi，2.0 ≈ 192dpi（Retina）
pub fn render_page_base64(
    path: &str,
    page_index: u32,
    scale: f32,
) -> Result<String, QuickLookError> {
    let pdfium = Pdfium::default();
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| QuickLookError::DocumentParse(e.to_string()))?;

    let page = doc
        .pages()
        .get(page_index as u16)
        .map_err(|e| QuickLookError::DocumentParse(e.to_string()))?;

    let width = (page.width().value * scale) as u16;
    let height = (page.height().value * scale) as u16;

    let config = PdfRenderConfig::new()
        .set_target_width(width.into())
        .set_maximum_height(height.into());

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| QuickLookError::DocumentParse(e.to_string()))?;

    // 转为 PNG bytes
    let img = bitmap.as_image();
    let mut png_bytes: Vec<u8> = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| QuickLookError::DocumentParse(e.to_string()))?;

    Ok(STANDARD.encode(&png_bytes))
}
