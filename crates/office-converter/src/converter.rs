//! Office 文档转换模块

use crate::detector::{get_preferred_office, OfficeApp};
use crate::error::{Error as InternalError, Result};
use crate::ms_office;
use crate::wps_office;
use std::path::{Path, PathBuf};

/// 转换选项
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// 使用的办公软件
    pub office_app: Option<OfficeApp>,
    /// 输出文件路径（如果为 None，则返回 HTML 字符串）
    pub output_path: Option<PathBuf>,
    /// 是否包含样式
    pub include_styles: bool,
    /// 是否包含图片
    pub include_images: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            office_app: None,
            output_path: None,
            include_styles: true,
            include_images: true,
        }
    }
}

/// 将文档转换为 HTML
pub fn convert_to_html<P: AsRef<Path>>(input_path: P, office_app: OfficeApp) -> Result<String> {
    let options = ConvertOptions {
        office_app: Some(office_app),
        ..Default::default()
    };

    convert_to_html_with_options(input_path, options)
}

/// 使用自定义选项将文档转换为 HTML
pub fn convert_to_html_with_options<P: AsRef<Path>>(
    input_path: P,
    options: ConvertOptions,
) -> Result<String> {
    let input_path = input_path.as_ref();

    // 检查文件是否存在
    if !input_path.exists() {
        return Err(InternalError::FileNotFound(
            input_path.display().to_string(),
        ));
    }

    // 确定使用哪个办公软件
    let office_info = if let Some(app) = options.office_app.as_ref() {
        crate::detector::detect_office_apps()
            .into_iter()
            .find(|info| &info.app == app)
            .ok_or(InternalError::NoOfficeInstalled)?
    } else {
        get_preferred_office().map_err(|_| InternalError::NoOfficeInstalled)?
    };

    // 获取文件扩展名
    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| InternalError::UnsupportedFormat("无法识别文件扩展名".to_string()))?
        .to_lowercase();

    // 根据办公软件类型进行转换
    match &office_info.app {
        OfficeApp::MsOffice => ms_office::convert_with_ms_office(input_path, &extension, &options),
        OfficeApp::Wps => wps_office::convert_with_wps(input_path, &extension, &options),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_options_default() {
        let options = ConvertOptions::default();
        assert!(options.include_styles);
        assert!(options.include_images);
        assert!(options.office_app.is_none());
        assert!(options.output_path.is_none());
    }
}
