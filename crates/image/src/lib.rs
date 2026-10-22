use std::path::PathBuf;

use quicklook_cache::{CacheDir, file_key};
use quicklook_error::QuickLookError;

const CACHE_DIR: &str = "quicklook_images";

/// 将 PSD、HEIC/HEIF、JXL 等图片转换为 PNG 并缓存，返回缓存文件路径。
pub fn convert_to_png(path: &str) -> Result<String, QuickLookError> {
    let cache = CacheDir::new(CACHE_DIR);
    cache.ensure()?;
    let cache_path = cache.file_path(format!("quicklook_{:x}.png", file_key(path)));

    if cache_path.exists() {
        log::info!("命中图片缓存: {:?}", cache_path);
        return Ok(cache_path.to_string_lossy().into_owned());
    }

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "psd" => psd_to_png(path, &cache_path)?,
        "heic" | "heif" => heic_to_png(path, &cache_path)?,
        "jxl" => jxl_to_png(path, &cache_path)?,
        _ => image_to_png(path, &cache_path)?,
    }

    Ok(cache_path.to_string_lossy().into_owned())
}

/// 清理图片转码缓存，返回删除的缓存条目数。
pub fn clear_image_cache() -> Result<u32, QuickLookError> {
    let removed = CacheDir::new(CACHE_DIR).clear()?;
    log::info!("共清理 {} 个图片缓存条目", removed);
    Ok(removed)
}

pub fn psd_to_png(path: &str, temp_path: &PathBuf) -> Result<(), QuickLookError> {
    let file = std::fs::File::open(path)
        .map_err(|e| QuickLookError::ImageProcessing(format!("psd: 打开文件失败: {}", e)))?;
    // 使用内存映射读取，避免将大 PSD 文件整体读入堆内存（按需分页加载）
    let mmap = unsafe { memmap2::Mmap::map(&file) }
        .map_err(|e| QuickLookError::ImageProcessing(format!("psd: 映射文件失败: {}", e)))?;
    let psd_obj = psd::Psd::from_bytes(&mmap)
        .map_err(|e| QuickLookError::ImageProcessing(format!("psd: 解析失败: {}", e)))?;
    let rgba = psd_obj.rgba();
    let width = psd_obj.width();
    let height = psd_obj.height();
    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| QuickLookError::ImageProcessing("psd: 构建 RgbaImage 失败".to_string()))?;
    img.save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| QuickLookError::ImageProcessing(e.to_string()))
}

pub fn heic_to_png(path: &str, temp_path: &PathBuf) -> Result<(), QuickLookError> {
    let options = heic_rs::DecodeOptions {
        layout: heic_rs::PixelLayout::Rgba8,
        ..Default::default()
    };
    let decoded = heic_rs::io::decode_file(path, &options)
        .map_err(|e| QuickLookError::ImageProcessing(format!("heic: 解码失败: {}", e)))?;
    let img = image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.data)
        .ok_or_else(|| QuickLookError::ImageProcessing("heic: 构建 RgbaImage 失败".to_string()))?;
    img.save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| QuickLookError::ImageProcessing(e.to_string()))
}

pub fn jxl_to_png(path: &str, temp_path: &PathBuf) -> Result<(), QuickLookError> {
    use jxl_oxide::integration::JxlDecoder;

    let file = std::fs::File::open(path)
        .map_err(|e| QuickLookError::ImageProcessing(format!("jxl: 打开文件失败: {}", e)))?;
    let decoder = JxlDecoder::new(file)
        .map_err(|e| QuickLookError::ImageProcessing(format!("jxl: 创建解码器失败: {}", e)))?;
    let img = image::DynamicImage::from_decoder(decoder)
        .map_err(|e| QuickLookError::ImageProcessing(format!("jxl: 解码失败: {}", e)))?;
    img.to_rgba8()
        .save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| QuickLookError::ImageProcessing(e.to_string()))
}

pub fn image_to_png(path: &str, temp_path: &PathBuf) -> Result<(), QuickLookError> {
    let img = image::open(path)
        .map_err(|e| QuickLookError::ImageProcessing(format!("image: 读取图片失败: {}", e)))?;
    img.to_rgba8()
        .save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| QuickLookError::ImageProcessing(e.to_string()))
}
