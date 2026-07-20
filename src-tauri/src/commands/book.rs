use tauri::command;

use crate::error::QuickLookError;
use quicklook_book::epub as epub_helper;

// ── EPUB 命令 ──────────────────────────────────────

/// 获取 epub 书籍信息（标题、作者、章节列表）。
#[command(async)]
pub async fn get_epub_info(path: String) -> Result<epub_helper::EpubInfo, QuickLookError> {
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
    let result =
        tokio::task::spawn_blocking(move || epub_helper::get_epub_chapter(&path, chapter_index))
            .await
            .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(_) => log::info!("[cmd] get_epub_chapter 成功"),
        Err(e) => log::error!("[cmd] get_epub_chapter 失败: {}", e),
    }
    result
}

/// 解析 epub 内容中的链接 href，返回目标章节索引和锚点。
#[command(async)]
pub async fn resolve_epub_link(
    path: String,
    current_chapter_index: usize,
    href: String,
) -> Result<Option<(usize, Option<String>)>, QuickLookError> {
    log::info!(
        "[cmd] resolve_epub_link path={}, current_chapter_index={}, href={}",
        path,
        current_chapter_index,
        href
    );
    let result = tokio::task::spawn_blocking(move || {
        epub_helper::resolve_epub_link(&path, current_chapter_index, &href)
    })
    .await
    .map_err(|e| QuickLookError::DocumentParse(format!("任务执行失败: {}", e)))?;
    match &result {
        Ok(Some((idx, frag))) => {
            log::info!(
                "[cmd] resolve_epub_link 成功: chapter={}, fragment={:?}",
                idx,
                frag
            );
        },
        Ok(None) => log::info!("[cmd] resolve_epub_link: 未匹配到章节"),
        Err(e) => log::error!("[cmd] resolve_epub_link 失败: {}", e),
    }
    result
}
