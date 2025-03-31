use std::sync::{mpsc, LazyLock, Mutex};
use std::thread;
use tauri::{
    webview::PageLoadEvent, AppHandle, Error as TauriError, Manager, WebviewUrl,
    WebviewWindowBuilder,
};
use windows::{
    core::{w, Error as WError, Interface, BOOL, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
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
            Input::KeyboardAndMouse,
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

#[path = "./helper/mod.rs"]
mod helper;
use helper::{monitor, win};

#[path = "./utils/mod.rs"]
mod utils;
use utils::{get_file_info, File as UFile};

#[derive(Debug, Clone)]
pub struct PreviewFile {
    hook_handle: Option<WindowsAndMessaging::HHOOK>, // 钩子的句柄
    app_handle: Option<AppHandle>,
}
unsafe impl Send for PreviewFile {}
unsafe impl Sync for PreviewFile {}

#[derive(Debug)]
pub enum FwWindowType {
    Explorer,
    Desktop,
    Dialog,
}

struct Selected;

impl Selected {
    pub fn new() -> Result<String, WError> {
        match Self::get_selected_file() {
            Ok(path) => Ok(path),
            Err(e) => {
                log::error!("Error: {:?}", e);
                Err(e)
            }
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
                println!("name: {:?}", name);
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
                }
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

impl Drop for Selected {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}

pub struct WebRoute {
    path: String,
    query: UFile,
}
impl WebRoute {
    pub fn to_url(&self) -> String {
        let mut url = self.path.clone();
        url.push_str("?");
        url.push_str(
            format!(
                "file_type={}&path={}&extension={}&size={}&last_modified={}&name={}",
                self.query.get_file_type(),
                urlencoding::encode(&self.query.get_path()),
                self.query.get_extension(),
                self.query.get_size(),
                self.query.get_last_modified(),
                self.query.get_name()
            )
            .as_str(),
        );
        url
    }
    pub fn new(path: String, query: UFile) -> Self {
        Self { path, query }
    }
    pub fn get_route(type_str: &str, file_info: &UFile) -> WebRoute {
        match type_str {
            "Markdown" => WebRoute::new("/preview/md".to_string(), file_info.clone()),
            "Text" => WebRoute::new("/preview/text".to_string(), file_info.clone()),
            "Image" => WebRoute::new("/preview/image".to_string(), file_info.clone()),
            "Audio" => WebRoute::new("/preview/audio".to_string(), file_info.clone()),
            "Video" => WebRoute::new("/preview/video".to_string(), file_info.clone()),
            "Font" => WebRoute::new("/preview/font".to_string(), file_info.clone()),
            "Code" => WebRoute::new("/preview/code".to_string(), file_info.clone()),
            "Book" => WebRoute::new("/preview/book".to_string(), file_info.clone()),
            "Archive" => WebRoute::new("/preview/archive".to_string(), file_info.clone()),
            "Doc" => WebRoute::new("/preview/document".to_string(), file_info.clone()),
            _ => WebRoute::new("/preview/not-support".to_string(), file_info.clone()),
        }
    }
}

#[allow(dead_code)]
impl PreviewFile {
    // 注册键盘钩子
    pub fn set_keyboard_hook(&mut self) {
        let hook_ex = unsafe {
            WindowsAndMessaging::SetWindowsHookExW(
                WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(Self::keyboard_proc), // 使用结构体的键盘回调
                None,                      // 当前进程实例句柄
                0,
            )
        };
        match hook_ex {
            Ok(result) => {
                self.hook_handle = Some(result);
            }
            Err(_) => {
                self.hook_handle = None;
            }
        }
    }

    // 取消键盘钩子
    pub fn remove_keyboard_hook(&mut self) {
        if let Some(hook) = self.hook_handle {
            unsafe {
                let _ = WindowsAndMessaging::UnhookWindowsHookEx(hook);
            }
            self.hook_handle = None;
        }
    }

    // 按键处理逻辑
    pub fn handle_key_down(&self, vk_code: u32) {
        if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32 {
            let result = Self::preview_file(self.app_handle.clone().unwrap());
            if result.is_err() {
                log::error!("Error: {:?}", result.err().unwrap());
            }
        }
    }

    // 全局键盘钩子的回调函数
    extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // 确保消息被传递给其他应用程序
        let next_hook_result =
            unsafe { WindowsAndMessaging::CallNextHookEx(None, ncode, wparam, lparam) };
        #[cfg(debug_assertions)]
        log::info!("Hook called - next_hook_result: {:?}", next_hook_result);

        if ncode >= 0
            && (wparam.0 == WindowsAndMessaging::WM_KEYDOWN as usize
                || wparam.0 == WindowsAndMessaging::WM_SYSKEYDOWN as usize)
        {
            let kb_struct = unsafe { *(lparam.0 as *const WindowsAndMessaging::KBDLLHOOKSTRUCT) };
            let vk_code = kb_struct.vkCode;

            if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32 {
                let type_str = Selected::get_focused_type();
                if type_str.is_none() {
                    return next_hook_result;
                }

                // 获取 PreviewFile 实例并处理按键事件
                if let Some(app) = get_global_instance() {
                    app.handle_key_down(vk_code);
                }
            }
        }

        next_hook_result
    }
    fn calc_window_size(file_type: &str) -> (f64, f64) {
        let monitor_info = monitor::get_monitor_info();

        let scale = monitor_info.scale;
        let mut width = 1000.0;
        let mut height = 600.0;

        if monitor_info.width > 0.0 {
            if file_type == "Audio" {
                width = 600.0;
                height = 240.0;
            } else {
                width = monitor_info.width * 0.8;
                height = monitor_info.height * 0.8;
            }
        }

        if monitor_info.scale > 1.0 {
            width = helper::get_scaled_size(width, scale);
            height = helper::get_scaled_size(height, scale);
        }

        log::info!(
            "Client Rect: width is {}, height is {}, scale is {}",
            width,
            height,
            scale
        );
        (width, height)
    }

    pub fn preview_file(app: AppHandle) -> Result<(), TauriError> {
        let file_path = Selected::new();
        if file_path.is_ok() {
            let file_path = file_path.unwrap();
            let file_info = get_file_info(&file_path);

            if file_info.is_none() {
                return Ok(());
            }

            let file_info = file_info.unwrap();
            let file_type = file_info.get_file_type();

            let (width, height) = Self::calc_window_size(&file_type);

            match app.get_webview_window("preview") {
                Some(window) => {
                    let type_str = file_info.get_file_type();
                    let route = WebRoute::get_route(&type_str, &file_info);

                    let url = route.to_url();
                    let js = format!("window.location.href = '{}'", &url);
                    let _ = window.eval(js.as_str());

                    let _ = window.show();
                    let _ = window.set_focus();
                }
                None => {
                    let result = WebviewWindowBuilder::new(
                        &app,
                        "preview",
                        WebviewUrl::App("/preview".into()),
                    )
                    .center()
                    .decorations(false)
                    .skip_taskbar(false)
                    .auto_resize()
                    .inner_size(width, height)
                    .min_inner_size(300.0, 300.0)
                    .on_page_load(move |window, payload| {
                        let cur_path = payload.url().path();
                        if cur_path == "/preview" {
                            match payload.event() {
                                PageLoadEvent::Finished => {
                                    let type_str = file_info.get_file_type();
                                    let route = WebRoute::get_route(&type_str, &file_info);

                                    let url = route.to_url();
                                    let js = format!("window.location.href = '{}'", &url);
                                    let _ = window.eval(js.as_str());

                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                _ => {}
                            }
                        }
                    })
                    .focused(true)
                    .visible_on_all_workspaces(true)
                    .build();

                    if let Ok(preview) = result {
                        let _ = preview.show();
                    }
                }
            }
        } else {
            log::error!("Error: {:?}", file_path.err().unwrap());
        }

        Ok(())
    }

    pub fn new() -> Self {
        Self {
            hook_handle: None,
            app_handle: None,
        }
    }
}

static PREVIEW_INSTANCE: LazyLock<Mutex<Option<PreviewFile>>> = LazyLock::new(|| Mutex::new(None));
// 函数用于设置全局 PreviewFile 实例
pub fn set_global_instance(instance: PreviewFile) {
    if let Ok(mut handle) = PREVIEW_INSTANCE.lock() {
        *handle = Some(instance);
    }
}
// 函数用于获取全局 PreviewFile 实例
fn get_global_instance() -> Option<PreviewFile> {
    PREVIEW_INSTANCE.lock().ok()?.clone()
}

impl Drop for PreviewFile {
    fn drop(&mut self) {
        println!("Dropping PreviewFile instance");
        self.remove_keyboard_hook();
    }
}

impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile::new()
    }
}

pub type PreviewState = Mutex<PreviewFile>;

//noinspection ALL
// 公开一个全局函数来初始化 PreviewFile
pub fn init_preview_file(handle: AppHandle) -> PreviewState {
    let mut preview_file = PreviewFile::default();
    preview_file.set_keyboard_hook();
    preview_file.app_handle = Some(handle);

    // 将实例存储在全局变量中
    set_global_instance(preview_file.clone());

    Mutex::new(preview_file)
}
