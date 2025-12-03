//! COM 辅助函数和工具类型

use crate::error::Result;
use windows::Win32::System::Com;
use windows::Win32::System::Variant::VARIANT;

/// COM 属性值枚举
pub(crate) enum PropertyValue {
    Bool(bool),
    Int(i32),
    String(String),
}

impl PropertyValue {
    /// 将 PropertyValue 转换为 VARIANT
    pub(crate) fn to_variant(&self) -> VARIANT {
        use windows::core::BSTR;
        use windows::Win32::Foundation::VARIANT_BOOL;
        use windows::Win32::System::Variant::{VT_BOOL, VT_BSTR, VT_I4};

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
