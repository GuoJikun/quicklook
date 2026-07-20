use std::sync::{mpsc, LazyLock};
use std::thread;
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

use crate::helper::win;

#[allow(dead_code)]
#[derive(Debug)]
pub enum FwWindowType {
    Explorer,
    Desktop,
    Dialog,
}

type ComTask = (
    Box<dyn FnOnce() -> Result<String, WError> + Send>,
    mpsc::SyncSender<Result<String, WError>>,
);

struct ComThread {
    tx: mpsc::SyncSender<ComTask>,
}

static COM_THREAD: LazyLock<ComThread> = LazyLock::new(|| {
    let (tx, rx) = mpsc::sync_channel::<ComTask>(1);
    thread::Builder::new()
        .name("com-worker".into())
        .spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            while let Ok((task, result_tx)) = rx.recv() {
                result_tx.send(task()).ok();
            }
            unsafe {
                CoUninitialize();
            }
        })
        .expect("failed to spawn COM thread");
    ComThread { tx }
});

impl ComThread {
    fn run<F>(task: F) -> Result<String, WError>
    where
        F: FnOnce() -> Result<String, WError> + Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::sync_channel::<Result<String, WError>>(1);
        COM_THREAD
            .tx
            .send((Box::new(task), result_tx))
            .map_err(|_| WError::from_win32())?;
        result_rx.recv().map_err(|_| WError::from_win32())?
    }
}

#[allow(dead_code)]
pub struct Selected;

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
                FwWindowType::Explorer => Self::get_selected_file_from_explorer(),
                FwWindowType::Desktop => Self::get_selected_file_from_desktop(),
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

    fn get_selected_file_from_explorer() -> Result<String, WError> {
        ComThread::run(|| unsafe {
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

                let shell_browser = match Selected::dispath2browser(dispatch) {
                    Some(sb) => sb,
                    None => continue,
                };
                let phwnd = shell_browser.GetWindow()?;
                if hwnd_gfw.0 != phwnd.0 && result_hwnd.0 != phwnd.0 {
                    continue;
                }

                if win::is_cursor_activated(HWND::default()) {
                    continue;
                };

                let shell_view = shell_browser.QueryActiveShellView().unwrap();
                target_path = Selected::get_selected_file_path_from_shellview(shell_view);
            }

            Ok(target_path)
        })
    }

    fn get_selected_file_from_desktop() -> Result<String, WError> {
        ComThread::run(|| unsafe {
            let hwnd_gfw = WindowsAndMessaging::GetForegroundWindow();
            log::info!("hwnd_gfw: {:?}", hwnd_gfw);
            let shell_windows: Result<IShellWindows, WError> =
                CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER);
            if shell_windows.is_err() {
                log::info!("shell_windows 不存在");
                return Ok(String::new());
            }
            let shell_windows = shell_windows?;

            let pvar_loc: VARIANT = Variant::VariantInit();

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
                return Ok(String::new());
            };

            let shell_browser = match Selected::dispath2browser(dispatch) {
                Some(sb) => sb,
                None => {
                    log::info!("shell_browser 不存在");
                    return Ok(String::new());
                },
            };

            let shell_view = shell_browser.QueryActiveShellView()?;

            let target_path = Selected::get_selected_file_path_from_shellview(shell_view);

            Ok(target_path)
        })
    }

    fn get_selected_file_from_dialog() -> Result<String, WError> {
        let mut target_path = String::new();
        let fw_hwnd = unsafe { WindowsAndMessaging::GetForegroundWindow() };
        log::debug!("fw_hwnd: {:?}", fw_hwnd);

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
        let listview_raw = listview.0 as usize;
        // 将 UI Automation COM 操作移入专用 COM 线程，避免在当前线程直接初始化 COM
        let seleced_file_title = ComThread::run(move || unsafe {
            let listview = HWND(listview_raw as *mut _);
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let list_element = automation.ElementFromHandle(listview)?;

            let selection_pattern = list_element.GetCurrentPattern(UIA_SelectionPatternId)?;
            let selection = selection_pattern.cast::<IUIAutomationSelectionPattern>()?;

            let selected = selection.GetCurrentSelection()?;
            let count = selected.Length()?;
            let mut file_name = String::new();
            if count > 0 {
                let item = selected.GetElement(0)?;
                let name = item.GetCurrentPropertyValue(UIA_NamePropertyId)?;
                file_name = name.to_string();
            }
            Ok(file_name)
        })?;
        log::debug!("seleced_file_title: {:?}", seleced_file_title);

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
        log::debug!("target_path: {:?}", target_path);

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

            log::debug!("libraries_path: {:?}", folder_id);

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
        log::debug!("shell_items: {:?}", shell_items);
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
                log::debug!("display_name: {:?}", display_name);
                let tmp = display_name.to_string();
                if tmp.is_err() {
                    log::debug!("display_name error: {:?}", tmp.err());
                    continue;
                }
                target_path = tmp.unwrap();
                break;
            }
        }
        target_path
    }
}
