#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod preferences;
mod session;
mod settings_position;
mod state;
mod toolbar_position;
mod tray;
mod windows;

use commands::{
    action, activate_overlay, expand_toolbar, get_session, perform, register_toggle,
    report_history, set_preferences,
};
use preferences::Preferences;
use session::Session;
use state::{report, snapshot, AppState};
use std::sync::Mutex;
use tauri::Manager;
use tray::TrayAnchor;
use windows::{
    apply_windows, select_cursor_monitor,
    toolbar::{position_toolbar, watch_toolbar_proximity, ToolbarExpanded, ToolbarLayout},
};

type Result<T> = std::result::Result<T, String>;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .on_window_event(|window, event| {
            let settings = window.label() == "settings";
            let should_close = match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    true
                }
                tauri::WindowEvent::Focused(false) if settings => window
                    .app_handle()
                    .try_state::<AppState>()
                    .is_some_and(|state| state.0.lock().unwrap().settings_open),
                _ => false,
            };
            if should_close {
                if let Err(error) = perform(
                    window.app_handle(),
                    if settings {
                        "close-settings"
                    } else if window.label() == "tutorial" {
                        "dismiss-tutorial"
                    } else {
                        "hide"
                    },
                ) {
                    report(window.app_handle(), error);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_session,
            action,
            activate_overlay,
            report_history,
            set_preferences,
            expand_toolbar
        ])
        .setup(|app| {
            let preferences = Preferences::load(app.handle());
            app.manage(AppState(Mutex::new(Session::new(preferences))));
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                // The default Hide menu item consumes Cmd+H before the webview
                // can select Highlighter. Keep the other standard menu actions.
                let menu = tauri::menu::Menu::default(app.handle())?;
                let hide_label = tauri::menu::PredefinedMenuItem::hide(app, None)?.text()?;
                for item in menu.items()? {
                    if let Some(submenu) = item.as_submenu() {
                        for item in submenu.items()? {
                            if let Some(predefined) = item.as_predefined_menuitem() {
                                if predefined.text()? == hide_label {
                                    submenu.remove(predefined)?;
                                }
                            }
                        }
                    }
                }
                app.set_menu(menu)?;
            }
            app.manage(TrayAnchor(Mutex::new(None)));
            app.manage(ToolbarExpanded(Mutex::new(false)));
            app.manage(ToolbarLayout(Mutex::new(None)));
            windows::create_windows(app)?;
            tray::create_tray(app)?;
            let handle = app.handle();
            if let Err(e) = register_toggle(handle, &snapshot(handle).preferences.shortcut) {
                report(handle, e);
            }
            select_cursor_monitor(handle);
            position_toolbar(handle).map_err(std::io::Error::other)?;
            apply_windows(handle).map_err(std::io::Error::other)?;
            if snapshot(handle).tutorial.is_some() {
                if let Some(tutorial) = app.get_webview_window("tutorial") {
                    tutorial.set_focus()?;
                }
            }
            watch_toolbar_proximity(handle.clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Could not start Glassboard")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Err(error) = perform(app, "show") {
                    report(app, error);
                }
                if let Some(toolbar) = app.get_webview_window("toolbar") {
                    let _ = toolbar.set_focus();
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
