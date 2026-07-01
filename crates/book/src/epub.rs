use iepub::prelude::*;
use iepub::prelude::IError;
use quicklook_error::QuickLookError;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

/// 章节内容缓存：文件路径 → (总章节数, 各章节 HTML)
static EPUB_CONTENT_CACHE: Mutex<Option<HashMap<String, (usize, Vec<String>)>>> =
    Mutex::new(None);

/// epub 章节信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpubChapter {
    /// 章节在章节列表中的索引
    pub index: usize,
    /// 章节标题
    pub title: String,
    /// 章节层级 (0-based, 来自 NCX)
    pub level: u32,
}

/// epub 书籍元信息 + 章节列表
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpubInfo {
    /// 书名
    pub title: String,
    /// 作者
    pub author: String,
    /// 语言
    pub language: String,
    /// 总章节数
    pub total_chapters: usize,
    /// 章节列表 (从目录提取)
    pub chapters: Vec<EpubChapter>,
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

/// 预加载所有章节内容到缓存
fn preload_chapters(path: &str, book: &mut EpubBook) -> Result<Vec<String>, QuickLookError> {
    let total = book.chapters().len();
    let mut contents = Vec::with_capacity(total);
    for ch in book.chapters_mut() {
        let html = ch.string_data();
        log::info!(
            "[epub] 预加载章节 {} 标题={}, 长度={}",
            contents.len(),
            ch.title(),
            html.len()
        );
        contents.push(html);
    }

    // 写入缓存
    if let Ok(mut cache) = EPUB_CONTENT_CACHE.lock() {
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(path.to_string(), (total, contents.clone()));
    }

    Ok(contents)
}

/// 从缓存中读取章节内容
fn get_cached_chapter(path: &str, chapter_index: usize) -> Option<String> {
    let cache = EPUB_CONTENT_CACHE.lock().ok()?;
    let map = cache.as_ref()?;
    let (total, contents) = map.get(path)?;
    if chapter_index < *total {
        contents.get(chapter_index).cloned()
    } else {
        None
    }
}

/// 解析 epub 文件，返回书籍信息和章节列表
pub fn get_epub_info(path: &str) -> Result<EpubInfo, QuickLookError> {
    log::info!("[epub] get_epub_info path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let mut book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;

    let title = book.title().to_string();
    let author = book
        .creator()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "未知作者".to_string());
    let language = book
        .language()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "zh".to_string());

    // 从目录提取章节，将目录项映射到实际的 chapter 索引
    let chapters = extract_chapters(&book);
    let total_chapters = chapters.len();

    // 预加载所有章节内容到缓存
    let _ = preload_chapters(path, &mut book);

    log::info!(
        "[epub] 解析完成: title={}, author={}, total={}, chapters_in_toc={}",
        title,
        author,
        total_chapters,
        chapters.len()
    );

    Ok(EpubInfo {
        title,
        author,
        language,
        total_chapters,
        chapters,
    })
}

/// 从目录中提取章节信息（优先 nav，回退到 chapters）
fn extract_chapters(book: &EpubBook) -> Vec<EpubChapter> {
    let mut chapters = Vec::new();

    // 优先从 nav（目录树）提取
    for np in book.nav() {
        flatten_navpoints(np, 0, book, &mut chapters);
    }

    // 如果目录为空，回退到章节顺序
    if chapters.is_empty() {
        for (i, ch) in book.chapters().enumerate() {
            chapters.push(EpubChapter {
                index: i,
                title: ch.title().to_string(),
                level: 0,
            });
        }
    }

    chapters
}

/// 递归展开目录导航点，将目录项映射到 chapters() 中的实际位置
fn flatten_navpoints(np: &EpubNav, level: u32, book: &EpubBook, out: &mut Vec<EpubChapter>) {
    // 尝试通过 file_name 匹配找到该目录项在 chapters() 中的位置
    let chapter_index = find_chapter_index(np, book);

    out.push(EpubChapter {
        index: chapter_index,
        title: np.title().to_string(),
        level,
    });

    for child in np.child() {
        flatten_navpoints(child, level + 1, book, out);
    }
}

/// 通过 EpubNav 的 file_name 与 EpubHtml 的 file_name 匹配，找到章节在 chapters() 中的索引
///
/// 处理以下常见情况：
/// - 精确匹配：nav="chapter1.xhtml" → ch="chapter1.xhtml"
/// - 片段引用：nav="chapter1.xhtml#sec2" → ch="chapter1.xhtml"（去除 # 后匹配）
/// - 路径差异：nav="OEBPS/Text/ch1.xhtml" → ch="Text/ch1.xhtml"（后缀匹配）
/// - 子路径：nav="Text/ch1.xhtml" → ch="OEBPS/Text/ch1.xhtml"（前缀匹配）
fn find_chapter_index(np: &EpubNav, book: &EpubBook) -> usize {
    let nav_file = np.file_name();
    // 去除片段标识（# 后面的部分）
    let nav_file_no_frag = nav_file.split('#').next().unwrap_or(nav_file);
    let chapters: Vec<&EpubHtml> = book.chapters().collect();

    if nav_file_no_frag.is_empty() {
        return 0;
    }

    let total = chapters.len();
    if total == 0 {
        return 0;
    }

    // 策略 1：精确匹配
    for (i, ch) in chapters.iter().enumerate() {
        if ch.file_name() == nav_file || ch.file_name() == nav_file_no_frag {
            return i;
        }
    }

    // 策略 2：后缀匹配（nav 路径以章节路径结尾，或章节路径以 nav 路径结尾）
    for (i, ch) in chapters.iter().enumerate() {
        let ch_file = ch.file_name();
        let ch_base = ch_file.split('#').next().unwrap_or(ch_file);
        if ch_file.ends_with(nav_file_no_frag)
            || nav_file_no_frag.ends_with(ch_file)
            || ch_base.ends_with(nav_file_no_frag)
            || nav_file_no_frag.ends_with(ch_base)
        {
            return i;
        }
    }

    // 策略 3：前缀匹配（nav 文件路径以章节文件路径开头——章节内的子片段）
    for (i, ch) in chapters.iter().enumerate() {
        let ch_base = ch.file_name().split('#').next().unwrap_or(ch.file_name());
        if !ch_base.is_empty() && nav_file_no_frag.starts_with(ch_base) {
            return i;
        }
    }

    // 策略 4：位置回退——导航树的末级节点倾向于映射到靠后的章节
    // 这里保守返回最后一个有效索引
    total - 1
}

/// 读取 epub 中指定章节的 HTML 内容
pub fn get_epub_chapter(path: &str, chapter_index: usize) -> Result<String, QuickLookError> {
    log::info!(
        "[epub] get_epub_chapter path={}, chapter_index={}",
        path,
        chapter_index
    );

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    // 优先从缓存读取
    if let Some(html) = get_cached_chapter(path, chapter_index) {
        log::info!("[epub] 缓存命中 chapter_index={}", chapter_index);
        return Ok(html);
    }

    // 缓存未命中，重新加载
    log::info!("[epub] 缓存未命中，重新加载 chapter_index={}", chapter_index);
    let mut book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;

    // 预加载所有章节到缓存（同时也返回当前请求的章节）
    let contents = preload_chapters(path, &mut book)?;

    contents
        .into_iter()
        .nth(chapter_index)
        .ok_or_else(|| {
            QuickLookError::DocumentParse(format!(
                "章节索引 {} 超出范围 (共 {} 章)",
                chapter_index,
                book.chapters().len()
            ))
        })
}
