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

// ── Active window detection ───────────────────────────────────────────────────

/// 获取当前活动窗口的应用类名
/// Wayland 环境优先，X11 作为回退
#[cfg(target_os = "linux")]
fn get_active_window_class() -> Option<String> {
    if super::is_wayland() {
        get_active_window_class_wayland().or_else(get_active_window_class_x11)
    } else {
        get_active_window_class_x11()
    }
}

/// X11：通过 xdotool 获取活动窗口类名
#[cfg(target_os = "linux")]
fn get_active_window_class_x11() -> Option<String> {
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

/// Wayland：依次尝试 Hyprland → Sway/wlroots → KDE → GNOME → kdotool
#[cfg(target_os = "linux")]
fn get_active_window_class_wayland() -> Option<String> {
    // Hyprland
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
    {
        if output.status.success() {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(class) = val.get("class").and_then(|v| v.as_str()) {
                    if !class.is_empty() {
                        return Some(class.to_string());
                    }
                }
            }
        }
    }

    // Sway / wlroots-based compositors
    if let Ok(output) = std::process::Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
    {
        if output.status.success() {
            if let Ok(tree) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(class) = find_sway_focused_class(&tree) {
                    return Some(class);
                }
            }
        }
    }

    // GNOME Wayland：Shell.Introspect.GetWindows（GNOME 3.36+，推荐方式）
    if let Some(class) = get_active_window_class_gnome_introspect() {
        return Some(class);
    }

    // GNOME Wayland：Shell.Eval 回退（GNOME < 41 或启用了 unsafe-mode-menu 扩展）
    if let Some(class) = get_active_window_class_gnome_eval() {
        return Some(class);
    }

    // KDE Wayland: kdotool (mirrors xdotool API)
    if let Ok(output) = std::process::Command::new("kdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
    {
        if output.status.success() {
            let class = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !class.is_empty() {
                return Some(class);
            }
        }
    }

    None
}

/// GNOME Wayland：通过 `org.gnome.Shell.Introspect.GetWindows` 获取焦点窗口的应用 ID
/// 此接口自 GNOME 3.36 起可用，返回所有窗口及其属性（含 `has-focus` 和 `app-id`）。
#[cfg(target_os = "linux")]
fn get_active_window_class_gnome_introspect() -> Option<String> {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/Shell/Introspect",
            "--method",
            "org.gnome.Shell.Introspect.GetWindows",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_gnome_introspect_focused_app(&text)
}

/// 解析 `GetWindows` 的 GVariant 文本输出，找出 `has-focus: true` 的窗口并返回其 app-id。
///
/// 示例输出（简化）：
/// ```
/// ({'8388611': {'app-id': <'org.gnome.Nautilus'>, 'has-focus': <true>, ...}, ...},)
/// ```
#[cfg(target_os = "linux")]
fn parse_gnome_introspect_focused_app(text: &str) -> Option<String> {
    use std::sync::OnceLock;

    static RE_APP_ID: OnceLock<regex::Regex> = OnceLock::new();
    static RE_WM_CLASS: OnceLock<regex::Regex> = OnceLock::new();

    let re_app_id =
        RE_APP_ID.get_or_init(|| regex::Regex::new(r"'app-id':\s*<'([^']+)'>").unwrap());
    let re_wm_class =
        RE_WM_CLASS.get_or_init(|| regex::Regex::new(r"'wm-class':\s*<'([^']+)'>").unwrap());

    // 找到第一个 has-focus: <true> 的位置
    let focus_marker = "'has-focus': <true>";
    let focus_pos = text.find(focus_marker)?;

    // 从该位置向前找到所在窗口属性块的起始 `{`
    let block_start = text[..focus_pos].rfind('{')? + 1;

    // 从 block_start 向后找到结束 `}`
    let block_end = block_start + text[block_start..].find('}')?;
    let block = &text[block_start..block_end];

    // 优先提取 app-id（如 'org.gnome.Nautilus'）
    if let Some(cap) = re_app_id.captures(block) {
        let id = cap.get(1)?.as_str().to_string();
        if !id.is_empty() {
            return Some(id);
        }
    }

    // 回退到 wm-class（部分合成器可能包含此字段）
    if let Some(cap) = re_wm_class.captures(block) {
        let class = cap.get(1)?.as_str().to_string();
        if !class.is_empty() {
            return Some(class);
        }
    }

    None
}

/// GNOME Wayland：通过 `org.gnome.Shell.Eval` 获取焦点窗口类名
/// 仅适用于 GNOME < 41 或启用了 `unsafe-mode-menu` 扩展的系统。
#[cfg(target_os = "linux")]
fn get_active_window_class_gnome_eval() -> Option<String> {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/Shell",
            "--method",
            "org.gnome.Shell.Eval",
            "global.display.focus_window?.wm_class ?? ''",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    // 输出格式：(true, 'ClassName') 或 (false, '')
    use std::sync::OnceLock;
    static RE_EVAL: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE_EVAL.get_or_init(|| regex::Regex::new(r"\(true,\s*'([^']*)'\)").unwrap());
    let cap = re.captures(text.trim())?;
    let class = cap.get(1)?.as_str().to_string();
    if class.is_empty() {
        None
    } else {
        Some(class)
    }
}

/// 递归在 Sway/i3 窗口树中寻找 focused 节点的 app_id
#[cfg(target_os = "linux")]
fn find_sway_focused_class(node: &serde_json::Value) -> Option<String> {
    if node
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        // Native Wayland window
        if let Some(id) = node.get("app_id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
        // XWayland window in Sway
        if let Some(props) = node.get("window_properties") {
            if let Some(class) = props.get("class").and_then(|v| v.as_str()) {
                if !class.is_empty() {
                    return Some(class.to_string());
                }
            }
        }
    }

    for key in &["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|v| v.as_array()) {
            for child in children {
                if let Some(r) = find_sway_focused_class(child) {
                    return Some(r);
                }
            }
        }
    }

    None
}

// ── URI-list parsing ──────────────────────────────────────────────────────────

/// 从 URI 列表文本中解析第一个本地文件路径
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

// ── Clipboard reading ─────────────────────────────────────────────────────────

/// 读取剪贴板中的 URI 列表
/// Wayland 环境优先（wl-paste），X11 作为回退（xclip / xsel）
#[cfg(target_os = "linux")]
fn read_clipboard_uri_list() -> Option<String> {
    if super::is_wayland() {
        read_clipboard_wayland().or_else(read_clipboard_x11)
    } else {
        read_clipboard_x11()
    }
}

/// Wayland：通过 wl-paste 读取剪贴板
#[cfg(target_os = "linux")]
fn read_clipboard_wayland() -> Option<String> {
    // 优先读取 text/uri-list MIME 类型（文件管理器通常写入这种格式）
    if let Ok(output) = std::process::Command::new("wl-paste")
        .args(["--no-newline", "--type", "text/uri-list"])
        .output()
    {
        if output.status.success() {
            let content = String::from_utf8_lossy(&output.stdout);
            if let Some(path) = parse_uri_list(&content) {
                return Some(path);
            }
        }
    }

    // 回退：读取纯文本（部分文件管理器写入 URI 格式的纯文本）
    if let Ok(output) = std::process::Command::new("wl-paste")
        .args(["--no-newline"])
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

/// X11：通过 xclip 或 xsel 读取剪贴板
#[cfg(target_os = "linux")]
fn read_clipboard_x11() -> Option<String> {
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

    // 回退到 xsel
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

// ── Key simulation (Ctrl+C) ───────────────────────────────────────────────────

/// 向当前活动窗口发送 Ctrl+C
/// Wayland：先尝试 ydotool，回退到 xdotool（XWayland）
/// X11：使用 xdotool
#[cfg(target_os = "linux")]
fn send_copy_shortcut() -> Result<(), String> {
    if super::is_wayland() {
        // ydotool: Wayland 原生输入模拟（需要 uinput 权限）
        // KEY_LEFTCTRL=29, KEY_C=46
        if let Ok(status) = std::process::Command::new("ydotool")
            .args(["key", "29:1", "46:1", "46:0", "29:0"])
            .status()
        {
            if status.success() {
                return Ok(());
            }
        }

        // 回退：xdotool 在 XWayland 下仍然可用
        let status = std::process::Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+c"])
            .status()
            .map_err(|e| format!("xdotool 执行失败: {}", e))?;

        if !status.success() {
            return Err("发送 Ctrl+C 失败（ydotool 和 xdotool 均不可用）".to_string());
        }
        Ok(())
    } else {
        let status = std::process::Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+c"])
            .status()
            .map_err(|e| format!("xdotool 执行失败: {}", e))?;

        if !status.success() {
            return Err("xdotool key 命令失败".to_string());
        }
        Ok(())
    }
}

// ── Selected impl ─────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
#[allow(dead_code)]
impl Selected {
    /// 获取当前文件管理器中选中的文件路径
    ///
    /// 实现原理：向活动窗口发送 Ctrl+C，然后从剪贴板读取 URI 列表。
    ///
    /// Wayland 所需工具（按优先级）：
    ///   - 窗口检测：hyprctl / swaymsg / kdotool
    ///   - 输入模拟：ydotool（需 uinput 权限）或 xdotool（XWayland 回退）
    ///   - 剪贴板：wl-paste（wl-clipboard）
    ///
    /// X11 所需工具：
    ///   `sudo apt install xdotool xclip`
    pub fn new() -> Result<String, String> {
        Self::get_selected_file()
    }

    /// 检查当前活动窗口是否为受支持的文件管理器，是则返回 Some(())
    pub fn get_focused_type() -> Option<()> {
        Self::get_selected_type()
    }

    fn get_selected_type() -> Option<()> {
        let class_name = get_active_window_class()?;
        let lower = class_name.to_lowercase();
        // 支持 Nautilus、Nemo、Thunar、Dolphin、PCManFM、Caja 等常见文件管理器
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
        send_copy_shortcut()?;

        // 等待剪贴板更新
        std::thread::sleep(std::time::Duration::from_millis(150));

        read_clipboard_uri_list().ok_or_else(|| "剪贴板中未找到文件 URI".to_string())
    }
}
