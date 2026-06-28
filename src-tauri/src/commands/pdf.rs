use tauri::command;

use crate::error::QuickLookError;
use quicklook_docs::pdf as pdf_helper;

/// 渲染 PDF 单页为 PNG。
#[command(async)]
pub async fn render_pdf_page(path: String, page_index: u32, dpi: u32) -> Result<pdf_helper::RenderedPage, QuickLookError> {
    log::info!("[cmd] render_pdf_page path={}, page_index={}, dpi={}", path, page_index, dpi);
    let result = tokio::task::spawn_blocking(move || pdf_helper::render_pdf_page(&path, page_index, dpi))
        .await
        .map_err(|e| QuickLookError::PdfRendering(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(page) => log::info!("[cmd] render_pdf_page 成功: page {}", page.page_num),
        Err(e) => log::error!("[cmd] render_pdf_page 失败: {}", e),
    }
    result
}

/// 获取 PDF 总页数。
#[command(async)]
pub async fn get_pdf_page_count(path: String) -> Result<u32, QuickLookError> {
    log::info!("[cmd] get_pdf_page_count path={}", path);
    let result = tokio::task::spawn_blocking(move || pdf_helper::get_pdf_page_count(&path))
        .await
        .map_err(|e| QuickLookError::PdfRendering(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(count) => log::info!("[cmd] get_pdf_page_count={}", count),
        Err(e) => log::error!("[cmd] get_pdf_page_count 失败: {}", e),
    }
    result
}

/// 获取 PDF 大纲（书签目录）。
#[command(async)]
pub async fn get_pdf_outline(path: String) -> Result<Vec<pdf_helper::OutlineItem>, QuickLookError> {
    log::info!("[cmd] get_pdf_outline path={}", path);
    let result = tokio::task::spawn_blocking(move || pdf_helper::get_pdf_outline(&path))
        .await
        .map_err(|e| QuickLookError::PdfRendering(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(items) => log::info!("[cmd] get_pdf_outline={} 项", items.len()),
        Err(e) => log::error!("[cmd] get_pdf_outline 失败: {}", e),
    }
    result
}

/// 清理 PDF 渲染缓存目录，返回删除的文件数。
#[command(async)]
pub async fn clear_pdf_cache() -> Result<u32, QuickLookError> {
    log::info!("[cmd] clear_pdf_cache");
    let result = tokio::task::spawn_blocking(|| pdf_helper::clear_pdf_cache())
        .await
        .map_err(|e| QuickLookError::PdfRendering(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(count) => log::info!("[cmd] clear_pdf_cache={}", count),
        Err(e) => log::error!("[cmd] clear_pdf_cache 失败: {}", e),
    }
    result
}
