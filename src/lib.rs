mod app_state;
mod config;
mod display_driver;
mod package;
mod remote_auth;
mod remote_server;
mod renderer;
mod scene;
mod sensors;
mod superwidgets;
mod ui;
mod windows_startup;

use app_state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            ui::get_config,
            ui::get_remote_info,
            ui::set_remote_security,
            ui::get_active_screen,
            ui::save_config,
            ui::list_screens,
            ui::set_quick_screen,
            ui::save_screen,
            ui::load_screen,
            ui::new_screen,
            ui::delete_screen,
            ui::export_package,
            ui::share_package,
            ui::import_package,
            ui::get_preview,
            ui::preview_config,
            ui::test_sensors,
            ui::select_background,
            ui::select_background_folder,
            ui::select_gif,
            ui::list_superwidgets,
            ui::import_superwidget,
            ui::list_displays,
            ui::start_rendering,
            ui::stop_rendering,
            ui::render_once,
            ui::test_display,
            ui::set_display_brightness,
            ui::get_status,
            ui::get_autostart,
            ui::set_autostart,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let remote_state = app.state::<AppState>().inner().clone();
            tauri::async_runtime::spawn(remote_server::manage(remote_state));
            let show = MenuItem::with_id(app, "show", "Open TelemetryForge", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new().menu(&menu).tooltip("TelemetryForge");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;

            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                if let Err(error) =
                    ui::start_rendering_from_windows_startup(app.state::<AppState>())
                {
                    tracing::error!(%error, "could not start rendering during Windows startup");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run TelemetryForge");
}
