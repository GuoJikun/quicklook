use std::ptr::null_mut;

use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, HORZRES, LOGPIXELSX, VERTRES},
};

#[derive(Debug, serde::Serialize, Clone)]
pub struct MonitorInfo {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl Default for MonitorInfo {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
            scale: Default::default(),
        }
    }
}

#[allow(dead_code)]
pub fn get_monitor_info() -> MonitorInfo {
    let hwnd = HWND(null_mut());
    let hdc = unsafe { GetDC(Some(hwnd)) };
    if hdc.is_invalid() {
        return MonitorInfo::default();
    }

    // 获取屏幕分辨率
    let width = unsafe { GetDeviceCaps(Some(hdc), HORZRES) };
    let height = unsafe { GetDeviceCaps(Some(hdc), VERTRES) };

    // 获取缩放比例
    let logical_width = unsafe { GetDeviceCaps(Some(hdc), LOGPIXELSX) };
    let scale_factor = logical_width as f64 / 96.0;

    unsafe { ReleaseDC(Some(hwnd), hdc) };

    MonitorInfo {
        width: width as f64,
        height: height as f64,
        scale: scale_factor,
    }
}
