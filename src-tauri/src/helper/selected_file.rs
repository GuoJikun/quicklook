// ============================================================
// Windows-specific implementation
// ============================================================
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use windows::{
    core::{w, Error as WError, Interface, BOOL, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM},
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, IServiceProvider,
                CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
            },
            SystemServices::SFGAO_FILESYSTEM,
            Variant::{self, VARIANT},
        },
        UI::{
            Accessibility::{
                CUIAutomation, IUIAutomation, IUIAutomationSelectionPattern, UIA_NamePropertyId,
                UIA_SelectionPatternId,
            },
            Shell::{
                FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Libraries,
                FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos, IShellBrowser, IShellItem,
                IShellItemArray, IShellView, IShellWindows, SHCreateItemFromParsingName,
                SHGetKnownFolderPath, ShellWindows, KF_FLAG_DEFAULT, SIGDN_DESKTOPABSOLUTEPARSING,
                SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH,
            },
            WindowsAndMessaging,
        },
    },
};

#[cfg(windows)]
use crate::helper::win;

#[cfg(windows)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum FwWindowType {
    Explorer,
    Desktop,
    Dialog,
}

#[allow(dead_code)]
pub struct Selected;

#[cfg(windows)]
#[allow(dead_code)]
impl Selected {
    pub fn new() -> Result<String, WError> {
        match Self::get_selected_file() {
            Ok(path) => Ok(path),
            Err(e) => {
                log::error!("Error: {:?}", e);
                Err(e)
            },
        }
    }

    fn get_selected_file() -> Result<String, WError> {
        if let Some(fw_window_type) = Self::get_focused_type() {
            match fw_window_type {
                FwWindowType::Explorer => unsafe { Self::get_selected_file_from_explorer() },
                FwWindowType::Desktop => unsafe { Self::get_selected_file_from_desktop() },
                FwWindowType::Dialog => Self::get_selected_file_from_dialog(),
            }
        } else {
            Err(WError::from_win32())
        }
    }
    pub fn get_focused_type() -> Option<FwWindowType> {
        let mut type_str: Option<FwWindowType> = None;
        let hwnd_gfw = unsafe { WindowsAndMessaging::GetForegroundWindow() };
        let class_name = win::get_window_class_name(hwnd_gfw);
        log::info!("class_name: {}", class_name);

        if class_name.contains("CabinetWClass") {
            type_str = Some(FwWindowType::Explorer);
        } else if class_name.contains("Progman") || class_name.contains("WorkerW") {
            let defview = unsafe {
                WindowsAndMessaging::FindWindowExW(
                    Some(hwnd_gfw),
                    None,
                    w!("SHELLDLL_DefView"),
                    None,
                )
            };
            if defview.is_ok() {
                type_str = Some(FwWindowType::Desktop);
            }
        } else if class_name.contains("#32770") {
            type_str = Some(FwWindowType::Dialog);
        }
        log::info!("type_str: {:?}", type_str);
        type_str
    }

    unsafe fn get_selected_file_from_explorer() -> Result<String, WError> {
        let (tx, rx) = mpsc::channel();

        // 在新的线程中执行 COM 操作
        thread::spawn(move || {
            let result: Result<String, WError> = (|| -> Result<String, WError> {
                // 在子线程中初始化 COM 库为单线程单元
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

                let hwnd_gfw = WindowsAndMessaging::GetForegroundWindow();
                let shell_windows: IShellWindows =
                    CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
                let result_hwnd = WindowsAndMessaging::FindWindowExW(
                    Some(hwnd_gfw),
                    None,
                    w!("ShellTabWindowClass"),
                    None,
                )?;

                let mut target_path = String::new();
                let count = shell_windows.Count().unwrap_or_default();

                for i in 0..count {
                    let variant = VARIANT::from(i);
                    let dispatch: IDispatch = shell_windows.Item(&variant)?;

                    let shell_browser = Self::dispath2browser(dispatch);

                    if shell_browser.is_none() {
                        continue;
                    }
                    let shell_browser = shell_browser.unwrap();
                    // 调用 GetWindow 可能会阻塞 GUI 消息
                    let phwnd = shell_browser.GetWindow()?;
                    if hwnd_gfw.0 != phwnd.0 && result_hwnd.0 != phwnd.0 {
                        continue;
                    }

                    if win::is_cursor_activated(HWND::default()) {
                        continue;
                    };

                    let shell_view = shell_browser.QueryActiveShellView().unwrap();
                    target_path = Self::get_selected_file_path_from_shellview(shell_view);
                }

                Ok(target_path)
            })();
            tx.send(result).unwrap();
        });
        let target_path = rx.recv().unwrap()?;

        Ok(target_path)
    }

    unsafe fn get_selected_file_from_desktop() -> Result<String, WError> {
        let (tx, rx) = mpsc::channel();

        // 在新的线程中执行 COM 操作
        thread::spawn(move || {
            let result: Result<String, WError> = (|| -> Result<String, WError> {
                // 初始化 COM 库
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

                let mut target_path = String::new();
                let hwnd_gfw = WindowsAndMessaging::GetForegroundWindow(); // 获取当前活动窗口句柄
                log::info!("hwnd_gfw: {:?}", hwnd_gfw);
                let shell_windows: Result<IShellWindows, WError> =
                    CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER);
                if shell_windows.is_err() {
                    log::info!("shell_windows 不存在");
                    return Ok(target_path);
                }
                let shell_windows = shell_windows?;

                let pvar_loc: VARIANT = Variant::VariantInit();

                // 获取活动窗口
                let mut phwnd: i32 = 0;

                let dispatch = shell_windows.FindWindowSW(
                    &pvar_loc,
                    &pvar_loc,
                    SWC_DESKTOP,
                    &mut phwnd,
                    SWFO_NEEDDISPATCH,
                )?;

                if win::is_cursor_activated(HWND(phwnd as *mut _)) {
                    log::info!("存在激活的鼠标");
                    return Ok(target_path);
                };

                let shell_browser = Self::dispath2browser(dispatch);
                if shell_browser.is_none() {
                    log::info!("shell_browser 不存在");
                    return Ok(target_path);
                }

                let shell_browser = shell_browser.unwrap();

                let shell_view = shell_browser.QueryActiveShellView()?;

                target_path = Self::get_selected_file_path_from_shellview(shell_view);

                Ok(target_path)
            })();
            tx.send(result).unwrap();
        });

        let target_path = rx.recv().unwrap()?;
        Ok(target_path)
    }

    fn get_selected_file_from_dialog() -> Result<String, WError> {
        let mut target_path = String::new();
        let fw_hwnd = unsafe { WindowsAndMessaging::GetForegroundWindow() };
        println!("fw_hwnd: {:?}", fw_hwnd);

        let defview = unsafe {
            let mut tmp: Option<HWND> = None;
            let _ = WindowsAndMessaging::EnumChildWindows(
                Some(fw_hwnd),
                Some(Self::dialog_defview_proc),
                LPARAM(&mut tmp as *mut _ as isize),
            );
            tmp
        };

        if defview.is_none() {
            log::info!("defview 不存在");
            return Ok(target_path);
        }
        // let defview = defview.unwrap();

        if win::is_cursor_activated(HWND::default()) {
            return Ok(target_path);
        };

        let listview =
            unsafe { WindowsAndMessaging::FindWindowExW(defview, None, w!("DirectUIHWND"), None) };
        if listview.is_err() {
            log::info!("listview(DirectUIHWND) 不存在");
            return Ok(target_path);
        }
        let listview = listview?;
        let seleced_file_title = unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            // 通过 ui automation 获取选中文件
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            // 获取列表元素
            let list_element = automation.ElementFromHandle(listview)?;

            // 获取选中项
            let selection_pattern = list_element.GetCurrentPattern(UIA_SelectionPatternId)?;
            let selection = selection_pattern.cast::<IUIAutomationSelectionPattern>()?;

            // 获取选中的元素
            let selected = selection.GetCurrentSelection()?;
            let count = selected.Length()?;
            let mut file_name = String::new();
            if count > 0 {
                // 获取第一个选中项
                let item = selected.GetElement(0)?;
                // 获取文件名
                let name = item.GetCurrentPropertyValue(UIA_NamePropertyId)?;
                file_name = name.to_string();
            }
            file_name
        };
        println!("seleced_file_title: {:?}", seleced_file_title);

        // 获取搜索框的 Text
        let mut breadcrumb_parent_hwnd: Option<HWND> = None;
        let _ = unsafe {
            WindowsAndMessaging::EnumChildWindows(
                Some(fw_hwnd),
                Some(Self::breadcrumb_proc),
                LPARAM(&mut breadcrumb_parent_hwnd as *const _ as isize),
            )
        };
        if breadcrumb_parent_hwnd.is_none() {
            return Ok(target_path);
        }
        // let breadcrumb_parent_hwnd = breadcrumb_parent_hwnd.unwrap();
        let breadcrumb_hwnd = unsafe {
            WindowsAndMessaging::FindWindowExW(
                breadcrumb_parent_hwnd,
                None,
                w!("ToolbarWindow32"),
                None,
            )
        };
        if breadcrumb_hwnd.is_err() {
            return Ok(target_path);
        }
        let breadcrumb_hwnd = breadcrumb_hwnd.unwrap();
        let mut breadcrumb_title = win::get_window_text(breadcrumb_hwnd);
        log::info!("弹窗目录: {:?}", breadcrumb_title);
        let arr = breadcrumb_title
            .split(": ")
            .map(|item| item.to_string())
            .collect::<Vec<String>>();
        if arr.len() > 1 {
            breadcrumb_title = arr[1].clone();
        }

        if !breadcrumb_title.contains(":\\") {
            let path = Self::get_library_path(&breadcrumb_title);
            log::error!("path: {:?}", path);
            if path.is_err() {
                return Ok(target_path);
            }
            breadcrumb_title = path.unwrap();
        }

        target_path = format!("{}\\{}", breadcrumb_title, seleced_file_title);
        println!("target_path: {:?}", target_path);

        Ok(target_path)
    }
    fn get_library_path(name: &str) -> Result<String, WError> {
        unsafe {
            // 1. 获取库文件夹路径
            let folder_id = match name {
                "下载" | "Downloads" => &FOLDERID_Downloads,
                "音乐" | "Music" => &FOLDERID_Music,
                "图片" | "Pictures" => &FOLDERID_Pictures,
                "文档" | "Documents" => &FOLDERID_Documents,
                "视频" | "Videos" => &FOLDERID_Videos,
                "桌面" | "Desktop" => &FOLDERID_Desktop,
                _ => {
                    // 如果是自定义库，尝试从Libraries文件夹读取
                    let libraries_path =
                        SHGetKnownFolderPath(&FOLDERID_Libraries, KF_FLAG_DEFAULT, None)?;

                    let lib_file = format!("{}\\{}.library-ms", libraries_path.to_string()?, name);
                    let shell_item: IShellItem =
                        SHCreateItemFromParsingName(&HSTRING::from(lib_file), None)?;

                    return Ok(shell_item.GetDisplayName(SIGDN_FILESYSPATH)?.to_string()?);
                },
            };

            println!("libraries_path: {:?}", folder_id);

            let path = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None)?;
            Ok(path.to_string()?)
        }
    }

    unsafe extern "system" fn dialog_defview_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list_view = lparam.0 as *mut Option<HWND>;
        let class_name = win::get_window_class_name(hwnd);
        if class_name.contains("SHELLDLL_DefView") {
            *list_view = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }
    unsafe extern "system" fn breadcrumb_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list_view = lparam.0 as *mut Option<HWND>;
        let class_name = win::get_window_class_name(hwnd);
        if class_name.contains("Breadcrumb Parent") {
            *list_view = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    unsafe fn dispath2browser(dispatch: IDispatch) -> Option<IShellBrowser> {
        let mut service_provider: Option<IServiceProvider> = None;
        dispatch
            .query(
                &IServiceProvider::IID,
                &mut service_provider as *mut _ as *mut _,
            )
            .ok()
            .unwrap();
        if service_provider.is_none() {
            return None;
        }
        let shell_browser = service_provider
            .unwrap()
            .QueryService::<IShellBrowser>(&IShellBrowser::IID)
            .ok();
        shell_browser
    }

    unsafe fn get_selected_file_path_from_shellview(shell_view: IShellView) -> String {
        let mut target_path = String::new();
        let shell_items = shell_view.GetItemObject::<IShellItemArray>(SVGIO_SELECTION);

        if shell_items.is_err() {
            return target_path;
        }
        println!("shell_items: {:?}", shell_items);
        let shell_items = shell_items.unwrap();
        let count = shell_items.GetCount().unwrap_or_default();
        for i in 0..count {
            let shell_item = shell_items.GetItemAt(i).unwrap();

            // 如果不是文件对象则继续循环
            if let Ok(attrs) = shell_item.GetAttributes(SFGAO_FILESYSTEM) {
                log::info!("attrs: {:?}", attrs);
                if attrs.0 == 0 {
                    continue;
                }
            }

            if let Ok(display_name) = shell_item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) {
                let tmp = display_name.to_string();
                if tmp.is_err() {
                    continue;
                }
                target_path = tmp.unwrap();
                break;
            }

            if let Ok(display_name) = shell_item.GetDisplayName(SIGDN_FILESYSPATH) {
                println!("display_name: {:?}", display_name);
                let tmp = display_name.to_string();
                if tmp.is_err() {
                    println!("display_name error: {:?}", tmp.err());
                    continue;
                }
                target_path = tmp.unwrap();
                break;
            }
        }
        target_path
    }
}

#[cfg(windows)]
impl Drop for Selected {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}

// ============================================================
// Linux implementation
// ============================================================

/// 获取当前活动窗口的 WM_CLASS（通过 xdotool）
#[cfg(target_os = "linux")]
fn get_active_window_class() -> Option<String> {
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// 从剪贴板 URI 列表中解析第一个本地文件路径
#[cfg(target_os = "linux")]
fn parse_uri_list(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(encoded) = line.strip_prefix("file://") {
            let path = urlencoding::decode(encoded)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| encoded.to_string());
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

/// 使用 xclip 或 xsel 读取剪贴板中的 URI 列表
#[cfg(target_os = "linux")]
fn read_clipboard_uri_list() -> Option<String> {
    // 优先使用 xclip
    if let Ok(output) = std::process::Command::new("xclip")
        .args(["-o", "-selection", "clipboard", "-t", "text/uri-list"])
        .output()
    {
        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout);
            if let Some(path) = parse_uri_list(&content) {
                return Some(path);
            }
        }
    }

    // 回退到 xsel（读取 UTF-8 文本，Nautilus 通常写入 URI 格式）
    if let Ok(output) = std::process::Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output()
    {
        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout);
            if let Some(path) = parse_uri_list(&content) {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
impl Selected {
    /// 获取当前文件管理器中选中的文件路径
    ///
    /// 实现原理：向活动窗口发送 Ctrl+C（xdotool），然后从剪贴板读取 URI 列表。
    /// 需要安装 xdotool 和 xclip（或 xsel）：
    ///   `sudo apt install xdotool xclip`
    pub fn new() -> Result<String, String> {
        Self::get_selected_type().ok_or_else(|| "当前窗口不是支持的文件管理器".to_string())?;
        Self::get_selected_file()
    }

    /// 检查当前活动窗口是否为受支持的文件管理器，是则返回 Some(())
    pub fn get_focused_type() -> Option<()> {
        Self::get_selected_type()
    }

    fn get_selected_type() -> Option<()> {
        let class_name = get_active_window_class()?;
        let lower = class_name.to_lowercase();
        // 支持 Nautilus、Nemo、Thunar、Dolphin 等常见文件管理器
        if lower.contains("nautilus")
            || lower.contains("nemo")
            || lower.contains("thunar")
            || lower.contains("dolphin")
            || lower.contains("pcmanfm")
            || lower.contains("caja")
        {
            Some(())
        } else {
            None
        }
    }

    fn get_selected_file() -> Result<String, String> {
        // 向活动文件管理器窗口发送 Ctrl+C，将选中文件写入剪贴板
        let status = std::process::Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+c"])
            .status()
            .map_err(|e| format!("xdotool 执行失败: {}", e))?;

        if !status.success() {
            return Err("xdotool key 命令失败".to_string());
        }

        // 等待剪贴板更新
        std::thread::sleep(std::time::Duration::from_millis(150));

        read_clipboard_uri_list().ok_or_else(|| "剪贴板中未找到文件 URI".to_string())
    }
}

