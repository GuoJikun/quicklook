use log::info;
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
    archive, document, get_default_program_name, get_monitor_info, show_open_with_dialog,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            let handle = app.handle();
            handle.plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Debug) // 改为 Debug 级别
                    .max_file_size(50000)
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                    .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                    .build(),
            )?;

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
                let autostart_manager = app.autolaunch();
                let _ = autostart_manager.enable();
            }

            // 创建托盘
            tray::create_tray(app)?;
            // 初始化预览文件
            let app_handle = app.handle().clone();
            let preview_instance = preview::init_preview_file(app_handle);
            app.manage(preview_instance);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_open_with_dialog,
            archive,
            document,
            get_monitor_info,
            get_default_program_name
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if code.is_none() {
                    api.prevent_exit();
                } else {
                    info!("exit code: {:?}", code);
                }
            }
            _ => {
                println!("event: {:?}", event);
            }
        });
}
