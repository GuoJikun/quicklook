#[cfg(windows)]
use std::ptr::null_mut;

#[cfg(windows)]
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
#[cfg(windows)]
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

/// Linux 实现：通过 xrandr 子进程获取主显示器信息
#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub fn get_monitor_info() -> MonitorInfo {
    use std::process::Command;

    let output = Command::new("xrandr").arg("--current").output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // 找到主显示器（包含 " connected primary" 或第一个 " connected"）
        let primary_line = stdout
            .lines()
            .find(|l| l.contains(" connected primary"))
            .or_else(|| stdout.lines().find(|l| l.contains(" connected")));

        if let Some(line) = primary_line {
            // 解析类似 "1920x1080+0+0" 的分辨率字段
            if let Some(res_token) = line.split_whitespace().find(|tok| {
                tok.contains('x') && tok.chars().next().map_or(false, |c| c.is_ascii_digit())
            }) {
                let dims: Vec<&str> = res_token.splitn(2, 'x').collect();
                if dims.len() == 2 {
                    let width_str = dims[0];
                    let height_str = dims[1].split('+').next().unwrap_or("0");
                    if let (Ok(w), Ok(h)) = (width_str.parse::<f64>(), height_str.parse::<f64>()) {
                        return MonitorInfo { width: w, height: h, scale: 1.0 };
                    }
                }
            }
        }
    }

    // 无法取得时返回合理默认值
    MonitorInfo { width: 1920.0, height: 1080.0, scale: 1.0 }
}

#[allow(dead_code)]
#[cfg(not(any(windows, target_os = "linux")))]
pub fn get_monitor_info() -> MonitorInfo {
    MonitorInfo::default()
}
