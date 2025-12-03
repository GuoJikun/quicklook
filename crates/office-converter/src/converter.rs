use crate::detector::{get_preferred_office, OfficeApp};
use std::path::{Path, PathBuf};
#[path = "./error.rs"]
mod error;
use error::{Error as InternalError, Result};
use windows::Win32::System::Com;
use windows::Win32::System::Variant::VARIANT;
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
        OfficeApp::MsOffice => convert_with_ms_office(input_path, &extension, &options),
        OfficeApp::Wps => convert_with_wps(input_path, &extension, &options),
    }
}

fn convert_with_ms_office(
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
        )?;

        // 保存为 HTML
        let output_path = options.output_path.clone().unwrap_or_else(|| {
            let mut path = absolute_path.clone();
            path.set_extension("html");
            path
        });
        invoke_method(
            &doc,
            "SaveAs2",
            &[
                PropertyValue::from(output_path.to_str().unwrap().to_string()),
                PropertyValue::from(8), // wdFormatHTML = 8
            ],
        )?;

        // 关闭文档
        invoke_method(&doc, "Close", &[PropertyValue::from(false)])?;

        // 退出 Word
        invoke_method(&word_app, "Quit", &[])?;

        // 读取生成的 HTML 文件
        let html_content = std::fs::read_to_string(&output_path)
            .map_err(|e| InternalError::ConversionFailed(e.to_string()))?;

        Ok(html_content)
    }
}

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
        invoke_method(
            &workbook,
            "SaveAs",
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

        set_property(&ppt_app, "Visible", 1)?;

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
        invoke_method(
            &presentation,
            "SaveAs",
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

// COM 辅助函数
unsafe fn get_property(
    object: &windows::Win32::System::Com::IDispatch,
    name: &str,
) -> Result<windows::Win32::System::Com::IDispatch> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut dispid = 0;

    object.GetIDsOfNames(
        &GUID::zeroed(),
        &PCWSTR(name_wide.as_ptr()),
        1,
        0,
        &mut dispid,
    )?;

    let mut result = VARIANT::default();
    let dispparams = DISPPARAMS::default();

    object.Invoke(
        dispid,
        &GUID::zeroed(),
        0,
        DISPATCH_PROPERTYGET,
        &dispparams,
        Some(&mut result),
        None,
        None,
    )?;

    // 从 VARIANT 中提取 IDispatch
    Ok(<std::option::Option<Com::IDispatch> as Clone>::clone(
        &result.Anonymous.Anonymous.Anonymous.pdispVal,
    )
    .unwrap())
}

unsafe fn set_property<T: Into<PropertyValue>>(
    object: &windows::Win32::System::Com::IDispatch,
    name: &str,
    value: T,
) -> Result<()> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut dispid = 0;

    object.GetIDsOfNames(
        &GUID::zeroed(),
        &PCWSTR(name_wide.as_ptr()),
        1,
        0,
        &mut dispid,
    )?;

    let variant = value.into().to_variant();
    let dispparams = DISPPARAMS {
        rgvarg: &variant as *const _ as *mut _,
        cArgs: 1,
        ..Default::default()
    };

    object.Invoke(
        dispid,
        &GUID::zeroed(),
        0,
        DISPATCH_PROPERTYPUT,
        &dispparams,
        None,
        None,
        None,
    )?;

    Ok(())
}

unsafe fn invoke_method(
    object: &windows::Win32::System::Com::IDispatch,
    name: &str,
    args: &[PropertyValue],
) -> Result<windows::Win32::System::Com::IDispatch> {
    use windows::core::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::System::Variant::VARIANT;

    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut dispid = 0;

    object.GetIDsOfNames(
        &GUID::zeroed(),
        &PCWSTR(name_wide.as_ptr()),
        1,
        0,
        &mut dispid,
    )?;

    let mut variants: Vec<VARIANT> = args.iter().map(|arg| arg.to_variant()).collect();
    variants.reverse();
    let dispparams = DISPPARAMS {
        rgvarg: variants.as_mut_ptr(),
        cArgs: variants.len() as u32,
        ..Default::default()
    };

    let mut result = VARIANT::default();

    object.Invoke(
        dispid,
        &GUID::zeroed(),
        0,
        DISPATCH_METHOD,
        &dispparams,
        Some(&mut result),
        None,
        None,
    )?;

    // 返回结果（如果是对象）
    if result.Anonymous.Anonymous.vt == windows::Win32::System::Variant::VARENUM(9) {
        // VT_DISPATCH
        Ok(<std::option::Option<Com::IDispatch> as Clone>::clone(
            &result.Anonymous.Anonymous.Anonymous.pdispVal,
        )
        .unwrap())
    } else {
        // 对于没有返回值的方法，返回原对象
        Ok(object.clone())
    }
}

enum PropertyValue {
    Bool(bool),
    Int(i32),
    String(String),
}

impl PropertyValue {
    fn to_variant(&self) -> VARIANT {
        use windows::core::BSTR;
        use windows::Win32::Foundation::VARIANT_BOOL;
        use windows::Win32::System::Variant::{VARIANT, VT_BOOL, VT_BSTR, VT_I4};

        let mut variant = VARIANT::default();
        unsafe {
            let anon = &mut variant.Anonymous.Anonymous;
            match self {
                PropertyValue::Bool(b) => {
                    (*anon).vt = VT_BOOL;
                    (*anon).Anonymous.boolVal = VARIANT_BOOL(if *b { -1 } else { 0 });
                },
                PropertyValue::Int(i) => {
                    (*anon).vt = VT_I4;
                    (*anon).Anonymous.lVal = *i;
                },
                PropertyValue::String(s) => {
                    let bstr = BSTR::from(s);
                    (*anon).vt = VT_BSTR;
                    (*anon).Anonymous.bstrVal = std::mem::ManuallyDrop::new(bstr);
                },
            }
        }
        variant
    }
}

impl From<bool> for PropertyValue {
    fn from(b: bool) -> Self {
        PropertyValue::Bool(b)
    }
}

impl From<i32> for PropertyValue {
    fn from(i: i32) -> Self {
        PropertyValue::Int(i)
    }
}

impl From<String> for PropertyValue {
    fn from(s: String) -> Self {
        PropertyValue::String(s)
    }
}

fn convert_with_wps(
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

fn convert_wps_writer_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KWps.Application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!("无法找到 WPS 文字 (KWps.Application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"))
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

fn convert_wps_spreadsheets_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KET.application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!("无法找到 WPS 表格 (KET.application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"))
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

fn convert_wps_presentation_to_html(input_path: &Path, options: &ConvertOptions) -> Result<String> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let prog_id = w!("KWPP.application");
        let clsid = CLSIDFromProgID(prog_id).map_err(|_| {
            InternalError::ConversionFailed(format!("无法找到 WPS 演示 (KWPP.application) 的 COM 组件。请检查 WPS Office 是否已正确安装。"))
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
