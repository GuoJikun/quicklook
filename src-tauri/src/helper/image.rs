use std::path::PathBuf;

pub fn psd_to_png(path: &str, temp_path: &PathBuf) -> Result<(), String> {
    let file_bytes =
        std::fs::read(path).map_err(|e| format!("psd: 读取文件失败: {}", e))?;
    let psd_obj =
        psd::Psd::from_bytes(&file_bytes).map_err(|e| format!("psd: 解析失败: {}", e))?;
    let rgba = psd_obj.rgba();
    let width = psd_obj.width();
    let height = psd_obj.height();
    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| "psd: 构建 RgbaImage 失败".to_string())?;
    img.save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}

pub fn heic_to_png(path: &str, temp_path: &PathBuf) -> Result<(), String> {
    libheif_rs::integration::image::register_all_decoding_hooks();
    let img =
        image::open(path).map_err(|e| format!("heic: 读取图片失败: {}", e))?;
    img.to_rgba8()
        .save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}

pub fn image_to_png(path: &str, temp_path: &PathBuf) -> Result<(), String> {
    let img =
        image::open(path).map_err(|e| format!("image: 读取图片失败: {}", e))?;
    img.to_rgba8()
        .save_with_format(temp_path, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
}
