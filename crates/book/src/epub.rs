use base64::Engine;
use iepub::prelude::IError;
use iepub::prelude::*;
use quicklook_error::QuickLookError;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

/// 章节内容缓存：文件路径 → (总章节数, 各章节 HTML, 文件修改时间)
static EPUB_CONTENT_CACHE: Mutex<Option<HashMap<String, (usize, Vec<String>, SystemTime)>>> =
    Mutex::new(None);

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
type ResourceMap = HashMap<String, (String, Vec<u8>)>;

/// 根据文件扩展名推断 MIME 类型
fn guess_mime_type(file_name: &str) -> &'static str {
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
    } else if lower.ends_with(".ttf") || lower.ends_with(".otf") {
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
            log::info!(
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
    log::info!("[epub] 资源映射表构建完成, 共 {} 个资源", map.len());
    map
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

    log::info!(
        "[epub] inline_resources: chapter={}, dir={}, resources={}",
        chapter_file,
        chapter_dir,
        resources.len()
    );

    let mut result = html.to_string();

    // 处理 <img src="..."> 标签
    result = inline_img_tags(&result, chapter_dir, resources);

    // 处理 <image href="..."> 和 <image xlink:href="..."> 标签 (SVG)
    result = inline_image_tags(&result, chapter_dir, resources);

    // 处理 <link rel="stylesheet" href="..."> 标签
    result = inline_link_tags(&result, chapter_dir, resources);

    // 处理 CSS 中的 url() 引用
    result = inline_css_urls(&result, chapter_dir, resources);

    result
}

/// 内联 <img src="..."> 标签
fn inline_img_tags(html: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut result = html.to_string();
    let mut pos = 0;
    let mut inlined_count = 0;

    while let Some(img_start) = result[pos..].find("<img") {
        let abs_img_start = pos + img_start;

        // 找到标签的结束位置（>）
        let tag_content_start = abs_img_start + 4; // 跳过 "<img"
        let Some(tag_end_rel) = result[tag_content_start..].find('>') else {
            break;
        };
        let tag_end = tag_content_start + tag_end_rel;

        // 在标签内查找 src="..." 或 src='...'
        let tag_body = &result[tag_content_start..tag_end];
        if let Some(src_value) = extract_attr_value(tag_body, "src") {
            let resolved = resolve_path(chapter_dir, &src_value);
            log::info!("[epub] img src原始值={}, 解析后={}", src_value, resolved);
            if let Some((mime, data)) = resources.get(&resolved) {
                if !mime.is_empty() && !data.is_empty() && !mime.starts_with("application/octet") {
                    let data_uri = to_data_uri(mime, data);
                    let tag_body_new = replace_attr_value(tag_body, "src", &src_value, &data_uri);
                    result = format!(
                        "{}{}{}",
                        &result[..tag_content_start],
                        tag_body_new,
                        &result[tag_end..]
                    );
                    inlined_count += 1;
                } else {
                    log::warn!(
                        "[epub] 图片资源类型不可用: mime={}, data_len={}",
                        mime,
                        data.len()
                    );
                }
            } else {
                log::warn!(
                    "[epub] 未找到图片资源: resolved={}, 资源表有{}个条目",
                    resolved,
                    resources.len()
                );
            }
        }

        pos = tag_end + 1;
    }

    if inlined_count > 0 {
        log::info!("[epub] inline_img_tags: 内联了 {} 个图片", inlined_count);
    }

    result
}

/// 内联 <image href="..."> 和 <image xlink:href="..."> 标签 (SVG)
fn inline_image_tags(html: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut result = html.to_string();
    let mut pos = 0;

    while let Some(img_start) = result[pos..].find("<image") {
        let abs_img_start = pos + img_start;

        // 找到标签的结束位置（> 或 />）
        let tag_content_start = abs_img_start + 6; // 跳过 "<image"
        let Some(tag_end_rel) = result[tag_content_start..].find('>') else {
            break;
        };
        let tag_end = tag_content_start + tag_end_rel;

        let tag_body = &result[tag_content_start..tag_end];

        // 优先检查 xlink:href，然后是 href
        let (attr_name, src_value) = if let Some(val) = extract_attr_value(tag_body, "xlink:href") {
            ("xlink:href", val)
        } else if let Some(val) = extract_attr_value(tag_body, "href") {
            ("href", val)
        } else {
            pos = tag_end + 1;
            continue;
        };

        let resolved = resolve_path(chapter_dir, &src_value);
        if let Some((mime, data)) = resources.get(&resolved) {
            if !mime.is_empty() && !data.is_empty() && !mime.starts_with("application/octet") {
                let data_uri = to_data_uri(mime, data);
                let tag_body_new = replace_attr_value(tag_body, attr_name, &src_value, &data_uri);
                result = format!(
                    "{}{}{}",
                    &result[..tag_content_start],
                    tag_body_new,
                    &result[tag_end..]
                );
            }
        }

        pos = tag_end + 1;
    }
    result
}

/// 内联 <link rel="stylesheet" href="..."> 标签，将 CSS 转为 <style> 标签
fn inline_link_tags(html: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut result = html.to_string();
    let mut pos = 0;

    while let Some(link_start) = result[pos..].find("<link") {
        let abs_link_start = pos + link_start;

        // 找到标签的结束位置（>）
        let tag_content_start = abs_link_start + 5; // 跳过 "<link"
        let Some(tag_end_rel) = result[tag_content_start..].find('>') else {
            break;
        };
        let tag_end = tag_content_start + tag_end_rel;

        let tag_body = &result[tag_content_start..tag_end].to_lowercase();

        // 检查是否是 stylesheet
        if tag_body.contains("rel=\"stylesheet\"") || tag_body.contains("rel='stylesheet'") {
            if let Some(href_value) =
                extract_attr_value(&result[tag_content_start..tag_end], "href")
            {
                let resolved = resolve_path(chapter_dir, &href_value);
                if let Some((mime, data)) = resources.get(&resolved) {
                    if mime == "text/css" && !data.is_empty() {
                        if let Ok(css_text) = std::str::from_utf8(data) {
                            // 处理 CSS 中的 url() 引用
                            let css_with_inlined =
                                inline_css_urls(css_text, chapter_dir, resources);
                            let style_tag = format!(
                                "<style>/* Inlined from {} */\n{}</style>",
                                href_value, css_with_inlined
                            );
                            result = format!(
                                "{}{}{}",
                                &result[..abs_link_start],
                                style_tag,
                                &result[tag_end + 1..]
                            );
                            // 跳过新插入的 style 标签
                            pos = abs_link_start + style_tag.len();
                            continue;
                        }
                    }
                }
            }
        }

        pos = tag_end + 1;
    }
    result
}

/// 内联 CSS 中的 url('...') / url("...") 引用
fn inline_css_urls(css: &str, chapter_dir: &str, resources: &ResourceMap) -> String {
    let mut result = css.to_string();
    let mut pos = 0;

    while let Some(url_start) = result[pos..].find("url(") {
        let abs_url_start = pos + url_start;
        let after_url = &result[abs_url_start + 4..];

        // 找到引号包裹的路径
        let (quote_char, path_start) = if after_url.starts_with('"') {
            ('"', 1)
        } else if after_url.starts_with('\'') {
            ('\'', 1)
        } else {
            // 无引号，取到下一个 ) 或空白
            let end = after_url
                .find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(after_url.len());
            let path = &after_url[..end];
            if !path.is_empty() && !path.starts_with("data:") {
                let resolved = resolve_path(chapter_dir, path);
                if let Some((mime, data)) = resources.get(&resolved) {
                    if !mime.is_empty()
                        && !data.is_empty()
                        && !mime.starts_with("application/octet")
                    {
                        let data_uri = to_data_uri(mime, data);
                        let replacement = format!("url({})", data_uri);
                        result = format!(
                            "{}{}{}",
                            &result[..abs_url_start],
                            replacement,
                            &result[abs_url_start + 4 + end..]
                        );
                        pos = abs_url_start + replacement.len();
                        continue;
                    }
                }
            }
            pos = abs_url_start + 4 + end + 1; // +1 for )
            continue;
        };

        // 找到闭合引号
        let Some(path_end) = after_url[path_start..].find(quote_char) else {
            pos = abs_url_start + 4;
            continue;
        };
        let path = &after_url[path_start..path_start + path_end];

        // 跳过已经内联的 data: URI
        if !path.starts_with("data:") && !path.is_empty() {
            let resolved = resolve_path(chapter_dir, path);
            if let Some((mime, data)) = resources.get(&resolved) {
                if !mime.is_empty() && !data.is_empty() && !mime.starts_with("application/octet") {
                    let data_uri = to_data_uri(mime, data);
                    // 替换整个 url(...) 表达式
                    let full_url_end = path_start + path_end + 1; // +1 for closing quote
                    let after_paren = &after_url[full_url_end..];
                    let close_paren = after_paren.find(')').map(|i| i + 1).unwrap_or(0);
                    let total_len = 4 + full_url_end + close_paren; // "url(" + content + ")"
                    let replacement = format!("url(\"{}\")", data_uri);
                    result = format!(
                        "{}{}{}",
                        &result[..abs_url_start],
                        replacement,
                        &result[abs_url_start + total_len..]
                    );
                    pos = abs_url_start + replacement.len();
                    continue;
                }
            }
        }

        pos = abs_url_start + 4 + path_end + 1; // 跳过 url("...")
    }
    result
}

/// 从 HTML 标签内容中提取指定属性的值
///
/// 支持 `attr="value"`、`attr='value'` 和无引号 `attr=value`
fn extract_attr_value(tag_body: &str, attr_name: &str) -> Option<String> {
    let lower = tag_body.to_lowercase();
    let attr_lower = attr_name.to_lowercase();

    // 查找属性名的位置（后面必须跟 =）
    let attr_pos = lower.find(&format!("{}=", attr_lower))?;
    let after_attr = &tag_body[attr_pos + attr_name.len() + 1..];

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
fn replace_attr_value(tag_body: &str, attr_name: &str, old_value: &str, new_value: &str) -> String {
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
fn to_data_uri(mime: &str, data: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(data);
    format!("data:{};base64,{}", mime, b64)
}

/// 解析相对路径：将 HTML 中的 src 路径解析为 epub 内的绝对路径
///
/// 例如：`resolve_path("Text", "../Images/cover.png")` → `"Images/cover.png"`
fn resolve_path(base_dir: &str, relative: &str) -> String {
    // 如果是绝对路径（不以 . 或 .. 开头），直接返回
    if !relative.starts_with('.') {
        return relative.to_string();
    }

    let mut components: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();

    for part in relative.split('/') {
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
fn preload_chapters(path: &str, book: &mut EpubBook) -> Result<Vec<String>, QuickLookError> {
    let total = book.chapters().len();

    // 清空旧缓存，确保使用最新的内联逻辑
    if let Ok(mut cache) = EPUB_CONTENT_CACHE.lock() {
        *cache = None;
    }

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
        log::info!(
            "[epub] 预加载章节 {} 标题={}, 长度={}",
            contents.len(),
            ch.title(),
            html.len()
        );
        contents.push(html);
    }

    // 获取文件修改时间，写入缓存时一并存储
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // 写入缓存
    if let Ok(mut cache) = EPUB_CONTENT_CACHE.lock() {
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(path.to_string(), (total, contents.clone(), mtime));
    }

    Ok(contents)
}

/// 从缓存中读取章节内容，校验文件修改时间确保缓存有效
fn get_cached_chapter(path: &str, chapter_index: usize) -> Option<String> {
    let cache = EPUB_CONTENT_CACHE.lock().ok()?;
    let map = cache.as_ref()?;
    let (total, contents, cached_mtime) = map.get(path)?;

    // 校验文件修改时间：如果文件已被修改，缓存失效
    let current_mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    if current_mtime != *cached_mtime {
        log::info!("[epub] 缓存已过期 (mtime 变化), path={}", path);
        return None;
    }

    if chapter_index < *total {
        contents.get(chapter_index).cloned()
    } else {
        None
    }
}

/// 从 epub 中提取封面图片，返回 base64 编码的 data URI
///
/// 提取优先级：
/// 1. meta 中的 cover-id 属性指向的资源
/// 2. assets 中名为 cover.* 的文件
/// 3. 第一个图片资源
fn extract_cover(book: &mut EpubBook, _resources: &ResourceMap) -> Option<String> {
    // 策略 1: 从 meta 中查找 cover-id
    let mut cover_id = None;
    for meta in book.meta() {
        if let Some(id) = meta.get_attr("name") {
            if id.eq_ignore_ascii_case("cover") {
                cover_id = meta.get_attr("content").map(|s| s.to_string());
                break;
            }
        }
    }

    if let Some(content) = cover_id {
        for asset in book.assets_mut() {
            if asset.id() == content {
                let file_name = asset.file_name().to_string();
                let mime = guess_mime_type(&file_name);
                if let Some(data) = asset.data_mut() {
                    if !mime.starts_with("application/octet") {
                        return Some(to_data_uri(mime, data));
                    }
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
            if let Some(data) = asset.data_mut() {
                if !mime.starts_with("application/octet") && data.len() > 100 {
                    return Some(to_data_uri(mime, data));
                }
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
            if let Some(data) = asset.data_mut() {
                if !mime.starts_with("application/octet") && data.len() > 1000 {
                    return Some(to_data_uri(mime, data));
                }
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
    let cover_data = extract_cover(&mut book, &ResourceMap::new());

    // 构建资源映射表（用于图片内联）
    let resources = build_resource_map(&mut book);

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

    out.push(EpubChapter {
        index: chapter_index,
        title: np.title().to_string(),
        file_name,
        level,
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
fn has_suffix_boundary(haystack: &str, needle: &str) -> bool {
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
fn has_prefix_boundary(haystack: &str, needle: &str) -> bool {
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
        log::info!("[epub] 缓存命中 chapter_index={}", chapter_index);
        return Ok(html);
    }

    // 缓存未命中，重新加载
    log::info!(
        "[epub] 缓存未命中，重新加载 chapter_index={}",
        chapter_index
    );
    let mut book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;

    // 预加载所有章节到缓存（同时也返回当前请求的章节）
    let contents = preload_chapters(path, &mut book)?;

    contents.into_iter().nth(chapter_index).ok_or_else(|| {
        QuickLookError::DocumentParse(format!(
            "章节索引 {} 超出范围 (共 {} 章)",
            chapter_index,
            book.chapters().len()
        ))
    })
}

/// 解析 epub 内容中的链接 href，返回目标章节的 spine 索引和锚点
pub fn resolve_epub_link(
    path: &str,
    current_chapter_index: usize,
    href: &str,
) -> Result<Option<(usize, Option<String>)>, QuickLookError> {
    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let mut book = read_from_file(path).map_err(|e| map_iepub_error(path, "打开 epub", e))?;

    // 分离路径和锚点
    let (link_path, fragment) = match href.find('#') {
        Some(pos) => (&href[..pos], Some(href[pos + 1..].to_string())),
        None => (href, None),
    };

    // 获取当前章节的目录
    let current_dir = if current_chapter_index < book.chapters().len() {
        let current_file = book
            .chapters()
            .nth(current_chapter_index)
            .map(|ch| ch.file_name().to_string())
            .unwrap_or_default();
        // 提取目录部分
        current_file
            .rfind('/')
            .map(|pos| &current_file[..pos])
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    log::info!(
        "[epub] resolve_epub_link: href={}, link_path={}, current_dir={}, current_index={}",
        href,
        link_path,
        current_dir,
        current_chapter_index
    );

    // 在所有 spine 章节中查找匹配
    for (i, ch) in book.chapters().enumerate() {
        let ch_file = ch.file_name();

        // 策略 1: 纯文件名匹配（链接无目录前缀）
        let link_file_name = link_path.rsplit('/').next().unwrap_or(link_path);
        let ch_file_name = ch_file.rsplit('/').next().unwrap_or(ch_file);

        // 策略 2: 带目录前缀匹配（处理 ./、../）
        let full_path = if current_dir.is_empty() {
            link_path.to_string()
        } else if link_path.starts_with("./") || link_path.starts_with("../") {
            resolve_path(&current_dir, link_path)
        } else {
            format!("{}/{}", current_dir, link_path)
        };

        let matched = ch_file == link_path
            || ch_file == full_path
            || ch_file_name == link_file_name
            || ch_file.ends_with(link_path)
            || ch_file.ends_with(&format!("/{}", link_path));

        if matched {
            log::info!(
                "[epub] resolve_epub_link: matched chapter {} file_name={}, fragment={:?}",
                i,
                ch_file,
                fragment
            );
            return Ok(Some((i, fragment)));
        }
    }

    log::warn!("[epub] resolve_epub_link: no match found for href={}", href);
    Ok(None)
}
