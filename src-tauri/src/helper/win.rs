use std::path::Path;
use windows::{
    core::{BOOL, PCWSTR, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, S_OK},
        System::Com,
        UI::{Shell, WindowsAndMessaging},
    },
};

#[allow(unused)]
pub fn get_window_class_name(hwnd: HWND) -> String {
    let mut class_name = [0u16; 256];
    let len = unsafe { WindowsAndMessaging::GetClassNameW(hwnd, &mut class_name) };
    String::from_utf16_lossy(&class_name[0..len as usize])
}

#[allow(unused)]
pub fn open_by_default(file_path: &str, hwnd: HWND) -> windows::core::Result<()> {
    // 将字符串转为 PCWSTR
    let file_path_wide: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
    let file_path_wide = PCWSTR(file_path_wide.as_ptr());

    // 配置 SHELLEXECUTEINFOW 结构
    let mut sei = Shell::SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<Shell::SHELLEXECUTEINFOW>() as u32,
        lpFile: file_path_wide,              // 文件路径
        nShow: 1,                            // SW_SHOWNORMAL
        fMask: Shell::SEE_MASK_INVOKEIDLIST, // 打开方式对话框
        hwnd,                                // 父窗口句柄（可替换为实际窗口句柄）
        ..Default::default()
    };

    // 调用 ShellExecuteExW
    unsafe {
        Shell::ShellExecuteExW(&mut sei)?;
    }
    Ok(())
}

#[allow(unused)]
pub fn show_open_with_dialog(file_path: &str, hwnd: HWND) -> windows::core::Result<()> {
    // 初始化 COM
    unsafe { Com::CoInitializeEx(None, Com::COINIT_APARTMENTTHREADED) };

    // 配置 OPENASINFO 结构
    let file_path_wide: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
    let file_path_wide = PCWSTR(file_path_wide.as_ptr());
    let open_as_info = Shell::OPENASINFO {
        pcszFile: file_path_wide,
        pcszClass: PCWSTR::null(), // 文件类型 (可选)
        oaifInFlags: Shell::OAIF_ALLOW_REGISTRATION | Shell::OAIF_EXEC, // 允许注册 & 立即执行
    };

    // 调用 SHOpenWithDialog 打开“打开方式”对话框
    unsafe {
        Shell::SHOpenWithDialog(Some(hwnd), &open_as_info)?;
    }

    // 释放 COM
    unsafe { Com::CoUninitialize() };

    Ok(())
}
#[allow(unused)]
pub fn get_default_program_name(path: &str) -> Result<String, String> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or("No file extension".to_string())?;

    let ext = format!(".{}", ext);
    let ext_wide: Vec<u16> = ext.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // 获取所需缓冲区大小
        let mut size = 0u32;
        let _ = Shell::AssocQueryStringW(
            Shell::ASSOCF_NONE,
            Shell::ASSOCSTR_FRIENDLYAPPNAME,
            PCWSTR(ext_wide.as_ptr()),
            None,
            None,
            &mut size,
        );

        // 分配缓冲区并获取程序名
        let mut buffer = vec![0u16; size as usize];
        let result = Shell::AssocQueryStringW(
            Shell::ASSOCF_NONE,
            Shell::ASSOCSTR_FRIENDLYAPPNAME,
            PCWSTR(ext_wide.as_ptr()),
            None,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut size,
        );

        if result != S_OK {
            return Err(format!("获取 name Buffer 失败 -> {:?}", result));
        }

        // 转换为字符串
        let name = String::from_utf16_lossy(&buffer[..size as usize - 1]);
        Ok(name)
    }
}
#[allow(unused)]
pub fn get_window_text(hwnd: HWND) -> String {
    let len = unsafe { WindowsAndMessaging::GetWindowTextLengthW(hwnd) + 1 };
    let mut buffer = vec![0u16; len as usize];
    let len = unsafe { WindowsAndMessaging::GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[0..len as usize])
}

#[allow(unused)]
pub fn is_rename_mode() -> bool {
    unsafe {
        let hwnd = WindowsAndMessaging::GetForegroundWindow();
        if hwnd.0 as usize == 0 {
            return false;
        }
        let class_name = get_window_class_name(hwnd);
        println!("#### class_name: {}", class_name);
        // 常见 Explorer 的重命名输入框类名为 "WorkerW" → "SHELLDLL_DefView" → "SysListView32" 等，
        // 也可能是直接 "Edit"。这里可以视具体情况判断。
        class_name.contains("Edit")
    }
}

#[allow(unused)]
pub fn is_cursor_activated(hwnd: HWND) -> bool {
    let t_id = unsafe { WindowsAndMessaging::GetWindowThreadProcessId(hwnd, None) };

    let mut gti = WindowsAndMessaging::GUITHREADINFO::default();
    gti.cbSize = std::mem::size_of::<WindowsAndMessaging::GUITHREADINFO>() as u32;

    unsafe { WindowsAndMessaging::GetGUIThreadInfo(t_id, &mut gti) };

    gti.flags.0 != 0 || !gti.hwndCaret.eq(&HWND::default()) || is_listary_toolbar_visible()
}

fn is_listary_toolbar_visible() -> bool {
    unsafe extern "system" fn find_listary_window_proc(hwnd: HWND, l_param: LPARAM) -> BOOL {
        let mut class_buffer = [0u16; 256];
        let result = WindowsAndMessaging::GetClassNameW(hwnd, &mut class_buffer);

        if result != 0 {
            let class_name = String::from_utf16_lossy(&class_buffer[..result as usize]);

            if class_name.starts_with("Listary_WidgetWin_")
                && WindowsAndMessaging::IsWindowVisible(hwnd).as_bool()
            {
                *(l_param.0 as usize as *mut bool) = true;
                return BOOL::from(false); // FALSE
            }
        }

        BOOL::from(true) // TRUE
    }

    let mut found = false;
    let _ = unsafe {
        WindowsAndMessaging::EnumWindows(
            Some(find_listary_window_proc),
            LPARAM(&mut found as *mut _ as isize),
        )
    };

    found
}
