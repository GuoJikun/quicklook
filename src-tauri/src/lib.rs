use tauri::{Listener, Manager};
use tauri_plugin_autostart::MacosLauncher;
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt;

mod helper;
mod preview;
mod tray;

#[path = "./command.rs"]
mod command;
use command::{
    archive, decode_video, document, get_default_program_name, get_monitor_info,
    show_open_with_dialog,
};
use tauri_plugin_store::StoreExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    // 注册插件
    builder = builder
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info) // 默认日志级别为 Info
                .max_file_size(10000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        );

    // 初始化
    let app = builder
        .setup(|app| {
            let handle = app.handle();
            let store = app.store("config.data")?;

            let config = helper::config::read_config(handle)?;
            app.manage(config);

            let handle_clone = handle.clone();
            let _ = app.listen("config_update", move |event| {
                let handle = handle_clone.clone();

                println!("update event: {:?}", event);
                if let Ok(conf) = helper::config::read_config(&handle_clone) {
                    handle.manage(conf);
                }
            });

            // 自动启动
            #[cfg(not(debug_assertions))]
            {
                let config_autostart = store
                    .get("autostart")
                    .unwrap_or(serde_json::Value::Bool(true));
                let autostart_manager = app.autolaunch();
                let is_enabled = config_autostart.as_bool();
                log::info!("自启动状态: {:?}", is_enabled);

                if let Some(enabled) = is_enabled {
                    if enabled {
                        let _ = autostart_manager.enable();
                        log::info!("自启动设置为开启");
                    } else {
                        let _ = autostart_manager.disable();
                        log::info!("自启动设置为禁用");
                    }
                } else {
                    let _ = autostart_manager.enable();
                    log::warn!("无法检查自启动状态时，默认启用自启动");
                }
            }

            // 初始化预览文件
            let app_handle = app.handle().clone();
            preview::init_preview_file(app_handle);

            // 创建托盘
            tray::create_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_open_with_dialog,
            archive,
            document,
            get_monitor_info,
            get_default_program_name,
            decode_video
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    // 阻止因窗口全部关闭而退出应用
    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() {
                api.prevent_exit();
            }
        }
        _ => {}
    });
}
