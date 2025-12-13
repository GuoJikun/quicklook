//! Microsoft Office 文档转换模块

use crate::com_utils::{
    get_property, invoke_method, invoke_method_named, set_property, PropertyValue,
};
use crate::converter::ConvertOptions;
use crate::error::{Error as InternalError, Result};
use std::path::Path;

/// 使用 Microsoft Office 转换文档
pub(crate) fn convert_with_ms_office(
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
            "doc" | "docx" => convert_word_to_html(input_path, options),
            "xls" | "xlsx" => convert_excel_to_html(input_path, options),
            "ppt" | "pptx" => convert_powerpoint_to_html(input_path, options),
            _ => Err(InternalError::UnsupportedFormat(extension.to_string())),
        };

        // 清理 COM
        CoUninitialize();

        result
    }
}

/// 将 Word 文档转换为 HTML
fn convert_word_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        // 创建 Word 应用程序实例
        let word_app: IDispatch = CoCreateInstance(
            &GUID::from_u128(0x000209FF_0000_0000_C000_000000000046), // Word.Application CLSID
            None,
            CLSCTX_LOCAL_SERVER,
        )?;

        // 设置 Visible = False
        set_property(&word_app, "Visible", false)?;
        // 设置 DisplayAlerts = wdAlertsNone (0)
        set_property(&word_app, "DisplayAlerts", 0)?;

        println!("Debug: Attempting to open document...");
        // 打开文档
        let documents = get_property(&word_app, "Documents")?;
        let absolute_path = input_path
            .canonicalize()
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;
        let doc = invoke_method(
            &documents,
            "Open",
            &[PropertyValue::from(
                absolute_path.to_str().unwrap().to_string(),
            )],
        );

        if let Err(e) = &doc {
            println!("Debug: Failed to open document: {:?}", e);
            invoke_method(&word_app, "Quit", &[])?;
            return Err(InternalError::ConversionFailed(format!(
                "Failed to open document: {:?}",
                e
            )));
        }
        let doc = doc?;
        println!("Debug: Document opened successfully.");

        // 保存为 HTML
        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        let output_path_str = output_path.to_str().unwrap().to_string();
        let output_path_str = if output_path_str.starts_with("\\\\?\\") {
            output_path_str[4..].to_string()
        } else {
            output_path_str
        };

        println!("Debug: Attempting to save as HTML to: {}", output_path_str);
        // 使用命名参数调用 SaveAs2
        let save_result = invoke_method_named(
            &doc,
            "SaveAs2",
            &["FileName", "FileFormat"],
            &[
                PropertyValue::from(output_path_str),
                PropertyValue::from(8), // wdFormatHTML = 8
            ],
        );

        if let Err(e) = &save_result {
            println!("Debug: Failed to save document: {:?}", e);
            // 如果文件已存在，可能是 SaveAs 返回了错误但实际上保存成功了
            // 或者是因为文件被占用等原因
            if output_path.exists() {
                println!("Debug: Output file exists, ignoring error.");
            } else {
                return Err(InternalError::ConversionFailed(format!(
                    "Failed to save document: {:?}",
                    e
                )));
            }
        } else {
            println!("Debug: Document saved successfully.");
        }
        // save_result?; // Don't propagate error if we handled it above

        // 关闭文档
        invoke_method(&doc, "Close", &[PropertyValue::from(false)])?;

        // 退出 Word
        invoke_method(&word_app, "Quit", &[])?;

        // 读取生成的 HTML 文件
        // Word 保存的 HTML 可能是 GBK 或其他编码，尝试使用 encoding_rs 处理
        let bytes = std::fs::read(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;

        let html_content = match String::from_utf8(bytes.clone()) {
            Ok(s) => s,
            Err(_) => {
                // 尝试 GBK 解码
                let (cow, _, _) = encoding_rs::GBK.decode(&bytes);
                cow.into_owned()
            },
        };

        Ok(html_content)
    }
}

/// 将 Excel 工作簿转换为 HTML
fn convert_excel_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        // 创建 Excel 应用程序实例
        let excel_app: IDispatch = CoCreateInstance(
            &GUID::from_u128(0x00024500_0000_0000_C000_000000000046), // Excel.Application CLSID
            None,
            CLSCTX_LOCAL_SERVER,
        )?;

        set_property(&excel_app, "Visible", false)?;
        set_property(&excel_app, "DisplayAlerts", false)?;

        // 打开工作簿
        let workbooks = get_property(&excel_app, "Workbooks")?;
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

        // 保存为 HTML
        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        // 使用命名参数调用 SaveAs
        invoke_method_named(
            &workbook,
            "SaveAs",
            &["Filename", "FileFormat"],
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(44), // xlHtml = 44
            ],
        )?;

        // 关闭工作簿
        invoke_method(&workbook, "Close", &[PropertyValue::from(false)])?;
        // 退出 Excel
        invoke_method(&excel_app, "Quit", &[])?;

        // 读取生成的 HTML 文件
        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;

        Ok(html_content)
    }
}

/// 将 PowerPoint 演示文稿转换为 HTML
fn convert_powerpoint_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        // 创建 PowerPoint 应用程序实例
        let ppt_app: IDispatch = CoCreateInstance(
            &GUID::from_u128(0x91493441_5A91_11CF_8700_00AA0060263B), // PowerPoint.Application CLSID
            None,
            CLSCTX_LOCAL_SERVER,
        )?;

        set_property(&ppt_app, "Visible", 1)?; // MsoTriState::msoTrue

        let presentations = get_property(&ppt_app, "Presentations")?;
        let absolute_path = input_path
            .canonicalize()
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;

        // 打开演示文稿
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

        // 保存为 HTML
        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });

        // PowerPoint SaveAs 需要完整的参数
        // SaveAs(FileName, FileFormat, EmbedTrueTypeFonts)
        // 使用命名参数
        invoke_method_named(
            &presentation,
            "SaveAs",
            &["FileName", "FileFormat", "EmbedTrueTypeFonts"],
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(12), // ppSaveAsHTMLv3 = 12
                PropertyValue::from(0),  // EmbedTrueTypeFonts: msoFalse
            ],
        )?;

        // 关闭演示文稿
        invoke_method(&presentation, "Close", &[])?;
        invoke_method(&ppt_app, "Quit", &[])?;

        // 读取生成的 HTML 文件
        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;

        Ok(html_content)
    }
}
