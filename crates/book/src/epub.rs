use base64::Engine;
use iepub::prelude::IError;
use iepub::prelude::*;
use quicklook_error::QuickLookError;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Instant, SystemTime};

/// 缓存书籍数量上限（按最近使用淘汰）
const MAX_CACHED_BOOKS: usize = 3;

/// 一本书的章节 HTML 缓存（Arc 避免整书 clone）
#[derive(Clone)]
struct CachedBook {
    chapters: Arc<Vec<String>>,
    mtime: SystemTime,
    last_used: Instant,
}

/// 章节内容缓存：文件路径 → 章节 HTML 列表 + mtime + 最近使用时间
static EPUB_CONTENT_CACHE: LazyLock<Mutex<HashMap<String, CachedBook>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// spine 文件名列表缓存上限（仅路径字符串，可多留几本）
const MAX_SPINE_CACHE: usize = 16;

/// spine 表缓存条目：章节文件名列表，用于链接解析，避免重复打开 epub
#[derive(Clone)]
struct SpineCacheEntry {
    file_names: Arc<Vec<String>>,
    mtime: SystemTime,
    last_used: Instant,
}

/// spine 文件名缓存：文件路径 → 章节文件名列表
static EPUB_SPINE_CACHE: LazyLock<Mutex<HashMap<String, SpineCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn file_mtime(path: &str) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// 获取或加载 epub 的 spine 章节文件名列表（不读章节正文）
fn get_or_load_spine(path: &str) -> Result<Arc<Vec<String>>, QuickLookError> {
    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }
    let mtime = file_mtime(path);

    if let Ok(mut cache) = EPUB_SPINE_CACHE.lock()
        && let Some(entry) = cache.get_mut(path)
    {
        if entry.mtime == mtime {
            entry.last_used = Instant::now();
            return Ok(Arc::clone(&entry.file_names));
        }
        cache.remove(path);
    }

    log::debug!("[epub] spine 缓存未命中，加载文件列表 path={}", path);
    let book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;
    let file_names = Arc::new(
        book.chapters()
            .map(|ch| ch.file_name().to_string())
            .collect::<Vec<_>>(),
    );

    if let Ok(mut cache) = EPUB_SPINE_CACHE.lock() {
        cache.insert(
            path.to_string(),
            SpineCacheEntry {
                file_names: Arc::clone(&file_names),
                mtime,
                last_used: Instant::now(),
            },
        );
        while cache.len() > MAX_SPINE_CACHE {
            let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            cache.remove(&oldest);
        }
    }

    Ok(file_names)
}

/// 写入 spine 缓存（preload 后同步，避免下次链接再打开文件）
fn store_spine(path: &str, file_names: Arc<Vec<String>>, mtime: SystemTime) {
    if let Ok(mut cache) = EPUB_SPINE_CACHE.lock() {
        cache.insert(
            path.to_string(),
            SpineCacheEntry {
                file_names,
                mtime,
                last_used: Instant::now(),
            },
        );
        while cache.len() > MAX_SPINE_CACHE {
            let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            cache.remove(&oldest);
        }
    }
}

/// epub 章节信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpubChapter {
    /// 章节在章节列表中的索引
    pub index: usize,
    /// 章节标题
    pub title: String,
    /// 章节文件名（如 "Text/chapter1.xhtml"）
    pub file_name: String,
    /// 章节层级 (0-based, 来自 NCX)
    pub level: u32,
    /// 锚点片段（如 "sec-1"，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
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
    /// 封面图片 (base64 编码的 data URI，可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_data: Option<String>,
}

/// epub 资源条目：路径 → (MIME 类型, 原始数据)
pub type ResourceMap = HashMap<String, (String, Vec<u8>)>;

/// 根据文件扩展名推断 MIME 类型
pub fn guess_mime_type(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") || lower.ends_with(".svgz") {
        "image/svg+xml"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".css") {
        "text/css"
    } else if lower.ends_with(".ttf") {
        "font/ttf"
    } else if lower.ends_with(".otf") {
        "font/otf"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}

/// 从 epub 的 assets 构建资源映射表（路径 → MIME + 数据）
fn build_resource_map(book: &mut EpubBook) -> ResourceMap {
    let mut map = ResourceMap::new();
    for asset in book.assets_mut() {
        let file_name = asset.file_name().to_string();
        let asset_id = asset.id().to_string();
        let media_type = guess_mime_type(&file_name).to_string();
        // 必须用 data_mut() 触发惰性读取，data() 在未读取时返回 None
        if let Some(data) = asset.data_mut() {
            log::debug!(
                "[epub] 资源: name={}, id={}, mime={}, size={}",
                file_name,
                asset_id,
                media_type,
                data.len()
            );
            map.insert(file_name, (media_type, data.to_vec()));
        } else {
            log::warn!("[epub] 资源读取失败: name={}, id={}", file_name, asset_id);
        }
    }
    log::debug!("[epub] 资源映射表构建完成, 共 {} 个资源", map.len());
    map
}

/// 资源相关标签类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceTagKind {
    Img,
    Image,
    Link,
}

/// 从 `pos` 开始查找下一个资源标签
/// 返回 (标签起始, 标签名结束, `>` 后一位, 标签类型)
fn find_next_resource_tag(
    html: &str,
    pos: usize,
) -> Option<(usize, usize, usize, ResourceTagKind)> {
    let bytes = html.as_bytes();
    let mut i = pos;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let name_start = i + 1;
        if name_start >= bytes.len() {
            return None;
        }
        let mut name_end = name_start;
        while name_end < bytes.len() && bytes[name_end].is_ascii_alphanumeric() {
            name_end += 1;
        }
        if name_end == name_start {
            i = name_end.max(i + 1);
            continue;
        }
        let name = &html[name_start..name_end];
        let kind = if name.eq_ignore_ascii_case("img") {
            ResourceTagKind::Img
        } else if name.eq_ignore_ascii_case("image") {
            ResourceTagKind::Image
        } else if name.eq_ignore_ascii_case("link") {
            ResourceTagKind::Link
        } else {
            i = name_end;
            continue;
        };
        let gt_rel = html[name_end..].find('>')?;
        let tag_end = name_end + gt_rel + 1; // '>' 后一位
        return Some((i, name_end, tag_end, kind));
    }
    None
}

/// 尝试把标签属性替换为 data URI；命中返回新 tag body，未命中返回 None
fn try_inline_attr(
    tag_body: &str,
    attr_name: &str,
    attr_value: &str,
    chapter_dir: &str,
    resources: &ResourceMap,
) -> Option<String> {
    let resolved = resolve_path(chapter_dir, attr_value);
    let (mime, data) = resources.get(&resolved)?;
    if mime.is_empty() || data.is_empty() || mime.starts_with("application/octet") {
        log::debug!(
            "[epub] 资源类型不可用: resolved={}, mime={}, data_len={}",
            resolved,
            mime,
            data.len()
        );
        return None;
    }
    let data_uri = to_data_uri(mime, data);
    Some(replace_attr_value(
        tag_body, attr_name, attr_value, &data_uri,
    ))
}

/// 单遍内联 img / image / link 标签中的资源引用
fn inline_resource_tags(html: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / 2);
    let mut pos = 0;
    let mut inlined = 0usize;

    while let Some((tag_start, name_end, tag_end, kind)) = find_next_resource_tag(html, pos) {
        out.push_str(&html[pos..tag_start]);
        let gt = tag_end - 1;
        let open_tag = &html[tag_start..name_end]; // 含 `<` 与标签名
        let body = &html[name_end..gt];

        match kind {
            ResourceTagKind::Img => {
                if let Some(src) = extract_attr_value(body, "src")
                    && let Some(new_body) =
                        try_inline_attr(body, "src", &src, chapter_dir, resources)
                {
                    out.push_str(open_tag);
                    out.push_str(&new_body);
                    out.push('>');
                    inlined += 1;
                    pos = tag_end;
                    continue;
                }
                out.push_str(&html[tag_start..tag_end]);
                pos = tag_end;
            },
            ResourceTagKind::Image => {
                let (attr_name, src) = if let Some(val) = extract_attr_value(body, "xlink:href") {
                    ("xlink:href", val)
                } else if let Some(val) = extract_attr_value(body, "href") {
                    ("href", val)
                } else {
                    out.push_str(&html[tag_start..tag_end]);
                    pos = tag_end;
                    continue;
                };
                if let Some(new_body) =
                    try_inline_attr(body, attr_name, &src, chapter_dir, resources)
                {
                    out.push_str(open_tag);
                    out.push_str(&new_body);
                    out.push('>');
                    inlined += 1;
                } else {
                    out.push_str(&html[tag_start..tag_end]);
                }
                pos = tag_end;
            },
            ResourceTagKind::Link => {
                let body_lower = body.to_lowercase();
                let is_stylesheet = body_lower.contains("rel=\"stylesheet\"")
                    || body_lower.contains("rel='stylesheet'");
                let style_tag = if is_stylesheet {
                    extract_attr_value(body, "href").and_then(|href| {
                        let resolved = resolve_path(chapter_dir, &href);
                        let (mime, data) = resources.get(&resolved)?;
                        if mime != "text/css" || data.is_empty() {
                            return None;
                        }
                        let css_text = std::str::from_utf8(data).ok()?;
                        Some(format!(
                            "<style>/* Inlined from {} */\n{}</style>",
                            href, css_text
                        ))
                    })
                } else {
                    None
                };
                if let Some(style_tag) = style_tag {
                    out.push_str(&style_tag);
                    inlined += 1;
                } else {
                    out.push_str(&html[tag_start..tag_end]);
                }
                pos = tag_end;
            },
        }
    }

    out.push_str(&html[pos..]);
    if inlined > 0 {
        log::debug!("[epub] inline_resource_tags: 内联了 {} 处资源", inlined);
    }
    out
}

/// 将 HTML 中的资源路径替换为 base64 data URI
///
/// 处理以下标签/属性：
/// - `<img src="...">` / `<img src='...'>`
/// - `<image href="...">` / `<image xlink:href="...">` (SVG)
/// - `<link rel="stylesheet" href="...">`
/// - CSS 中的 `url('...')` / `url("...")`
///
/// `chapter_file` 是当前章节在 epub 中的路径（如 `Text/chapter1.xhtml`），
/// 用于解析 HTML 中的相对路径。
fn inline_resources(html: &str, chapter_file: &str, resources: &ResourceMap) -> String {
    // 获取章节所在目录（如 `Text/chapter1.xhtml` → `Text/`）
    let chapter_dir = chapter_file.rsplit_once('/').map_or("", |(dir, _)| dir);

    log::debug!(
        "[epub] inline_resources: chapter={}, dir={}, resources={}",
        chapter_file,
        chapter_dir,
        resources.len()
    );

    // 第一遍：img / image / link 标签
    let tagged = inline_resource_tags(html, chapter_dir, resources);
    // 第二遍：CSS url()（覆盖 <style>、style 属性等）
    inline_css_urls(&tagged, chapter_dir, resources)
}

/// 内联 CSS 中的 url('...') / url("...") 引用（单遍缓冲，避免整段反复复制）
pub fn inline_css_urls(css: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut out = String::with_capacity(css.len() * 2);
    let mut pos = 0;

    while let Some(url_start) = css[pos..].find("url(") {
        let abs = pos + url_start;
        out.push_str(&css[pos..abs]);
        let after_url = &css[abs + 4..];

        // 解析 url(...) 路径与表达式长度
        let parsed = parse_css_url_expr(after_url);
        match parsed {
            Some((path, expr_len)) => {
                if path.starts_with("data:") || path.is_empty() {
                    out.push_str(&css[abs..abs + 4 + expr_len]);
                    pos = abs + 4 + expr_len;
                    continue;
                }
                let resolved = resolve_path(chapter_dir, path);
                if let Some((mime, data)) = resources.get(&resolved)
                    && !mime.is_empty()
                    && !data.is_empty()
                    && !mime.starts_with("application/octet")
                {
                    let data_uri = to_data_uri(mime, data);
                    out.push_str(&format!("url(\"{}\")", data_uri));
                    pos = abs + 4 + expr_len;
                    continue;
                }
                // 未命中：保留原样
                out.push_str(&css[abs..abs + 4 + expr_len]);
                pos = abs + 4 + expr_len;
            },
            None => {
                // 无法解析：原样输出 `url(`，继续向后扫描
                out.push_str("url(");
                pos = abs + 4;
            },
        }
    }
    out.push_str(&css[pos..]);
    out
}

/// 解析 `url(` 之后的表达式，返回 (路径, 表达式长度（不含 `url(`，含 `)`）
pub fn parse_css_url_expr(after_url: &str) -> Option<(&str, usize)> {
    let (quote, path_start) = if after_url.starts_with('"') {
        (Some('"'), 1)
    } else if after_url.starts_with('\'') {
        (Some('\''), 1)
    } else {
        let end = after_url
            .find(|c: char| c == ')' || c.is_whitespace())
            .unwrap_or(after_url.len());
        let path = &after_url[..end];
        // 无引号形式：path 后应是 ) 或空白+）
        let rest = &after_url[end..];
        let close = rest.find(')').map(|i| end + i + 1)?;
        return Some((path, close));
    };

    let quote = quote?;
    let path_end = after_url[path_start..].find(quote)?;
    let path = &after_url[path_start..path_start + path_end];
    let after_quote = path_start + path_end + 1; // 闭合引号之后
    let close_rel = after_url[after_quote..].find(')')?;
    Some((path, after_quote + close_rel + 1))
}

/// 判断 `attr_pos` 处的属性名是否处于合法边界
///
/// 属性名前必须是空白、`>` 或 `/`，避免 `data-src=` 误匹配 `src=`、
/// `xlink:href=` 误匹配 `href=`。
pub fn is_attr_boundary(lower: &str, attr_pos: usize) -> bool {
    if attr_pos == 0 {
        return true;
    }
    let prev = lower.as_bytes()[attr_pos - 1];
    prev.is_ascii_whitespace() || prev == b'>' || prev == b'/'
}

/// 从 HTML 标签内容中提取指定属性的值
///
/// 支持 `attr="value"`、`attr='value'` 和无引号 `attr=value`
pub fn extract_attr_value(tag_body: &str, attr_name: &str) -> Option<String> {
    let lower = tag_body.to_lowercase();
    let attr_lower = attr_name.to_lowercase();
    let search_key = format!("{}=", attr_lower);

    // 查找属性名的位置（后面必须跟 =，且前面必须是合法属性边界）
    let mut from = 0;
    let attr_pos = loop {
        let rel = lower[from..].find(&search_key)?;
        let pos = from + rel;
        if is_attr_boundary(&lower, pos) {
            break pos;
        }
        from = pos + search_key.len();
    };
    let after_attr = &tag_body[attr_pos + search_key.len()..];

    // 跳过空白
    let after_attr = after_attr.trim_start();

    let (quote_char, value_start) = if after_attr.starts_with('"') {
        ('"', 1)
    } else if after_attr.starts_with('\'') {
        ('\'', 1)
    } else {
        // 无引号，取到下一个空白或 >
        let end = after_attr
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(after_attr.len());
        return Some(after_attr[..end].to_string());
    };

    // 找到闭合引号
    let value_end = after_attr[value_start..].find(quote_char)?;
    Some(after_attr[value_start..value_start + value_end].to_string())
}

/// 替换 HTML 标签中指定属性的值
pub fn replace_attr_value(
    tag_body: &str,
    attr_name: &str,
    old_value: &str,
    new_value: &str,
) -> String {
    // 尝试双引号形式
    let double_quoted = format!("{}=\"{}\"", attr_name, old_value);
    if tag_body.contains(&double_quoted) {
        let replacement = format!("{}=\"{}\"", attr_name, new_value);
        return tag_body.replacen(&double_quoted, &replacement, 1);
    }

    // 尝试单引号形式
    let single_quoted = format!("{}='{}'", attr_name, old_value);
    if tag_body.contains(&single_quoted) {
        let replacement = format!("{}='{}'", attr_name, new_value);
        return tag_body.replacen(&single_quoted, &replacement, 1);
    }

    tag_body.to_string()
}

/// 将 MIME 类型和数据转换为 data URI
pub fn to_data_uri(mime: &str, data: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(data);
    format!("data:{};base64,{}", mime, b64)
}

/// 解析相对路径：将 HTML 中的资源/链接路径解析为 epub 内的实际路径
///
/// 规则：
/// - `data:` / `http:` / `https:` / `mailto:` / `javascript:` 这类外部 URI 保持原样
/// - 以 `/` 开头的路径视为包内根路径，去掉前导 `/`
/// - 其他路径（包括 `image.png`、`Images/cover.png`）都按相对当前章节目录解析
///
/// 例如：`resolve_path("Text", "../Images/cover.png")` → `"Images/cover.png"`
///       `resolve_path("Text", "Images/cover.png")` → `"Text/Images/cover.png"`
pub fn resolve_path(base_dir: &str, relative: &str) -> String {
    let trimmed = relative.trim();
    if trimmed.is_empty() {
        return base_dir.to_string();
    }

    if trimmed.starts_with("data:")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
    {
        return trimmed.to_string();
    }

    let mut components = if trimmed.starts_with('/') {
        Vec::new()
    } else {
        base_dir
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    };

    for part in trimmed.trim_start_matches('/').split('/') {
        match part {
            "." | "" => {},
            ".." => {
                components.pop();
            },
            other => {
                components.push(other);
            },
        }
    }

    components.join("/")
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

/// 预加载所有章节内容到缓存，并将图片内联为 data URI
///
/// 返回 `Arc<Vec<String>>`，写入缓存时共享底层数据，避免整书 clone。
fn preload_chapters(path: &str, book: &mut EpubBook) -> Result<Arc<Vec<String>>, QuickLookError> {
    let total = book.chapters().len();

    // 构建资源映射表（图片、CSS 等）
    let resources = build_resource_map(book);

    let mut contents = Vec::with_capacity(total);
    // 收集章节文件名列表（用于图片路径解析）
    let chapter_files: Vec<String> = book
        .chapters()
        .map(|ch| ch.file_name().to_string())
        .collect();

    for (i, ch) in book.chapters_mut().enumerate() {
        let raw_html = ch.string_data();
        // 将 HTML 中的资源路径替换为 base64 data URI
        let html = if i < chapter_files.len() {
            inline_resources(&raw_html, &chapter_files[i], &resources)
        } else {
            raw_html
        };
        log::debug!(
            "[epub] 预加载章节 {} 标题={}, 长度={}",
            contents.len(),
            ch.title(),
            html.len()
        );
        contents.push(html);
    }

    // 获取文件修改时间，写入缓存时一并存储
    let mtime = file_mtime(path);

    let chapters = Arc::new(contents);

    // 同步 spine 缓存，供 resolve_epub_link 直接使用
    store_spine(path, Arc::new(chapter_files), mtime);

    // 写入缓存（共享 Arc，不 clone 底层字符串）
    if let Ok(mut cache) = EPUB_CONTENT_CACHE.lock() {
        insert_with_eviction(
            &mut cache,
            path.to_string(),
            CachedBook {
                chapters: Arc::clone(&chapters),
                mtime,
                last_used: Instant::now(),
            },
        );
    }

    Ok(chapters)
}

/// 写入缓存，超出 `MAX_CACHED_BOOKS` 时淘汰最久未使用的条目
fn insert_with_eviction(map: &mut HashMap<String, CachedBook>, key: String, entry: CachedBook) {
    map.insert(key, entry);
    while map.len() > MAX_CACHED_BOOKS {
        let Some(oldest) = map
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
        else {
            break;
        };
        log::info!("[epub] 缓存淘汰: path={}", oldest);
        map.remove(&oldest);
    }
}

/// 从缓存中读取章节内容，校验文件修改时间确保缓存有效
fn get_cached_chapter(path: &str, chapter_index: usize) -> Option<String> {
    let mut cache = EPUB_CONTENT_CACHE.lock().ok()?;

    // 校验文件修改时间：如果文件已被修改，缓存失效
    let current_mtime = file_mtime(path);

    let expired = match cache.get(path) {
        Some(entry) if current_mtime != entry.mtime => true,
        Some(_) => false,
        None => return None,
    };
    if expired {
        log::info!("[epub] 缓存已过期 (mtime 变化), path={}", path);
        cache.remove(path);
        return None;
    }

    let entry = cache.get_mut(path)?;
    entry.last_used = Instant::now();
    entry.chapters.get(chapter_index).cloned()
}

/// 从 epub 中提取封面图片，返回 base64 编码的 data URI
///
/// 提取优先级：
/// 1. meta 中的 cover-id 属性指向的资源
/// 2. assets 中名为 cover.* 的文件
/// 3. 第一个图片资源
fn extract_cover(book: &mut EpubBook) -> Option<String> {
    // 策略 1: 从 meta 中查找 cover-id
    let mut cover_id = None;
    for meta in book.meta() {
        if let Some(id) = meta.get_attr("name")
            && id.eq_ignore_ascii_case("cover")
        {
            cover_id = meta.get_attr("content").map(|s| s.to_string());
            break;
        }
    }

    if let Some(content) = cover_id {
        for asset in book.assets_mut() {
            if asset.id() == content {
                let file_name = asset.file_name().to_string();
                let mime = guess_mime_type(&file_name);
                if let Some(data) = asset.data_mut()
                    && !mime.starts_with("application/octet")
                {
                    return Some(to_data_uri(mime, data));
                }
            }
        }
    }

    // 策略 2: 查找名为 cover.* 的资源
    for asset in book.assets_mut() {
        let file_name = asset.file_name().to_string();
        let lower = file_name.to_lowercase();
        if lower.starts_with("cover") {
            let mime = guess_mime_type(&file_name);
            if let Some(data) = asset.data_mut()
                && !mime.starts_with("application/octet")
                && data.len() > 100
            {
                return Some(to_data_uri(mime, data));
            }
        }
    }

    // 策略 3: 第一个图片资源
    for asset in book.assets_mut() {
        let file_name = asset.file_name().to_string();
        let lower = file_name.to_lowercase();
        if lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp")
        {
            let mime = guess_mime_type(&file_name);
            if let Some(data) = asset.data_mut()
                && !mime.starts_with("application/octet")
                && data.len() > 1000
            {
                return Some(to_data_uri(mime, data));
            }
        }
    }

    None
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

    // 提取封面图片（需要 &mut book 触发惰性加载）
    let cover_data = extract_cover(&mut book);

    // 从目录提取章节，将目录项映射到实际的 chapter 索引
    let chapters = extract_chapters(&book);
    let actual_chapter_count = book.chapters().len();
    let total_chapters = actual_chapter_count;

    // 预加载所有章节内容到缓存
    if let Err(e) = preload_chapters(path, &mut book) {
        log::warn!("[epub] 预加载章节到缓存失败: {}", e);
    }

    log::info!(
        "[epub] 解析完成: title={}, author={}, total={}, chapters_in_toc={}, has_cover={}",
        title,
        author,
        total_chapters,
        chapters.len(),
        cover_data.is_some()
    );

    Ok(EpubInfo {
        title,
        author,
        language,
        total_chapters,
        chapters,
        cover_data,
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
                file_name: ch.file_name().to_string(),
                level: 0,
                fragment: None,
            });
        }
    }

    chapters
}

/// 递归展开目录导航点，将目录项映射到 chapters() 中的实际位置
fn flatten_navpoints(np: &EpubNav, level: u32, book: &EpubBook, out: &mut Vec<EpubChapter>) {
    // 尝试通过 file_name 匹配找到该目录项在 chapters() 中的位置
    let chapter_index = find_chapter_index(np, book);

    // 获取章节文件名
    let file_name = if chapter_index < book.chapters().len() {
        book.chapters()
            .nth(chapter_index)
            .map(|ch| ch.file_name().to_string())
            .unwrap_or_default()
    } else {
        np.file_name().to_string()
    };

    let fragment = np
        .file_name()
        .split('#')
        .nth(1)
        .filter(|frag| !frag.is_empty())
        .map(|frag| frag.to_string());

    out.push(EpubChapter {
        index: chapter_index,
        title: np.title().to_string(),
        file_name,
        level,
        fragment,
    });

    for child in np.child() {
        flatten_navpoints(child, level + 1, book, out);
    }
}

/// 检查 `haystack` 是否以 `needle` 结尾，且匹配边界是路径分隔符 `/` 或文件扩展名 `.`
///
/// 例如：`"Text/ch10.xhtml".ends_with("ch1.xhtml")` → false（`ch` 后面紧跟数字 `10`，不是边界）
///       `"Text/ch1.xhtml".ends_with("ch1.xhtml")` → true（`ch` 后面是路径分隔符 `/`）
///       `"OEBPS/ch1.xhtml".ends_with("ch1.xhtml")` → true（`ch` 前面是 `/`，后面是文件结束）
pub fn has_suffix_boundary(haystack: &str, needle: &str) -> bool {
    if !haystack.ends_with(needle) {
        return false;
    }
    if needle.is_empty() {
        return true;
    }
    // needle 在 haystack 中的起始位置
    let start = haystack.len() - needle.len();
    if start == 0 {
        // needle 完全等于 haystack，匹配
        return true;
    }
    // needle 前一个字符必须是 `/`（路径分隔符）
    let prev_char = haystack.as_bytes()[start - 1];
    prev_char == b'/'
}

/// 检查 `haystack` 是否以 `needle` 开头，且匹配边界是路径分隔符 `/`
///
/// 例如：`"ch10.xhtml".starts_with("ch1")` → false（`ch1` 后面紧跟数字 `0`，不是边界）
///       `"ch1.xhtml".starts_with("ch1")` → true（`ch1` 后面是 `.`，或 needle 就是 haystack）
///       `"OEBPS/ch1.xhtml".starts_with("ch1")` → false
pub fn has_prefix_boundary(haystack: &str, needle: &str) -> bool {
    if !haystack.starts_with(needle) {
        return false;
    }
    if needle.is_empty() || needle.len() == haystack.len() {
        return true;
    }
    // needle 后一个字符必须是 `/`（子路径）或 `.`（文件扩展名）
    let next_byte = haystack.as_bytes()[needle.len()];
    next_byte == b'/' || next_byte == b'.'
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
    // 要求匹配边界必须是路径分隔符 `/` 或文件扩展名 `.`，防止 ch10 匹配到 ch1
    for (i, ch) in chapters.iter().enumerate() {
        let ch_file = ch.file_name();
        let ch_base = ch_file.split('#').next().unwrap_or(ch_file);
        if has_suffix_boundary(ch_file, nav_file_no_frag)
            || has_suffix_boundary(nav_file_no_frag, ch_file)
            || has_suffix_boundary(ch_base, nav_file_no_frag)
            || has_suffix_boundary(nav_file_no_frag, ch_base)
        {
            return i;
        }
    }

    // 策略 3：前缀匹配（nav 文件路径以章节文件路径开头——章节内的子片段）
    // 要求匹配边界必须是路径分隔符 `/`，防止 ch10 匹配到 ch1
    for (i, ch) in chapters.iter().enumerate() {
        let ch_base = ch.file_name().split('#').next().unwrap_or(ch.file_name());
        if !ch_base.is_empty()
            && nav_file_no_frag.starts_with(ch_base)
            && has_prefix_boundary(nav_file_no_frag, ch_base)
        {
            return i;
        }
    }

    // 策略 4：位置回退——导航树的末级节点倾向于映射到靠后的章节
    // 这里保守返回最后一个有效索引
    log::warn!(
        "[epub] 目录项 '{}' 无法精确匹配到章节，回退到最后一个章节 (index={})",
        nav_file,
        total - 1
    );
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
        log::debug!("[epub] 缓存命中 chapter_index={}", chapter_index);
        return Ok(html);
    }

    // 缓存未命中，重新加载
    log::debug!(
        "[epub] 缓存未命中，重新加载 chapter_index={}",
        chapter_index
    );
    let mut book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;

    // 预加载所有章节到缓存（Arc 共享，只取当前章节）
    let chapters = preload_chapters(path, &mut book)?;

    chapters.get(chapter_index).cloned().ok_or_else(|| {
        QuickLookError::DocumentParse(format!(
            "章节索引 {} 超出范围 (共 {} 章)",
            chapter_index,
            chapters.len()
        ))
    })
}

/// 解析 epub 内容中的链接 href，返回目标章节的 spine 索引和锚点
///
/// 优先从 spine 缓存读取章节文件名列表，未命中时只加载文件名（不读正文）。
pub fn resolve_epub_link(
    path: &str,
    current_chapter_index: usize,
    href: &str,
) -> Result<Option<(usize, Option<String>)>, QuickLookError> {
    let spine = get_or_load_spine(path)?;
    Ok(match_spine_href(&spine, current_chapter_index, href))
}

/// 在 spine 文件名列表中匹配 href，返回 (章节索引, 锚点)
pub fn match_spine_href(
    spine: &[String],
    current_chapter_index: usize,
    href: &str,
) -> Option<(usize, Option<String>)> {
    // 分离路径和锚点
    let (link_path, fragment) = match href.find('#') {
        Some(pos) => (
            &href[..pos],
            Some(href[pos + 1..].to_string()).filter(|s| !s.is_empty()),
        ),
        None => (href, None),
    };

    // 获取当前章节的目录
    let current_dir = if current_chapter_index < spine.len() {
        let current_file = spine[current_chapter_index].as_str();
        current_file
            .rfind('/')
            .map(|pos| &current_file[..pos])
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    // 规范化链接路径：去掉前导 `/`，用于精确比较
    // resolve_path 会把 `/xxx` 当作包根路径，相对路径则基于 current_dir 解析
    let link_normalized = link_path.trim_start_matches('/');
    let full_path = if current_dir.is_empty() {
        link_normalized.to_string()
    } else {
        resolve_path(&current_dir, link_path)
    };

    log::debug!(
        "[epub] resolve_epub_link: href={}, link_path={}, full_path={}, current_dir={}, current_index={}",
        href,
        link_path,
        full_path,
        current_dir,
        current_chapter_index
    );

    // 在所有 spine 章节中查找匹配
    for (i, ch_file) in spine.iter().enumerate() {
        // 策略 1: 纯文件名匹配（链接无目录前缀）
        let link_file_name = link_path.rsplit('/').next().unwrap_or(link_path);
        let ch_file_name = ch_file.rsplit('/').next().unwrap_or(ch_file.as_str());

        let matched = ch_file == link_normalized
            || ch_file == &full_path
            || ch_file_name == link_file_name
            // 后缀匹配要求路径边界为 '/' 或文件结尾，避免 chapter11.xhtml 误匹配 chapter1.xhtml
            || has_suffix_boundary(ch_file, link_normalized)
            || has_suffix_boundary(ch_file, &format!("/{}", link_normalized));

        if matched {
            log::debug!(
                "[epub] resolve_epub_link: matched chapter {} file_name={}, fragment={:?}",
                i,
                ch_file,
                fragment
            );
            return Some((i, fragment));
        }
    }

    log::debug!("[epub] resolve_epub_link: no match found for href={}", href);
    None
}
