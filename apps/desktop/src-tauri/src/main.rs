#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
mod autostart;
mod capture;
mod commands;
mod error;
mod preference_saves;
mod preferences;
mod session;
mod startup_error;
mod state;
mod tray;
mod updates;
mod windows;

use capture::{copy_capture, copy_capture_region, get_capture_image};
use commands::{
    action, activate_overlay, expand_toolbar, get_autostart, get_session, register_toggle,
    report_history, set_autostart, set_preferences, transition,
};
use preference_saves::PreferenceSaves;
use preferences::Preferences;
use session::{Session, Transition};
use state::{report, snapshot, AppState};
use std::sync::Mutex;
use tauri::Manager;
use tray::TrayAnchor;
use windows::{
    apply_windows, select_cursor_monitor,
    toolbar::{position_toolbar, watch_toolbar_proximity, Toolbar},
    Surface,
};

use error::Result;

fn main() {
    // Release builds have no console, so record panics in the log before the
    // default hook prints them and the process unwinds or aborts.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("{info}\n{}", std::backtrace::Backtrace::force_capture());
        default_hook(info);
    }));
    let app = tauri::Builder::default()
        // Registered first so a second launch exits before creating windows,
        // a tray icon, or a competing global shortcut.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || reopen(&handle));
        }))
        // Writes to ~/Library/Logs/dev.glassboard.desktop on macOS and
        // %LOCALAPPDATA%\dev.glassboard.desktop\logs on Windows.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        // Login items use a macOS Launch Agent and the Windows Run registry key.
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            // Native activation (including clicking another display) can raise a
            // transparent drawing window above the toolbar before a drawing command.
            let surface = Surface::parse(window.label());
            #[cfg(target_os = "windows")]
            if matches!(surface, Some(Surface::Overlay(_)))
                && matches!(event, tauri::WindowEvent::Focused(true))
            {
                windows::raise_toolbar(window.app_handle());
            }
            let settings = surface == Some(Surface::Settings);
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
                if let Err(error) = transition(
                    window.app_handle(),
                    match surface {
                        Some(Surface::Settings) => Transition::CloseSettings,
                        Some(Surface::Capture) => Transition::CancelCapture,
                        Some(Surface::Tutorial) => Transition::DismissTutorial,
                        Some(Surface::Toolbar | Surface::Overlay(_)) | None => Transition::Hide,
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
            copy_capture_region,
            updates::get_update
        ])
        .setup(|app| {
            if let Err(error) = setup(app) {
                // Tauri panics on a setup error, which closes the app silently.
                startup_error::exit(&error.to_string());
            }
            Ok(())
        })
        .build(tauri::generate_context!());
    let app = app.unwrap_or_else(|error| startup_error::exit(&error.to_string()));
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            if let Err(error) = app.state::<PreferenceSaves>().flush() {
                log::error!("Could not save preferences on exit: {error}");
            }
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            reopen(app);
        }
    });
}

fn setup(app: &mut tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    log::info!(
        "Starting Glassboard {} on {} {}",
        app.package_info().version,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    // Problems found while starting are shown together once the windows exist.
    let mut problems = Vec::new();
    #[cfg(target_os = "macos")]
    problems.extend(
        commands::associate_login_item(app.handle())
            .err()
            .map(|e| e.to_string()),
    );
    let (preferences, warning) = Preferences::load(app.handle());
    problems.extend(warning);
    app.manage(AppState(Mutex::new(Session::new(preferences))));
    app.manage(PreferenceSaves::start(app.handle().clone()));
    updates::start(app);
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
    app.manage(Toolbar::default());
    windows::create_windows(app)?;
    tray::create_tray(app)?;
    let handle = app.handle();
    if let Err(error) = register_toggle(handle, &snapshot(handle).preferences.shortcut) {
        log::error!("{error}");
        app.state::<AppState>()
            .0
            .lock()
            .unwrap()
            .shortcut_unavailable = true;
    }
    select_cursor_monitor(handle);
    // The app still works with a misplaced window, so these are not fatal.
    problems.extend(position_toolbar(handle).err().map(|e| e.to_string()));
    problems.extend(apply_windows(handle).err().map(|e| e.to_string()));
    if snapshot(handle).tutorial.is_some() {
        if let Some(tutorial) = Surface::Tutorial.window(app) {
            problems.extend(tutorial.set_focus().err().map(|e| e.to_string()));
        }
    }
    let session = snapshot(handle);
    // The tutorial explains problems itself. Otherwise nothing is on screen
    // at launch, so open Settings to explain them.
    if (!problems.is_empty() || session.shortcut_unavailable) && session.tutorial.is_none() {
        if let Err(error) = transition(handle, Transition::Settings) {
            log::error!("Could not open settings: {error}");
        }
    }
    if !problems.is_empty() {
        report(handle, problems.join(" "));
    }
    watch_toolbar_proximity(handle.clone());
    Ok(())
}

// Reopening the app (from the Dock, or by launching it again while it runs)
// must preserve an unfinished capture or its permission/error explanation,
// and an open tutorial step, rather than switch to live drawing.
fn reopen(app: &tauri::AppHandle) {
    let session = snapshot(app);
    let target = if session.capture.is_some() {
        Surface::Capture
    } else if session.settings_open {
        Surface::Settings
    } else if session.tutorial.is_some() {
        Surface::Tutorial
    } else {
        if let Err(error) = transition(app, Transition::Show) {
            report(app, error);
        }
        Surface::Toolbar
    };
    if let Some(window) = target.window(app) {
        let _ = window.set_focus();
    }
}
