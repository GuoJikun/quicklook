use epub::doc::EpubDoc;
use quicklook_error::QuickLookError;
use std::io::BufReader;
use std::path::Path;

type EpubReader = EpubDoc<BufReader<std::fs::File>>;

/// epub 章节信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpubChapter {
    /// 章节在 spine 中的索引
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
    /// 章节列表 (从 NCX 目录提取, 回退到 spine 顺序)
    pub chapters: Vec<EpubChapter>,
}

/// 解析 epub 文件，返回书籍信息和章节列表
pub fn get_epub_info(path: &str) -> Result<EpubInfo, QuickLookError> {
    log::info!("[epub] get_epub_info path={}", path);

    if !Path::new(path).exists() {
        return Err(QuickLookError::FileNotFound(path.to_string()));
    }

    let doc = EpubDoc::new(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 epub 失败: {}", e)))?;

    let title = doc.get_title().unwrap_or_else(|| "未知标题".to_string());
    let author = doc
        .mdata("creator")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "未知作者".to_string());
    let language = doc
        .mdata("language")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "zh".to_string());
    let total_chapters = doc.get_num_chapters();

    // 从 NCX 目录中提取章节
    let chapters = extract_chapters(&doc);

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

/// 从 NCX 目录或 spine 中提取章节信息
fn extract_chapters(doc: &EpubReader) -> Vec<EpubChapter> {
    let mut chapters = Vec::new();

    // 优先从 NCX 目录提取
    if !doc.toc.is_empty() {
        flatten_navpoints(&doc.toc, &doc, 0, &mut chapters);
    }

    // 如果 NCX 为空，回退到 spine 顺序
    if chapters.is_empty() {
        for (i, _spine_item) in doc.spine.iter().enumerate() {
            let title = format!("章节 {}", i + 1);
            chapters.push(EpubChapter { index: i, title, level: 0 });
        }
    }

    chapters
}

/// 递归展开 NCX 导航点，将 content 路径映射到 spine 索引
fn flatten_navpoints(
    navpoints: &[epub::doc::NavPoint],
    doc: &EpubReader,
    level: u32,
    out: &mut Vec<EpubChapter>,
) {
    for np in navpoints {
        // 将 NavPoint 的 content 路径映射到 spine 索引
        let index = doc
            .resource_uri_to_chapter(&np.content)
            .unwrap_or(out.len());

        out.push(EpubChapter { index, title: np.label.clone(), level });

        if !np.children.is_empty() {
            flatten_navpoints(&np.children, doc, level + 1, out);
        }
    }
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

    let mut doc = EpubDoc::new(path)
        .map_err(|e| QuickLookError::DocumentParse(format!("打开 epub 失败: {}", e)))?;

    if !doc.set_current_chapter(chapter_index) {
        return Err(QuickLookError::DocumentParse(format!(
            "章节索引 {} 超出范围 (共 {} 章)",
            chapter_index,
            doc.get_num_chapters()
        )));
    }

    let (content, _mime) = doc
        .get_current_str()
        .ok_or_else(|| QuickLookError::DocumentParse("读取章节内容失败".to_string()))?;

    Ok(content)
}
