use tauri::{
    App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Wry,
    menu::{Menu, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_store::StoreExt;
use tauri_plugin_updater::UpdaterExt;

use crate::i18n::{self, Locale};

/// 托盘图标 id，重建菜单时按此取回托盘实例
const TRAY_ID: &str = "tray";

async fn updater_check(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    let texts = i18n::current_locale(&app).tray_texts();

    let checked = app.updater()?.check().await;
    if checked.is_err() {
        let _ = app
            .dialog()
            .message(texts.update_failed)
            .kind(MessageDialogKind::Error)
            .title(texts.update_error_title)
            .blocking_show();
        return Ok(());
    }

    let checked = checked.unwrap();

    match checked {
        Some(_) => {
            let _ = WebviewWindowBuilder::new(&app, "upgrade", WebviewUrl::App("/upgrade".into()))
                .center()
                .title(texts.update_window_title)
                .inner_size(600.0, 500.0)
                .focused(true)
                .window_classname("quicklook-upgrade")
                .auto_resize()
                .build();
        },
        None => {
            let _ = app
                .dialog()
                .message(texts.update_none)
                .kind(MessageDialogKind::Info)
                .title(texts.update_check_title)
                .blocking_show();
        },
    }

    Ok(())
}

/// 开机自启当前是否启用：store 优先，缺失时读取系统实际状态
fn autostart_enabled(app: &AppHandle) -> bool {
    let stored = app
        .store(i18n::CONFIG_STORE)
        .ok()
        .and_then(|store| store.get("autostart"))
        .and_then(|value| value.as_bool());

    stored
        .or_else(|| app.autolaunch().is_enabled().ok())
        .unwrap_or(false)
}

/// 按当前语言与自启状态构建托盘菜单
fn build_menu(app: &AppHandle, locale: Locale) -> tauri::Result<Menu<Wry>> {
    let texts = locale.tray_texts();
    let autostart_text = if autostart_enabled(app) {
        texts.disable_autostart
    } else {
        texts.enable_autostart
    };

    let setting = MenuItem::with_id(app, "setting", texts.settings, true, None::<&str>)?;
    let auto_start = MenuItem::with_id(app, "auto_start", autostart_text, true, None::<&str>)?;
    let upgrade = MenuItem::with_id(app, "upgrade", texts.check_update, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)?;

    MenuBuilder::new(app)
        .items(&[&setting, &auto_start, &upgrade, &quit])
        .build()
}

/// 重建托盘菜单。语言切换与自启状态变化后调用。
///
/// 菜单事件处理器挂在托盘图标上（`TrayIcon::on_menu_event` 注册的是应用级监听，
/// 与具体 Menu 实例无关），因此整块替换菜单不会丢失事件响应。
pub fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log::warn!("未找到托盘图标 {TRAY_ID}，跳过菜单刷新");
        return;
    };

    match build_menu(app, i18n::current_locale(app)) {
        Ok(menu) => {
            if let Err(e) = tray.set_menu(Some(menu)) {
                log::warn!("刷新托盘菜单失败: {:?}", e);
            }
        },
        Err(e) => log::warn!("构建托盘菜单失败: {:?}", e),
    }
}

pub fn create_tray(app: &mut App) -> tauri::Result<()> {
    let handle = app.handle().clone();
    let locale = i18n::current_locale(&handle);
    let menu = build_menu(&handle, locale)?;

    let pkg_info = app.package_info();
    let name = pkg_info.name.clone();
    let version = pkg_info.version.clone();
    let tooltip_text = format!(
        "{} v{}.{}.{}",
        name, version.major, version.minor, version.patch
    );

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip_text)
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    let _ = tray
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            },
            "upgrade" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = updater_check(handle).await;
                });
            },
            "setting" => {
                log::debug!("Setting");
                let texts = i18n::current_locale(app).tray_texts();
                // 打开设置窗口
                if let Ok(webview_window) =
                    crate::helper::get_webview_window(app, "settings", "/settings")
                {
                    let _ = webview_window.set_title(texts.settings);
                    let _ = webview_window.show();
                }
            },
            "auto_start" => {
                let autostart_manager = app.autolaunch();
                let store = match app.store(i18n::CONFIG_STORE) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("Failed to open config.data store: {:?}", e);
                        return;
                    },
                };
                let store_auto_start = store
                    .get("autostart")
                    .unwrap_or(serde_json::Value::Bool(true));
                let is_enabled = store_auto_start.as_bool();

                if let Some(enabled) = is_enabled {
                    if enabled {
                        let _ = autostart_manager.disable();
                        store.set("autostart", serde_json::Value::Bool(false));
                        log::info!("自启动设置为禁用");
                    } else {
                        let _ = autostart_manager.enable();
                        store.set("autostart", serde_json::Value::Bool(true));
                        log::info!("自启动设置为开启");
                    }
                    // 菜单项文案随状态变化，直接整块重建
                    refresh_tray(app);
                }
            },
            // Add more events here
            _ => {},
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
