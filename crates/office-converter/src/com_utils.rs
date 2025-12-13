//! COM 辅助函数和工具类型

use crate::error::Result;
use windows::Win32::System::Com;
use windows::Win32::System::Variant::VARIANT;

/// COM 属性值枚举
pub(crate) enum PropertyValue {
    Bool(bool),
    Int(i32),
    String(String),
    Missing, // 用于表示缺失/可选参数
}

impl PropertyValue {
    /// 将 PropertyValue 转换为 VARIANT
    pub(crate) fn to_variant(&self) -> VARIANT {
        use windows::core::BSTR;
        use windows::Win32::Foundation::VARIANT_BOOL;
        use windows::Win32::System::Variant::{VT_BOOL, VT_BSTR, VT_ERROR, VT_I4};

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
                PropertyValue::Missing => {
                    // DISP_E_PARAMNOTFOUND = 0x80020004
                    (*anon).vt = VT_ERROR;
                    (*anon).Anonymous.scode = 0x80020004_u32 as i32;
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

/// 获取 COM 对象的属性
pub(crate) unsafe fn get_property(object: &Com::IDispatch, name: &str) -> Result<Com::IDispatch> {
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

/// 设置 COM 对象的属性
pub(crate) unsafe fn set_property<T: Into<PropertyValue>>(
    object: &Com::IDispatch,
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
    // DISPATCH_PROPERTYPUT 需要 DISPID_PROPERTYPUT (-3) 作为命名参数
    let mut named_args = [-3]; // DISPID_PROPERTYPUT
    let dispparams = DISPPARAMS {
        rgvarg: &variant as *const _ as *mut _,
        rgdispidNamedArgs: named_args.as_mut_ptr(),
        cArgs: 1,
        cNamedArgs: 1,
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

/// 调用 COM 对象的方法
pub(crate) unsafe fn invoke_method(
    object: &Com::IDispatch,
    name: &str,
    args: &[PropertyValue],
) -> Result<Com::IDispatch> {
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

/// 调用 COM 对象的方法（使用命名参数）
/// 用于 WPS Office 等需要命名参数的 COM 接口
pub(crate) unsafe fn invoke_method_named(
    object: &Com::IDispatch,
    name: &str,
    param_names: &[&str],
    args: &[PropertyValue],
) -> Result<Com::IDispatch> {
    use windows::core::*;
    use windows::Win32::System::Com::*;

    // 构建方法名和参数名的宽字符串
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    // 构建所有名称（方法名 + 参数名）
    let mut all_names_wide: Vec<Vec<u16>> = Vec::with_capacity(1 + param_names.len());
    all_names_wide.push(name_wide);
    for param_name in param_names {
        all_names_wide.push(
            param_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect(),
        );
    }

    let name_ptrs: Vec<PCWSTR> = all_names_wide.iter().map(|n| PCWSTR(n.as_ptr())).collect();

    // 获取所有 DISPID（方法 + 参数）
    let mut dispids: Vec<i32> = vec![0; name_ptrs.len()];

    object.GetIDsOfNames(
        &GUID::zeroed(),
        name_ptrs.as_ptr(),
        name_ptrs.len() as u32,
        0,
        dispids.as_mut_ptr(),
    )?;

    let method_dispid = dispids[0];
    let param_dispids: Vec<i32> = dispids[1..].to_vec();

    // 构建参数（反向顺序）
    let mut variants: Vec<VARIANT> = args.iter().map(|arg| arg.to_variant()).collect();
    variants.reverse();

    // 命名参数的 DISPID 也需要反向
    let mut named_dispids: Vec<i32> = param_dispids.clone();
    named_dispids.reverse();

    let dispparams = DISPPARAMS {
        rgvarg: variants.as_mut_ptr(),
        rgdispidNamedArgs: named_dispids.as_mut_ptr(),
        cArgs: variants.len() as u32,
        cNamedArgs: named_dispids.len() as u32,
    };

    let mut result = VARIANT::default();

    object.Invoke(
        method_dispid,
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
