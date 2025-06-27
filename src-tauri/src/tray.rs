use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

#[path = "./helper/mod.rs"]
mod helper;

async fn updater_check(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    let checked = app.updater()?.check().await;
    if let Err(_) = checked {
        let _ = app
            .dialog()
            .message("检查更新失败")
            .kind(MessageDialogKind::Error)
            .title("Warning")
            .blocking_show();
        return Ok(());
    }

    let checked = checked.unwrap();

    match checked {
        Some(_) => {
            let _ = WebviewWindowBuilder::new(&app, "upgrade", WebviewUrl::App("/upgrade".into()))
                .center()
                .title("检查更新")
                .inner_size(500.0, 500.0)
                .focused(true)
                .window_classname("quicklook-upgrade")
                .auto_resize()
                .build();
        }
        None => {
            let _ = app
                .dialog()
                .message("没有可用的更新")
                .kind(MessageDialogKind::Info)
                .title("更新检查")
                .blocking_show();
        }
    }

    Ok(())
}

pub fn create_tray(app: &mut App) -> tauri::Result<()> {
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let upgrade = MenuItem::with_id(app, "upgrade", "检查更新", true, None::<&str>)?;
    let auto_start = MenuItem::with_id(app, "auto_start", "开机自启", true, None::<&str>)?;
    let setting = MenuItem::with_id(app, "setting", "设置", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&setting, &auto_start, &upgrade, &quit])
        .build()?;
    let pkg_info = app.package_info();
    let name = pkg_info.name.clone();
    let version = pkg_info.version.clone();
    let tooltip_text = format!(
        "{} v{}.{}.{}",
        &name, &version.major, &version.minor, &version.patch
    );

    let _ = TrayIconBuilder::with_id("tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip(tooltip_text)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            }
            "upgrade" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = updater_check(handle).await;
                });

                // let _ =
                //     WebviewWindowBuilder::new(app, "upgrade", WebviewUrl::App("/upgrade".into()))
                //         .center()
                //         .title("检查更新")
                //         .inner_size(400.0, 500.0)
                //         .focused(true)
                //         .window_classname("quicklook-upgrade")
                //         .auto_resize()
                //         .build();
            }
            "setting" => {
                println!("Setting");
                // 打开设置窗口
                if let Ok(webview_window) = helper::get_webview_window(app, "main", "/settings") {
                    let _ = webview_window.set_title("设置");
                    let _ = webview_window.show();
                }
            }
            // Add more events here
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app);

    Ok(())
}
