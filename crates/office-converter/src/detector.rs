use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OfficeApp {
    /// Microsoft Office
    MsOffice,
    /// WPS Office
    Wps,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeInfo {
    pub app: OfficeApp,
    pub version: String,
    pub install_path: PathBuf,
}

/// 检测本地已安装的办公软件
pub fn detect_office_apps() -> Vec<OfficeInfo> {
    let mut apps = Vec::new();

    // 检测 Microsoft Office
    if let Some(office_info) = detect_ms_office() {
        apps.push(office_info);
    }

    // 检测 WPS Office
    if let Some(wps_info) = detect_wps() {
        apps.push(wps_info);
    }

    apps
}

/// 检测 Microsoft Office
fn detect_ms_office() -> Option<OfficeInfo> {
    use winreg::enums::*;
    use winreg::RegKey;

    // 尝试在多个可能的注册表位置查找 Office
    let registry_paths = [
        // Office 2016 及更高版本
        r"SOFTWARE\Microsoft\Office\ClickToRun\Configuration",
        r"SOFTWARE\Microsoft\Office\16.0\Common\InstallRoot",
        r"SOFTWARE\Microsoft\Office\15.0\Common\InstallRoot",
        r"SOFTWARE\Microsoft\Office\14.0\Common\InstallRoot",
        // 64位系统上的32位Office
        r"SOFTWARE\WOW6432Node\Microsoft\Office\16.0\Common\InstallRoot",
        r"SOFTWARE\WOW6432Node\Microsoft\Office\15.0\Common\InstallRoot",
        r"SOFTWARE\WOW6432Node\Microsoft\Office\14.0\Common\InstallRoot",
    ];

    for path in &registry_paths {
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
            let install_path: std::result::Result<String, _> = hklm.get_value("Path");

            if let Ok(path_str) = install_path {
                let install_path = PathBuf::from(&path_str);

                // 检查路径是否存在
                if install_path.exists() {
                    // 尝试获取版本号
                    let version = hklm
                        .get_value::<String, _>("VersionToReport")
                        .or_else(|_| hklm.get_value::<String, _>("Version"))
                        .unwrap_or_else(|_| "Unknown".to_string());

                    return Some(OfficeInfo {
                        app: OfficeApp::MsOffice,
                        version,
                        install_path,
                    });
                }
            }
        }
    }

    None
}

/// 检测 WPS Office
fn detect_wps() -> Option<OfficeInfo> {
    use winreg::enums::*;
    use winreg::RegKey;

    // WPS 可能的注册表路径
    let registry_paths = [
        r"SOFTWARE\Kingsoft\Office\6.0\Common",
        r"SOFTWARE\WPS\Office",
        r"SOFTWARE\WOW6432Node\Kingsoft\Office\6.0\Common",
    ];

    for path in &registry_paths {
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
            let install_path: std::result::Result<String, _> = hklm.get_value("InstallRoot");

            if let Ok(path_str) = install_path {
                let install_path = PathBuf::from(&path_str);

                if install_path.exists() {
                    let version = hklm
                        .get_value::<String, _>("Version")
                        .unwrap_or_else(|_| "Unknown".to_string());

                    return Some(OfficeInfo {
                        app: OfficeApp::Wps,
                        version,
                        install_path,
                    });
                }
            }
        }
    }

    // 尝试在程序文件目录中查找 WPS
    let possible_paths = [
        r"C:\Program Files\Kingsoft\WPS Office",
        r"C:\Program Files (x86)\Kingsoft\WPS Office",
    ];

    for path_str in &possible_paths {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return Some(OfficeInfo {
                app: OfficeApp::Wps,
                version: "Unknown".to_string(),
                install_path: path,
            });
        }
    }

    None
}

/// 检查是否安装了任何办公软件
pub fn is_office_installed() -> bool {
    !detect_office_apps().is_empty()
}

/// 获取首选的办公软件（优先返回 Microsoft Office）
pub fn get_preferred_office() -> Result<OfficeInfo> {
    let apps = detect_office_apps();

    // 优先使用 Microsoft Office
    apps.iter()
        .find(|info| info.app == OfficeApp::MsOffice)
        .or_else(|| apps.first())
        .cloned()
        .ok_or(Error::NoOfficeInstalled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_apps() {
        let apps = detect_office_apps();
        for app in apps {
            println!("{:?}", app);
        }
    }
}
