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

/// 检测当前 Linux 会话是否为 Wayland
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

/// Linux 实现：Wayland 环境优先，X11 作为回退
#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub fn get_monitor_info() -> MonitorInfo {
    if is_wayland() {
        get_monitor_info_wayland()
            .or_else(get_monitor_info_xrandr)
            .unwrap_or(MonitorInfo {
                width: 1920.0,
                height: 1080.0,
                scale: 1.0,
            })
    } else {
        get_monitor_info_xrandr().unwrap_or(MonitorInfo {
            width: 1920.0,
            height: 1080.0,
            scale: 1.0,
        })
    }
}

/// Wayland：通过 wlr-randr（wlroots 合成器：Sway、Hyprland 等）获取显示器信息
#[cfg(target_os = "linux")]
fn get_monitor_info_wayland() -> Option<MonitorInfo> {
    let output = std::process::Command::new("wlr-randr").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_wlr_randr_output(&stdout)
}

/// 解析 wlr-randr 输出，提取当前（current）模式的分辨率
/// 示例行：`  1920x1080 px, 60.000000 Hz (preferred, current)`
#[cfg(target_os = "linux")]
fn parse_wlr_randr_output(output: &str) -> Option<MonitorInfo> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.contains("current") && trimmed.contains("px") {
            if let Some(res) = trimmed.split_whitespace().next() {
                let parts: Vec<&str> = res.splitn(2, 'x').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        return Some(MonitorInfo { width: w, height: h, scale: 1.0 });
                    }
                }
            }
        }
    }
    None
}

/// X11：通过 xrandr 获取主显示器信息
#[cfg(target_os = "linux")]
fn get_monitor_info_xrandr() -> Option<MonitorInfo> {
    use std::process::Command;

    let output = Command::new("xrandr").arg("--current").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);

    // 找到主显示器（包含 " connected primary" 或第一个 " connected"）
    let primary_line = stdout
        .lines()
        .find(|l| l.contains(" connected primary"))
        .or_else(|| stdout.lines().find(|l| l.contains(" connected")))?;

    // 解析类似 "1920x1080+0+0" 的分辨率字段
    let res_token = primary_line.split_whitespace().find(|tok| {
        tok.contains('x') && tok.chars().next().map_or(false, |c| c.is_ascii_digit())
    })?;

    let dims: Vec<&str> = res_token.splitn(2, 'x').collect();
    if dims.len() == 2 {
        let width_str = dims[0];
        let height_str = dims[1].split('+').next().unwrap_or("0");
        if let (Ok(w), Ok(h)) = (width_str.parse::<f64>(), height_str.parse::<f64>()) {
            return Some(MonitorInfo { width: w, height: h, scale: 1.0 });
        }
    }

    None
}

#[allow(dead_code)]
#[cfg(not(any(windows, target_os = "linux")))]
pub fn get_monitor_info() -> MonitorInfo {
    MonitorInfo::default()
}
