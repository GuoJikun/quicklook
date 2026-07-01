use tauri::command;

use crate::error::QuickLookError;
use quicklook_book::epub as epub_helper;
use quicklook_book::mobi as mobi_helper;

// ── EPUB 命令 ──────────────────────────────────────

/// 获取 epub 书籍信息（标题、作者、章节列表）。
#[command(async)]
pub async fn get_epub_info(
    path: String,
) -> Result<epub_helper::EpubInfo, QuickLookError> {
    log::info!("[cmd] get_epub_info path={}", path);
    let result = tokio::task::spawn_blocking(move || epub_helper::get_epub_info(&path))
        .await
        .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(info) => log::info!(
            "[cmd] get_epub_info 成功: title={}, chapters={}",
            info.title,
            info.chapters.len()
        ),
        Err(e) => log::error!("[cmd] get_epub_info 失败: {}", e),
    }
    result
}

/// 读取 epub 指定章节的 HTML 内容。
#[command(async)]
pub async fn get_epub_chapter(
    path: String,
    chapter_index: usize,
) -> Result<String, QuickLookError> {
    log::info!(
        "[cmd] get_epub_chapter path={}, chapter_index={}",
        path,
        chapter_index
    );
    let result = tokio::task::spawn_blocking(move || {
        epub_helper::get_epub_chapter(&path, chapter_index)
    })
    .await
    .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(_) => log::info!("[cmd] get_epub_chapter 成功"),
        Err(e) => log::error!("[cmd] get_epub_chapter 失败: {}", e),
    }
    result
}

// ── MOBI 命令 ──────────────────────────────────────

/// 获取 mobi 书籍信息（标题、作者、描述）。
#[command(async)]
pub async fn get_mobi_info(
    path: String,
) -> Result<mobi_helper::MobiInfo, QuickLookError> {
    log::info!("[cmd] get_mobi_info path={}", path);
    let result = tokio::task::spawn_blocking(move || mobi_helper::get_mobi_info(&path))
        .await
        .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(info) => log::info!(
            "[cmd] get_mobi_info 成功: title={}, author={}",
            info.title,
            info.author
        ),
        Err(e) => log::error!("[cmd] get_mobi_info 失败: {}", e),
    }
    result
}

/// 读取 mobi 文件的完整 HTML 内容。
#[command(async)]
pub async fn get_mobi_content(
    path: String,
) -> Result<String, QuickLookError> {
    log::info!("[cmd] get_mobi_content path={}", path);
    let result = tokio::task::spawn_blocking(move || mobi_helper::get_mobi_content(&path))
        .await
        .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(_) => log::info!("[cmd] get_mobi_content 成功"),
        Err(e) => log::error!("[cmd] get_mobi_content 失败: {}", e),
    }
    result
}
