use iepub::prelude::IError;
use iepub::prelude::*;
use quicklook_error::QuickLookError;
use std::fs::File;
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

/// 将 iepub 错误映射为 QuickLookError
fn map_iepub_error(_path: &str, context: &str, e: IError) -> QuickLookError {
    let msg = match &e {
        IError::Io(io_err) => format!("{} IO 错误: {}", context, io_err),
        IError::InvalidArchive(reason) => format!("{} 无效文件: {}", context, reason),
        IError::UnsupportedArchive(reason) => format!("{} 不支持的格式: {}", context, reason),
        IError::NoNav(reason) => format!("{} 缺少目录: {}", context, reason),
        IError::Xml(xml_err) => format!("{} XML 解析错误: {}", context, xml_err),
        IError::Utf8(utf8_err) => format!("{} 编码错误: {}", context, utf8_err),
        _ => format!("{} 未知错误: {}", context, e),
    };
    QuickLookError::DocumentParse(msg)
}

/// 解析 mobi 文件，返回书籍信息
pub fn get_mobi_info(path: &str) -> Result<MobiInfo, QuickLookError> {
    log::info!("[mobi] get_mobi_info path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let file = File::open(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 mobi 失败: {}", e)))?;
    let mut reader = MobiReader::new(file).map_err(|e| map_iepub_error(path, "打开 mobi", e))?;
    let book = reader
        .load()
        .map_err(|e| map_iepub_error(path, "解析 mobi", e))?;

    let title = book.title().to_string();
    let author = book
        .creator()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "未知作者".to_string());
    let description = book
        .description()
        .map(|s| s.to_string())
        .unwrap_or_default();

    // 判断内容是否为 HTML
    let is_html = book
        .chapters()
        .next()
        .and_then(|ch| ch.data())
        .map(|d| d.starts_with(b"<"))
        .unwrap_or(false);

    log::info!(
        "[mobi] 解析完成: title={}, author={}, is_html={}",
        title,
        author,
        is_html
    );

    Ok(MobiInfo { title, author, description, is_html })
}

/// 读取 mobi 文件的完整 HTML 内容（拼接所有章节）
pub fn get_mobi_content(path: &str) -> Result<String, QuickLookError> {
    log::info!("[mobi] get_mobi_content path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let file = File::open(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 mobi 失败: {}", e)))?;
    let mut reader = MobiReader::new(file).map_err(|e| map_iepub_error(path, "打开 mobi", e))?;
    let book = reader
        .load()
        .map_err(|e| map_iepub_error(path, "解析 mobi", e))?;

    // 拼接所有章节的 HTML 内容
    let mut full_html = String::new();
    for ch in book.chapters() {
        if let Some(data) = ch.data() {
            full_html.push_str(&String::from_utf8_lossy(data));
        }
    }

    Ok(full_html)
}

/// 读取 mobi 文件的内容（容错版本，编码错误时返回 lossy 字符串）
pub fn get_mobi_content_lossy(path: &str) -> Result<String, QuickLookError> {
    // iepub 内部已处理编码问题，与 get_mobi_content 行为一致
    get_mobi_content(path)
}
