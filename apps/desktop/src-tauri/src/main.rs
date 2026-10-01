#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod capture_screen;
mod commands;
mod preference_saves;
mod preferences;
mod session;
mod settings_position;
mod state;
mod toolbar_position;
mod tray;
mod windows;

use capture::{copy_capture, copy_capture_region, get_capture_image, CaptureImage};
use commands::{
    action, activate_overlay, expand_toolbar, get_autostart, get_session, perform, register_toggle,
    report_history, set_autostart, set_preferences,
};
use preference_saves::PreferenceSaves;
use preferences::Preferences;
use session::Session;
use state::{report, snapshot, AppState};
use std::sync::Mutex;
use tauri::Manager;
use tray::TrayAnchor;
use windows::{
    apply_windows, select_cursor_monitor,
    toolbar::{
        position_toolbar, watch_toolbar_proximity, ToolbarExpanded, ToolbarLayout, ToolbarTracking,
    },
};

type Result<T> = std::result::Result<T, String>;

fn main() {
    tauri::Builder::default()
        // Registered first so a second launch exits before creating windows,
        // a tray icon, or a competing global shortcut.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || reopen(&handle));
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        // Login items use a macOS Launch Agent and the Windows Run registry key.
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .on_window_event(|window, event| {
            // Native activation (including clicking another display) can raise a
            // transparent drawing window above the toolbar before a drawing command.
            #[cfg(target_os = "windows")]
            if window.label().starts_with("overlay-")
                && matches!(event, tauri::WindowEvent::Focused(true))
            {
                windows::raise_toolbar(window.app_handle());
            }
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
                    } else if window.label() == "capture" {
                        "cancel-capture"
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
            get_autostart,
            set_autostart,
            expand_toolbar,
            get_capture_image,
            copy_capture,
            copy_capture_region
        ])
        .setup(|app| {
            let preferences = Preferences::load(app.handle());
            app.manage(AppState(Mutex::new(Session::new(preferences))));
            app.manage(PreferenceSaves::start(app.handle().clone()));
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
            app.manage(CaptureImage(Mutex::new(None)));
            app.manage(TrayAnchor(Mutex::new(None)));
            app.manage(ToolbarExpanded(Mutex::new(false)));
            app.manage(ToolbarLayout(Mutex::new(None)));
            app.manage(ToolbarTracking::default());
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
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                if let Err(error) = app.state::<PreferenceSaves>().flush() {
                    eprintln!("Could not save preferences on exit: {error}");
                }
            }
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                reopen(app);
            }
        });
}

// Reopening the app (from the Dock, or by launching it again while it runs)
// must preserve an unfinished capture or its permission/error explanation,
// and an open tutorial step, rather than switch to live drawing.
fn reopen(app: &tauri::AppHandle) {
    let session = snapshot(app);
    let target = if session.capture.is_some() {
        "capture"
    } else if session.settings_open {
        "settings"
    } else if session.tutorial.is_some() {
        "tutorial"
    } else {
        if let Err(error) = perform(app, "show") {
            report(app, error);
        }
        "toolbar"
    };
    if let Some(window) = app.get_webview_window(target) {
        let _ = window.set_focus();
    }
}
