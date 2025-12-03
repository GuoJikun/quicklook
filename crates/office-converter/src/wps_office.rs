//! WPS Office 文档转换模块

use crate::com_utils::{get_property, invoke_method, set_property, PropertyValue};
use crate::converter::ConvertOptions;
use crate::error::{Error as InternalError, Result};
use std::path::Path;

/// 使用 WPS Office 转换文档
pub(crate) fn convert_with_wps(
    input_path: &Path,
    extension: &str,
    options: &ConvertOptions,
) -> Result<String> {
    use windows::Win32::System::Com::*;

    unsafe {
        // 初始化 COM
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(InternalError::ConversionFailed(format!(
                "Failed to initialize COM: {:?}",
                hr
            )));
        }

        let result = match extension {
            "doc" | "docx" => convert_wps_writer_to_html(input_path, options),
            "xls" | "xlsx" => convert_wps_spreadsheets_to_html(input_path, options),
            "ppt" | "pptx" => convert_wps_presentation_to_html(input_path, options),
            _ => Err(InternalError::UnsupportedFormat(extension.to_string())),
        };

        // 清理 COM
        CoUninitialize();

        result
    }
}

/// 将 WPS 文字文档转换为 HTML
fn convert_wps_writer_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KWps.Application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!(
                "无法找到 WPS 文字 (KWps.Application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"
            ))
        })?;
        let app: IDispatch = CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)?;

        set_property(&app, "Visible", false)?;

        let documents = get_property(&app, "Documents")?;
        let absolute_path = input_path
            .canonicalize()
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        let doc = invoke_method(
            &documents,
            "Open",
            &[PropertyValue::from(
                absolute_path.to_str().unwrap().to_string(),
            )],
        )?;

        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        // WPS Writer 的 SaveAs 方法需要更多参数
        // SaveAs(FileName, FileFormat, LockComments, Password, AddToRecentFiles, ...)
        invoke_method(
            &doc,
            "SaveAs",
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(8),                // wdFormatHTML = 8
                PropertyValue::from(false),            // LockComments
                PropertyValue::String("".to_string()), // Password
                PropertyValue::from(false),            // AddToRecentFiles
            ],
        )?;

        invoke_method(&doc, "Close", &[PropertyValue::from(false)])?;
        invoke_method(&app, "Quit", &[])?;

        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        Ok(html_content)
    }
}

/// 将 WPS 表格转换为 HTML
fn convert_wps_spreadsheets_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KET.application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!(
                "无法找到 WPS 表格 (KET.application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"
            ))
        })?;
        let app: IDispatch = CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)?;

        set_property(&app, "Visible", false)?;
        set_property(&app, "DisplayAlerts", false)?;

        let workbooks = get_property(&app, "Workbooks")?;
        let absolute_path = input_path
            .canonicalize()
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        let workbook = invoke_method(
            &workbooks,
            "Open",
            &[PropertyValue::from(
                absolute_path.to_str().unwrap().to_string(),
            )],
        )?;

        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        // WPS Spreadsheets 保存为 HTML 的格式值为 44
        invoke_method(
            &workbook,
            "SaveAs",
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(44), // xlHtml = 44
            ],
        )?;

        invoke_method(&workbook, "Close", &[PropertyValue::from(false)])?;
        invoke_method(&app, "Quit", &[])?;

        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        Ok(html_content)
    }
}

/// 将 WPS 演示转换为 HTML
fn convert_wps_presentation_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KWPP.application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!(
                "无法找到 WPS 演示 (KWPP.application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"
            ))
        })?;
        let app: IDispatch = CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)?;

        set_property(&app, "Visible", 1)?; // MsoTriState::msoTrue

        let presentations = get_property(&app, "Presentations")?;
        let absolute_path = input_path
            .canonicalize()
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        let presentation = invoke_method(
            &presentations,
            "Open",
            &[
                PropertyValue::from(absolute_path.to_str().unwrap().to_string()),
                PropertyValue::from(-1), // ReadOnly: MsoTriState::msoTrue
                PropertyValue::from(0),  // Untitled: MsoTriState::msoFalse
                PropertyValue::from(0),  // WithWindow: MsoTriState::msoFalse
            ],
        )?;

        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        // WPS Presentation 保存为 HTML 的格式值为 12
        // SaveAs(FileName, FileFormat, EmbedTrueTypeFonts)
        invoke_method(
            &presentation,
            "SaveAs",
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(12), // ppSaveAsHTMLv3 = 12
                PropertyValue::from(0),  // EmbedTrueTypeFonts: msoFalse
            ],
        )?;

        invoke_method(&presentation, "Close", &[])?;
        invoke_method(&app, "Quit", &[])?;

        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        Ok(html_content)
    }
}
