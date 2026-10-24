use pdfium_render::prelude::{PdfPageRenderRotation, *};
use quicklook_cache::{CacheDir, file_key};
use quicklook_error::QuickLookError;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Mutex, OnceLock};

const CACHE_DIR: &str = "quicklook_pdf";
const MAX_CACHED_DOCS: usize = 2;

pub fn normalize_rotation(rotation: u32) -> u32 {
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

/// 逻辑页尺寸（PDF point，1/72 英寸），不含用户额外旋转
#[derive(Debug, Clone, serde::Serialize)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

fn get_pdfium() -> Result<&'static Pdfium, QuickLookError> {
    static INSTANCE: OnceLock<Result<Pdfium, String>> = OnceLock::new();
    let cached = INSTANCE.get_or_init(|| {
        let dll_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let lib_name = Pdfium::pdfium_platform_library_name_at_path(&dll_dir);

        log::info!("[pdf] dll_dir={}", dll_dir.display());
        log::info!("[pdf] lib_name={}", lib_name.display());

        match Pdfium::bind_to_library(&lib_name) {
            Ok(bindings) => {
                log::info!("[pdf] pdfium 库加载成功");
                Ok(Pdfium::new(bindings))
            },
            Err(e) => {
                log::error!("[pdf] 加载 pdfium 库失败: {}", e);
                Err(format!("加载 pdfium 库失败: {}", e))
            },
        }
    });
    match cached {
        Ok(p) => Ok(p),
        Err(e) => Err(QuickLookError::PdfRendering(e.clone())),
    }
}

struct DocCache {
    path: String,
    doc: PdfDocument<'static>,
}

/// 简单 LRU：`order` 尾部为最近使用，头部为最久未使用。
#[derive(Debug)]
pub struct Lru<K, V> {
    entries: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + Hash + Copy, V> Default for Lru<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Copy, V> Lru<K, V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn contains(&self, key: K) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        if !self.entries.contains_key(&key) {
            return None;
        }
        self.touch(key);
        self.entries.get_mut(&key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.touch(key);
        self.entries.insert(key, value)
    }

    /// 条目数超过 `max` 时淘汰最久未使用的一条。
    pub fn evict_if_over(&mut self, max: usize) -> Option<V> {
        if self.entries.len() <= max {
            return None;
        }
        let oldest = self.order.pop_front()?;
        self.entries.remove(&oldest)
    }

    fn touch(&mut self, key: K) {
        self.order.retain(|k| *k != key);
        self.order.push_back(key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}

fn doc_cache() -> &'static Mutex<Lru<u64, DocCache>> {
    static CACHE: OnceLock<Mutex<Lru<u64, DocCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Lru::new()))
}

/// 在全局锁内访问文档并调用 Pdfium。
///
/// Pdfium 本身不保证线程安全，`thread_safe` feature 仅提供 `Send`/`Sync` 标记，
/// 因此所有 Pdfium 调用必须串行。调用方只应在闭包内完成栅格化，
/// 编码/写盘等耗时操作放到锁外。
fn with_doc<T>(
    path: &str,
    f: impl FnOnce(&PdfDocument<'static>) -> Result<T, QuickLookError>,
) -> Result<T, QuickLookError> {
    let mut cache = doc_cache()
        .lock()
        .map_err(|e| QuickLookError::PdfRendering(format!("锁竞争失败: {}", e)))?;
    let key = file_key(path);

    if !cache.contains(key) {
        let pdfium = get_pdfium()?;
        let doc = pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| QuickLookError::PdfRendering(format!("打开 PDF 失败: {}", e)))?;

        cache.insert(key, DocCache { path: path.to_string(), doc });
        if let Some(evicted) = cache.evict_if_over(MAX_CACHED_DOCS) {
            log::info!("[pdf] 淘汰文档: {}", evicted.path);
            drop(evicted);
        }
        log::info!("[pdf] 已缓存文档: {}", path);
    }

    let entry = cache
        .get_mut(key)
        .ok_or_else(|| QuickLookError::PdfRendering("文档未加载".into()))?;
    f(&entry.doc)
}

/// 读取各页逻辑尺寸（MediaBox，已含页面自身 /Rotate），无需渲染
pub fn get_pdf_page_sizes(path: &str) -> Result<Vec<PageSize>, QuickLookError> {
    log::info!("[pdf] get_pdf_page_sizes path={}", path);
    let sizes = with_doc(path, |doc| {
        let pages = doc.pages();
        let count = pages.len();
        let mut sizes = Vec::with_capacity(count as usize);
        for i in 0..count {
            match pages.page_size(i) {
                Ok(rect) => sizes.push(PageSize {
                    width: rect.width().value.max(1.0),
                    height: rect.height().value.max(1.0),
                }),
                Err(e) => {
                    log::warn!("[pdf] page {} size 失败: {}，回退 Letter", i + 1, e);
                    sizes.push(PageSize { width: 612.0, height: 792.0 });
                },
            }
        }
        Ok(sizes)
    })?;
    log::info!("[pdf] page_sizes={} 页", sizes.len());
    Ok(sizes)
}

pub fn get_pdf_outline(path: &str) -> Result<Vec<OutlineItem>, QuickLookError> {
    log::info!("[pdf] get_pdf_outline path={}", path);

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

    with_doc(path, |doc| {
        let bookmarks = doc.bookmarks();
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
    })
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

    let cache = CacheDir::new(CACHE_DIR);
    cache
        .ensure()
        .map_err(|e| QuickLookError::PdfRendering(format!("创建缓存目录失败: {}", e)))?;

    let normalized_rotation = normalize_rotation(rotation);
    let cache_path = cache.file_path(format!(
        "pdf_{:x}_{}_{}_{}.webp",
        file_key(path),
        page_index,
        dpi,
        normalized_rotation
    ));

    if cache_path.exists()
        && let Ok(img) = image::ImageReader::open(&cache_path)
        && let Ok((w, h)) = img.into_dimensions()
    {
        log::info!("[pdf] page {} 命中缓存", page_index + 1);
        return Ok(RenderedPage {
            page_num: page_index + 1,
            path: cache_path.to_string_lossy().to_string(),
            width: w,
            height: h,
        });
    }

    // 只在栅格化阶段持锁；DynamicImage 已持有像素数据，编码/写盘放到锁外，
    // 避免 WebP 编码期间阻塞其它页渲染请求。
    let (dynamic_image, width, height) = with_doc(path, |doc| {
        let pages = doc.pages();
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
        Ok((dynamic_image, width, height))
    })?;

    // WebP 无损：相对 PNG 显著减小体积且保持文字清晰（image 0.25 仅有损需额外 libwebp）
    let mut file = std::fs::File::create(&cache_path)
        .map_err(|e| QuickLookError::PdfRendering(format!("创建文件失败: {}", e)))?;
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut file);
    dynamic_image
        .write_with_encoder(encoder)
        .map_err(|e| QuickLookError::PdfRendering(format!("保存 WebP 失败: {}", e)))?;
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
    let mut cache = doc_cache()
        .lock()
        .map_err(|e| QuickLookError::PdfRendering(format!("锁竞争失败: {}", e)))?;
    cache.clear();
    drop(cache);

    CacheDir::new(CACHE_DIR).clear()
}
