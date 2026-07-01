use mobi::Mobi;
use quicklook_error::QuickLookError;
use std::path::Path;

/// mobi 书籍信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct MobiInfo {
    /// 书名
    pub title: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 内容是否为 HTML
    pub is_html: bool,
}

/// 解析 mobi 文件，返回书籍信息
pub fn get_mobi_info(path: &str) -> Result<MobiInfo, QuickLookError> {
    log::info!("[mobi] get_mobi_info path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let book = Mobi::from_path(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 mobi 失败: {}", e)))?;

    let title = book.title();
    let author = book.author().unwrap_or_else(|| "未知作者".to_string());
    let description = book.description().unwrap_or_default();

    // 尝试获取内容来判断是否为 HTML（使用 lossy 避免编码问题导致元数据获取失败）
    let is_html = book
        .content_as_string_lossy()
        .trim_start()
        .starts_with('<');

    log::info!(
        "[mobi] 解析完成: title={}, author={}, is_html={}",
        title,
        author,
        is_html
    );

    Ok(MobiInfo { title, author, description, is_html })
}

/// 读取 mobi 文件的完整 HTML 内容（先尝试严格解码，失败时回退到 lossy）
pub fn get_mobi_content(path: &str) -> Result<String, QuickLookError> {
    log::info!("[mobi] get_mobi_content path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let book = Mobi::from_path(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 mobi 失败: {}", e)))?;

    // 先尝试严格解码，失败时回退到 lossy
    match book.content_as_string() {
        Ok(content) => Ok(content),
        Err(_) => {
            log::warn!("[mobi] 严格解码失败，使用 lossy 回退");
            Ok(book.content_as_string_lossy())
        }
    }
}

/// 直接以 lossy 方式读取 mobi 文件内容（无错误路径）
pub fn get_mobi_content_lossy(path: &str) -> Result<String, QuickLookError> {
    log::info!("[mobi] get_mobi_content_lossy path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let book = Mobi::from_path(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 mobi 失败: {}", e)))?;

    let content = book.content_as_string_lossy();
    Ok(content)
}
