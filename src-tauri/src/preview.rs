use std::sync::{Arc, LazyLock, Mutex};
use tauri::{
    webview::PageLoadEvent, AppHandle, Error as TauriError, Manager, WebviewUrl,
    WebviewWindowBuilder,
};

#[cfg(windows)]
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse, WindowsAndMessaging},
};

#[path = "./helper/mod.rs"]
mod helper;
use helper::{monitor, selected_file::Selected};

#[path = "./utils/mod.rs"]
mod utils;
use utils::{get_file_info, File as UFile};

// ── Windows-specific fields ──────────────────────────────────────────────────

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct PreviewFile {
    hook_handle: Option<WindowsAndMessaging::HHOOK>,
    app_handle: Option<AppHandle>,
}

#[cfg(windows)]
unsafe impl Send for PreviewFile {}
#[cfg(windows)]
unsafe impl Sync for PreviewFile {}

// ── Linux / other platforms ──────────────────────────────────────────────────

#[cfg(not(windows))]
#[derive(Debug, Clone)]
pub struct PreviewFile {
    #[allow(dead_code)]
    app_handle: Option<AppHandle>,
}

#[cfg(not(windows))]
unsafe impl Send for PreviewFile {}
#[cfg(not(windows))]
unsafe impl Sync for PreviewFile {}

// ── Shared types ─────────────────────────────────────────────────────────────

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

// ── Windows implementation ───────────────────────────────────────────────────

#[cfg(windows)]
#[allow(dead_code)]
impl PreviewFile {
    pub fn set_keyboard_hook(&mut self) {
        let hook_ex = unsafe {
            WindowsAndMessaging::SetWindowsHookExW(
                WindowsAndMessaging::WH_KEYBOARD_LL,
                Some(Self::keyboard_proc),
                None,
                0,
            )
        };
        match hook_ex {
            Ok(result) => {
                self.hook_handle = Some(result);
            },
            Err(_) => {
                self.hook_handle = None;
            },
        }
    }

    pub fn remove_keyboard_hook(&mut self) {
        if let Some(hook) = self.hook_handle {
            unsafe {
                let _ = WindowsAndMessaging::UnhookWindowsHookEx(hook);
            }
            self.hook_handle = None;
        }
    }

    pub fn handle_key_down(&self, vk_code: u32) {
        if vk_code == KeyboardAndMouse::VK_SPACE.0 as u32 {
            let result = Self::preview_file(self.app_handle.clone().unwrap());
            if result.is_err() {
                log::error!("Error: {:?}", result.err().unwrap());
            }
        }
    }

    extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
                width = 560.0;
                height = 200.0;
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

            let preview_state = app.state::<PreviewState>();
            let mut preview_state = preview_state.lock().unwrap();
            preview_state.input_path = file_path.clone();

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
                },
                None => {
                    let result = WebviewWindowBuilder::new(
                        &app,
                        "preview",
                        WebviewUrl::App("/preview".into()),
                    )
                    .title("Preview")
                    .center()
                    .devtools(cfg!(debug_assertions))
                    .decorations(false)
                    .skip_taskbar(false)
                    .auto_resize()
                    .inner_size(width, height)
                    .min_inner_size(300.0, 200.0)
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
                                },
                                _ => {},
                            }
                        }
                    })
                    .focused(true)
                    .visible_on_all_workspaces(true)
                    .build();
                    if let Ok(preview) = result {
                        let _ = preview.show();
                    }
                },
            }
        } else {
            log::error!("Error: {:?}", file_path.err().unwrap());
        }

        Ok(())
    }

    pub fn new() -> Self {
        Self { hook_handle: None, app_handle: None }
    }
}

#[cfg(windows)]
impl Drop for PreviewFile {
    fn drop(&mut self) {
        println!("Dropping PreviewFile instance");
        self.remove_keyboard_hook();
    }
}

#[cfg(windows)]
impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile::new()
    }
}

// ── Linux / other platforms implementation ───────────────────────────────────

#[cfg(not(windows))]
#[allow(dead_code)]
impl PreviewFile {
    pub fn new() -> Self {
        Self { app_handle: None }
    }

    fn calc_window_size(file_type: &str) -> (f64, f64) {
        let monitor_info = monitor::get_monitor_info();

        let scale = monitor_info.scale;
        let mut width = 1000.0;
        let mut height = 600.0;

        if monitor_info.width > 0.0 {
            if file_type == "Audio" {
                width = 560.0;
                height = 200.0;
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

            let preview_state = app.state::<PreviewState>();
            let mut preview_state = preview_state.lock().unwrap();
            preview_state.input_path = file_path.clone();

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
                },
                None => {
                    let result = WebviewWindowBuilder::new(
                        &app,
                        "preview",
                        WebviewUrl::App("/preview".into()),
                    )
                    .title("Preview")
                    .center()
                    .devtools(cfg!(debug_assertions))
                    .decorations(false)
                    .skip_taskbar(false)
                    .auto_resize()
                    .inner_size(width, height)
                    .min_inner_size(300.0, 200.0)
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
                                },
                                _ => {},
                            }
                        }
                    })
                    .focused(true)
                    .visible_on_all_workspaces(true)
                    .build();
                    if let Ok(preview) = result {
                        let _ = preview.show();
                    }
                },
            }
        } else {
            log::error!("Error: {:?}", file_path.err().unwrap());
        }

        Ok(())
    }
}

#[cfg(not(windows))]
impl Default for PreviewFile {
    fn default() -> Self {
        PreviewFile::new()
    }
}

// ── Global singleton ─────────────────────────────────────────────────────────

static PREVIEW_INSTANCE: LazyLock<Mutex<Option<Arc<PreviewFile>>>> =
    LazyLock::new(|| Mutex::new(None));

pub fn set_global_instance(instance: PreviewFile) {
    if let Ok(mut handle) = PREVIEW_INSTANCE.lock() {
        *handle = Some(Arc::new(instance));
    }
}

#[cfg(windows)]
fn get_global_instance() -> Option<Arc<PreviewFile>> {
    if let Ok(guard) = PREVIEW_INSTANCE.lock() {
        guard.clone()
    } else {
        None
    }
}

// ── Shared state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PreviewStateInner {
    input_path: String,
}

unsafe impl Send for PreviewStateInner {}
unsafe impl Sync for PreviewStateInner {}

pub type PreviewState = Mutex<PreviewStateInner>;

// ── init_preview_file ─────────────────────────────────────────────────────────

/// Windows: 注册全局键盘钩子
#[cfg(windows)]
pub fn init_preview_file(handle: AppHandle) {
    let mut preview_file = PreviewFile::default();
    preview_file.set_keyboard_hook();
    preview_file.app_handle = Some(handle.clone());

    set_global_instance(preview_file);
    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));
}

/// Linux: 在后台线程中使用 rdev 监听全局键盘事件
#[cfg(target_os = "linux")]
pub fn init_preview_file(handle: AppHandle) {
    let preview_file = PreviewFile { app_handle: Some(handle.clone()) };
    set_global_instance(preview_file);
    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));

    let app_handle = handle.clone();

    if helper::is_wayland() {
        // Wayland 会话：使用 evdev（直接读 /dev/input/event*），可捕获所有应用的键盘事件，
        // 包括 GTK4/Wayland 原生应用（如 Nautilus）。
        // rdev 默认使用 X11/XRecord，对原生 Wayland 窗口的事件不可见。
        std::thread::spawn(move || {
            init_evdev_listener(app_handle);
        });
    } else {
        // X11 会话：继续使用 rdev
        std::thread::spawn(move || {
            use rdev::{listen, Event, EventType, Key};

            let callback = move |event: Event| {
                if let EventType::KeyPress(Key::Space) = event.event_type {
                    if Selected::get_focused_type().is_some() {
                        let result = PreviewFile::preview_file(app_handle.clone());
                        if let Err(e) = result {
                            log::error!("预览失败: {:?}", e);
                        }
                    }
                }
            };

            if let Err(e) = listen(callback) {
                log::error!("键盘监听器错误: {:?}", e);
            }
        });
    }
}

/// Wayland 键盘监听器：通过 evdev 读取原始输入设备事件
///
/// 需要用户对 `/dev/input/event*` 有读权限。在 Ubuntu/systemd 会话中，
/// `TAG+="uaccess"` udev 规则会让 logind 自动将当前登录用户授权访问这些设备。
/// 若检测到权限不足，会尝试通过 `pkexec` 自动安装该规则（会弹出系统授权对话框）。
#[cfg(target_os = "linux")]
fn init_evdev_listener(app_handle: AppHandle) {
    use evdev::{InputEventKind, Key};

    // 先枚举一次；若为空则检查原因
    let keyboards = enumerate_keyboards();

    let keyboards = if keyboards.is_empty() {
        match check_input_device_permission() {
            InputPermStatus::PermissionDenied => {
                log::warn!("evdev: /dev/input/event* 权限不足，尝试通过 pkexec 安装 udev 规则...");
                if try_install_udev_rule() {
                    log::info!("evdev: udev 规则安装成功，重新枚举键盘设备");
                    enumerate_keyboards()
                } else {
                    log::warn!(
                        "evdev: udev 规则安装失败（用户取消授权或 pkexec 不可用），回退到 rdev"
                    );
                    vec![]
                }
            },
            InputPermStatus::NoDevices => {
                log::warn!("evdev: 系统中未找到键盘设备");
                vec![]
            },
            InputPermStatus::Ok => {
                // 枚举返回空但权限 OK —— 理论上不应发生，直接回退
                vec![]
            },
        }
    } else {
        keyboards
    };

    if keyboards.is_empty() {
        // 最终回退：使用 rdev（仅对 X11 / XWayland 窗口有效）
        log::warn!(
            "evdev: 回退到 rdev 监听（原生 Wayland 应用（如 Nautilus）的按键事件可能无法捕获）"
        );
        use rdev::{listen, Event, EventType, Key as RdevKey};
        let callback = move |event: Event| {
            if let EventType::KeyPress(RdevKey::Space) = event.event_type {
                if Selected::get_focused_type().is_some() {
                    let result = PreviewFile::preview_file(app_handle.clone());
                    if let Err(e) = result {
                        log::error!("预览失败: {:?}", e);
                    }
                }
            }
        };
        if let Err(e) = listen(callback) {
            log::error!("键盘监听器错误: {:?}", e);
        }
        return;
    }

    // 为每个键盘设备启动一个监听线程
    let handles: Vec<_> = keyboards
        .into_iter()
        .map(|(path, mut device)| {
            let handle = app_handle.clone();
            std::thread::spawn(move || {
                log::info!("evdev: 监听键盘设备 {:?}", path);
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                if ev.kind() == InputEventKind::Key(Key::KEY_SPACE)
                                    && ev.value() == 1
                                {
                                    if Selected::get_focused_type().is_some() {
                                        let result = PreviewFile::preview_file(handle.clone());
                                        if let Err(e) = result {
                                            log::error!("预览失败: {:?}", e);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("evdev 读取错误 ({:?}): {:?}", path, e);
                            break;
                        },
                    }
                }
            })
        })
        .collect();

    for h in handles {
        if let Err(e) = h.join() {
            log::error!("evdev listener thread panicked: {:?}", e);
        }
    }
}

/// 枚举系统中所有支持空格键的键盘 evdev 设备
#[cfg(target_os = "linux")]
fn enumerate_keyboards() -> Vec<(std::path::PathBuf, evdev::Device)> {
    use evdev::Key;
    evdev::enumerate()
        .filter(|(_, d)| {
            d.supported_keys()
                .map_or(false, |k| k.contains(Key::KEY_SPACE))
        })
        .collect()
}

/// `/dev/input/event*` 设备的访问权限状态
#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq)]
enum InputPermStatus {
    /// 至少有一个设备可读
    Ok,
    /// 设备存在但均无读权限（EACCES）
    PermissionDenied,
    /// `/dev/input` 中没有 event* 设备
    NoDevices,
}

/// 检查 `/dev/input/event*` 的实际访问权限状态
#[cfg(target_os = "linux")]
fn check_input_device_permission() -> InputPermStatus {
    let entries = match std::fs::read_dir("/dev/input") {
        Ok(e) => e,
        Err(_) => return InputPermStatus::NoDevices,
    };

    let mut found_event = false;
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("event") {
            continue;
        }
        found_event = true;
        match std::fs::File::open(entry.path()) {
            Ok(_) => return InputPermStatus::Ok,
            Err(e) if e.raw_os_error() == Some(libc_eacces()) => {
                return InputPermStatus::PermissionDenied;
            },
            Err(_) => {},
        }
    }

    if found_event {
        // All event devices failed with a non-EACCES error; treat as denied
        InputPermStatus::PermissionDenied
    } else {
        InputPermStatus::NoDevices
    }
}

/// Returns the EACCES errno value (13 on Linux)
#[cfg(target_os = "linux")]
fn libc_eacces() -> i32 {
    13
}

/// udev 规则内容（使用 TAG+="uaccess" 让 logind 自动为活跃会话用户授权）
#[cfg(target_os = "linux")]
const UDEV_RULE_CONTENT: &str = "\
# QuickLook: grant active login session access to keyboard input devices.\n\
# This uses the systemd/logind \"uaccess\" tag so no manual group changes are needed.\n\
KERNEL==\"event[0-9]*\", SUBSYSTEM==\"input\", TAG+=\"uaccess\"\n";

/// udev 规则的安装路径
#[cfg(target_os = "linux")]
const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/99-quicklook-input.rules";

/// 通过 `pkexec` 安装 udev 规则并重新加载，使当前会话立即生效
///
/// `pkexec` 会弹出系统授权对话框；用户取消时返回 `false`。
/// 安装成功后调用 `udevadm trigger`，logind 会随即更新设备 ACL。
#[cfg(target_os = "linux")]
fn try_install_udev_rule() -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // pkexec tee <path>  — writes stdin to the target file with root privileges
    let child = Command::new("pkexec")
        .args(["tee", UDEV_RULE_PATH])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            log::error!("pkexec 启动失败: {:?}", e);
            return false;
        },
    };

    // Write rule content to pkexec's stdin
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(UDEV_RULE_CONTENT.as_bytes()) {
            log::error!("写入 udev 规则内容失败: {:?}", e);
            let _ = child.wait();
            return false;
        }
    }

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            log::error!("等待 pkexec 完成失败: {:?}", e);
            return false;
        },
    };

    if !status.success() {
        log::warn!(
            "pkexec 返回非零退出码（用户可能取消了授权）: {:?}",
            status.code()
        );
        return false;
    }

    // Reload udev rules
    let _ = Command::new("udevadm")
        .args(["control", "--reload-rules"])
        .status();

    // Trigger input subsystem so logind updates device ACLs in the current session
    let _ = Command::new("udevadm")
        .args(["trigger", "--subsystem-match=input", "--action=change"])
        .status();

    // Give logind/udevd a moment to apply the new ACLs
    std::thread::sleep(std::time::Duration::from_millis(500));

    true
}

/// 其他不支持的平台：空实现
#[cfg(not(any(windows, target_os = "linux")))]
pub fn init_preview_file(handle: AppHandle) {
    handle.manage::<PreviewState>(Mutex::new(PreviewStateInner::default()));
    log::warn!("当前平台不支持全局键盘监听，预览功能不可用");
}
