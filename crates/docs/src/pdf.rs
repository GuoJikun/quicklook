use pdfium_render::prelude::{PdfPageRenderRotation, *};
use quicklook_error::QuickLookError;
use std::sync::{Mutex, OnceLock};

fn normalize_rotation(rotation: u32) -> u32 {
    match rotation % 360 {
        90 => 90,
        180 => 180,
        270 => 270,
        _ => 0,
    }
}

fn rotation_to_pdfium(rotation: u32) -> PdfPageRenderRotation {
    match normalize_rotation(rotation) {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RenderedPage {
    pub page_num: u32,
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OutlineItem {
    pub title: String,
    pub page: u32,
    pub items: Vec<OutlineItem>,
}

fn get_pdfium() -> Result<&'static Pdfium, QuickLookError> {
    static INSTANCE: OnceLock<Pdfium> = OnceLock::new();
    Ok(INSTANCE.get_or_init(|| {
        let dll_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let lib_name = Pdfium::pdfium_platform_library_name_at_path(&dll_dir);

        log::info!("[pdf] dll_dir={}", dll_dir.display());
        log::info!("[pdf] lib_name={}", lib_name.display());

        let bindings = Pdfium::bind_to_library(&lib_name).expect("加载 pdfium 库失败");

        log::info!("[pdf] pdfium 库加载成功");

        Pdfium::new(bindings)
    }))
}

struct DocCache {
    path: String,
    doc: PdfDocument<'static>,
}

type DocMutex = Mutex<Option<DocCache>>;

fn doc_mutex() -> &'static DocMutex {
    static CACHE: OnceLock<DocMutex> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn ensure_doc(
    path: &str,
) -> Result<std::sync::MutexGuard<'static, Option<DocCache>>, QuickLookError> {
    let mutex = doc_mutex();
    let mut guard = mutex
        .lock()
        .map_err(|e| QuickLookError::PdfRendering(format!("锁竞争失败: {}", e)))?;

    if guard.as_ref().map_or(true, |c| c.path != path) {
        let pdfium = get_pdfium()?;
        let doc = pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| QuickLookError::PdfRendering(format!("打开 PDF 失败: {}", e)))?;
        *guard = Some(DocCache { path: path.to_string(), doc });
        log::info!("[pdf] 已缓存文档: {}", path);
    }

    Ok(guard)
}

pub fn get_pdf_page_count(path: &str) -> Result<u32, QuickLookError> {
    log::info!("[pdf] get_pdf_page_count path={}", path);
    let guard = ensure_doc(path)?;
    let cache = guard
        .as_ref()
        .ok_or_else(|| QuickLookError::PdfRendering("文档未加载".into()))?;
    let count = cache.doc.pages().len() as u32;
    log::info!("[pdf] page_count={}", count);
    Ok(count)
}

pub fn get_pdf_outline(path: &str) -> Result<Vec<OutlineItem>, QuickLookError> {
    log::info!("[pdf] get_pdf_outline path={}", path);
    let guard = ensure_doc(path)?;
    let cache = guard
        .as_ref()
        .ok_or_else(|| QuickLookError::PdfRendering("文档未加载".into()))?;

    let bookmarks = cache.doc.bookmarks();

    fn collect_children(bookmark: &pdfium_render::prelude::PdfBookmark<'_>) -> Vec<OutlineItem> {
        let mut items = Vec::new();
        let mut child = bookmark.first_child();
        while let Some(c) = child {
            let title = c.title().unwrap_or_default();
            let page = c
                .destination()
                .and_then(|dest| dest.page_index().ok())
                .map(|idx| idx as u32 + 1)
                .unwrap_or(0);
            let sub_items = collect_children(&c);
            if !title.is_empty() {
                items.push(OutlineItem { title, page, items: sub_items });
            }
            child = c.next_sibling();
        }
        items
    }

    let mut items = Vec::new();
    let mut current = bookmarks.root();
    while let Some(bm) = current {
        let title = bm.title().unwrap_or_default();
        let page = bm
            .destination()
            .and_then(|dest| dest.page_index().ok())
            .map(|idx| idx as u32 + 1)
            .unwrap_or(0);
        let sub_items = collect_children(&bm);
        if !title.is_empty() {
            items.push(OutlineItem { title, page, items: sub_items });
        }
        current = bm.next_sibling();
    }

    Ok(items)
}

pub fn render_pdf_page(
    path: &str,
    page_index: u32,
    dpi: u32,
    rotation: u32,
) -> Result<RenderedPage, QuickLookError> {
    log::info!(
        "[pdf] render_pdf_page path={}, page_index={}, dpi={}, rotation={}",
        path,
        page_index,
        dpi,
        rotation
    );

    let cache_dir = std::env::temp_dir().join("quicklook_pdf");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| QuickLookError::PdfRendering(format!("创建缓存目录失败: {}", e)))?;

    let file_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                modified.hash(&mut hasher);
            }
        }
        hasher.finish()
    };

    let normalized_rotation = normalize_rotation(rotation);
    let cache_path = cache_dir.join(format!(
        "pdf_{:x}_{}_{}_{}.png",
        file_hash, page_index, dpi, normalized_rotation
    ));

    if cache_path.exists() {
        if let Ok(img) = image::ImageReader::open(&cache_path) {
            if let Ok((w, h)) = img.into_dimensions() {
                log::info!("[pdf] page {} 命中缓存", page_index + 1);
                return Ok(RenderedPage {
                    page_num: page_index + 1,
                    path: cache_path.to_string_lossy().to_string(),
                    width: w,
                    height: h,
                });
            }
        }
    }

    let guard = ensure_doc(path)?;
    let cache = guard
        .as_ref()
        .ok_or_else(|| QuickLookError::PdfRendering("文档未加载".into()))?;
    let pages = cache.doc.pages();
    let page_count = pages.len();
    if page_index >= page_count as u32 {
        return Err(QuickLookError::PdfRendering(format!(
            "页索引 {} 超出范围 (共 {} 页)",
            page_index, page_count
        )));
    }

    log::info!("[pdf] 渲染第 {} 页...", page_index + 1);
    let page = pages.get(page_index as i32).map_err(|e| {
        QuickLookError::PdfRendering(format!("获取第 {} 页失败: {}", page_index + 1, e))
    })?;

    let render_config = PdfRenderConfig::default()
        .scale_page_by_factor(dpi as f32 / 72.0)
        .rotate(rotation_to_pdfium(rotation), false);

    let bitmap = page.render_with_config(&render_config).map_err(|e| {
        QuickLookError::PdfRendering(format!("渲染第 {} 页失败: {}", page_index + 1, e))
    })?;

    let width = bitmap.width() as u32;
    let height = bitmap.height() as u32;
    log::info!(
        "[pdf] page {} 渲染完成: {}x{}",
        page_index + 1,
        width,
        height
    );

    let dynamic_image = bitmap
        .as_image()
        .map_err(|e| QuickLookError::PdfRendering(format!("转换图像失败: {}", e)))?;

    dynamic_image
        .write_to(
            &mut std::fs::File::create(&cache_path)
                .map_err(|e| QuickLookError::PdfRendering(format!("创建文件失败: {}", e)))?,
            image::ImageFormat::Png,
        )
        .map_err(|e| QuickLookError::PdfRendering(format!("保存 PNG 失败: {}", e)))?;
    log::info!(
        "[pdf] page {} 保存到: {}",
        page_index + 1,
        cache_path.display()
    );

    Ok(RenderedPage {
        page_num: page_index + 1,
        path: cache_path.to_string_lossy().to_string(),
        width,
        height,
    })
}

pub fn clear_pdf_cache() -> Result<u32, QuickLookError> {
    let cache_dir = std::env::temp_dir().join("quicklook_pdf");
    if !cache_dir.exists() {
        return Ok(0);
    }

    let mut removed = 0u32;
    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if std::fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                }
            }
        }
    }
    Ok(removed)
}
