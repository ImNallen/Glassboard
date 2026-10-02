use crate::{
    preference_saves::PreferenceSaves,
    preferences::Preferences,
    session::{HistoryAvailability, Mode, Session, TutorialStep},
    state::{publish, report, snapshot, AppState},
    windows::{
        apply_windows, focus_drawing, raise_toolbar, select_cursor_monitor,
        settings::position_settings,
        sync_tutorial,
        toolbar::{position_toolbar, ToolbarExpanded},
    },
    Result,
};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;

pub(crate) const REPOSITORY_URL: &str = "https://github.com/ImNallen/Glassboard";

pub(crate) fn perform(app: &tauri::AppHandle, action: &str) -> Result<()> {
    if action == "capture" {
        return crate::capture::start(app).inspect_err(|error| {
            let _ = perform(app, "settings");
            // The caller can be a hidden toolbar webview; surface the error in
            // the shared session so the visible settings window explains it.
            report(app, error.clone());
        });
    }
    if action == "dismiss-tutorial" {
        let mut preferences = snapshot(app).preferences;
        preferences.tutorial_completed = true;
        set_preferences(app.clone(), preferences)?;
    }
    // Fixed URLs keep the webviews from opening arbitrary links.
    let link = match action {
        "open-github" => Some(REPOSITORY_URL.to_string()),
        "report-issue" => Some(format!("{REPOSITORY_URL}/issues/new")),
        _ => None,
    };
    if let Some(url) = link {
        return app
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| e.to_string());
    }
    if action == "quit" {
        app.exit(0);
        return Ok(());
    }
    if ["undo", "redo", "clear"].contains(&action) {
        let target = snapshot(app).active_overlay;
        return app
            .emit_to(target, "drawing-action", action)
            .map_err(|e| e.to_string());
    }
    if action == "clear-all" {
        return app
            .emit("drawing-action", "clear")
            .map_err(|e| e.to_string());
    }
    if matches!(action, "settings" | "close-settings") {
        let settings = app
            .get_webview_window("settings")
            .ok_or("Settings window is unavailable")?;
        if action == "settings" {
            position_settings(app, &settings)?;
        }
        app.state::<AppState>()
            .0
            .lock()
            .unwrap()
            .transition(action)?;
        apply_windows(app)?;
        if action == "settings" {
            settings.show().map_err(|e| e.to_string())?;
            settings.set_focus().map_err(|e| e.to_string())?;
        } else {
            settings.hide().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if (action == "toggle" && snapshot(app).mode == Mode::Hidden)
        || matches!(action, "show" | "tutorial-start" | "replay-tutorial")
    {
        select_cursor_monitor(app);
        position_toolbar(app)?;
    }
    app.state::<AppState>()
        .0
        .lock()
        .unwrap()
        .transition(action)?;
    apply_windows(app)?;
    if snapshot(app).tutorial == Some(TutorialStep::Done)
        && !snapshot(app).preferences.tutorial_completed
    {
        let mut preferences = snapshot(app).preferences;
        preferences.tutorial_completed = true;
        // Returning to work must still succeed if saving onboarding progress fails.
        if let Err(error) = set_preferences(app.clone(), preferences) {
            report(app, error);
        }
    }
    if matches!(action, "toggle" | "show" | "tutorial-start") {
        focus_drawing(app);
    }
    if matches!(
        snapshot(app).tutorial,
        Some(TutorialStep::Welcome | TutorialStep::Done)
    ) {
        if let Some(window) = app.get_webview_window("tutorial") {
            window.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn get_session(app: tauri::AppHandle) -> Session {
    snapshot(&app)
}
#[tauri::command]
pub(crate) fn action(app: tauri::AppHandle, action: String) -> Result<()> {
    perform(&app, &action)
}
#[tauri::command]
pub(crate) fn activate_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<()> {
    if window.label().starts_with("overlay-") {
        let changed = {
            let state = app.state::<AppState>();
            let mut state = state.0.lock().unwrap();
            let changed = state.active_overlay != window.label();
            state.active_overlay = window.label().into();
            changed
        };
        if changed {
            position_toolbar(&app)?;
            sync_tutorial(&app)?;
        }
        raise_toolbar(&app);
        publish(&app)?;
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn report_history(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    availability: HistoryAvailability,
    advance_cycle: bool,
    annotation_session: u32,
) -> Result<()> {
    if !window.label().starts_with("overlay-") && window.label() != "capture" {
        return Err("Only drawing windows can report history".into());
    }
    app.state::<AppState>().0.lock().unwrap().record_history(
        window.label(),
        availability,
        annotation_session,
        advance_cycle,
    );
    publish(&app)
}
#[tauri::command]
pub(crate) fn expand_toolbar(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    expanded: bool,
) -> Result<()> {
    if window.label() != "toolbar" {
        return Err("Only the toolbar can resize itself".into());
    }
    *app.state::<ToolbarExpanded>().0.lock().unwrap() = expanded;
    position_toolbar(&app)
}
/// Whether Glassboard opens at login. The OS holds this setting, so it can change outside the app.
#[tauri::command]
pub(crate) fn get_autostart(app: tauri::AppHandle) -> Result<bool> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Could not read the login item: {e}"))
}
#[tauri::command]
pub(crate) fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    }
    .map_err(|e| format!("Could not change the login item: {e}"))?;
    launcher
        .is_enabled()
        .map_err(|e| format!("Could not read the login item: {e}"))
}
pub(crate) fn register_toggle(app: &tauri::AppHandle, shortcut: &str) -> Result<()> {
    let parsed: Shortcut = shortcut
        .parse()
        .map_err(|e| format!("Invalid shortcut: {e}"))?;
    app.global_shortcut()
        .on_shortcut(parsed, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                if let Err(e) = perform(app, "toggle") {
                    report(app, e);
                }
            }
        })
        .map_err(|e| format!("Could not register {shortcut}: {e}"))
}
#[tauri::command]
pub(crate) fn set_preferences(app: tauri::AppHandle, preferences: Preferences) -> Result<()> {
    let parsed = preferences.validate()?;
    let old = snapshot(&app).preferences;
    if preferences == old {
        return Ok(());
    }
    let old_parsed: Shortcut = old
        .shortcut
        .parse()
        .map_err(|e| format!("Invalid saved shortcut: {e}"))?;
    let changed = parsed != old_parsed;
    if changed {
        register_toggle(&app, &preferences.shortcut)?;
    }
    let saves = app.state::<PreferenceSaves>();
    if changed {
        if let Err(e) = saves.save_now(preferences.clone()) {
            let _ = app.global_shortcut().unregister(parsed);
            return Err(format!("Could not save preferences: {e}"));
        }
        // A saved shortcut that another app already held at launch was never
        // registered, and Windows refuses to unregister it. Skip it so the user
        // can always move to a shortcut that works.
        let unregistered = if app.global_shortcut().is_registered(old_parsed) {
            app.global_shortcut().unregister(old_parsed)
        } else {
            Ok(())
        };
        if let Err(error) = unregistered {
            let _ = app.global_shortcut().unregister(parsed);
            if let Err(error) = saves.save_now(old.clone()) {
                report(&app, format!("Could not restore preferences: {error}"));
            }
            return Err(error.to_string());
        }
    }
    let reposition = old.toolbar_position != preferences.toolbar_position;
    {
        let state = app.state::<AppState>();
        let mut state = state.0.lock().unwrap();
        state.preferences = preferences.clone();
        if changed {
            // The new shortcut registered above, so the toggle works again.
            state.shortcut_unavailable = false;
        }
    }
    if !changed {
        saves.queue(preferences)?;
    }
    if reposition {
        position_toolbar(&app)?;
    }
    publish(&app)
}
