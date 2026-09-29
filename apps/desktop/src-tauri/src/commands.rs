use crate::{
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
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub(crate) fn perform(app: &tauri::AppHandle, action: &str) -> Result<()> {
    if action == "dismiss-tutorial" {
        let mut preferences = snapshot(app).preferences;
        preferences.tutorial_completed = true;
        set_preferences(app.clone(), preferences)?;
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
        publish(app)?;
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
    if !window.label().starts_with("overlay-") {
        return Err("Only annotation windows can report drawing history".into());
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
    let old_parsed: Shortcut = old
        .shortcut
        .parse()
        .map_err(|e| format!("Invalid saved shortcut: {e}"))?;
    let changed = parsed != old_parsed;
    if changed {
        register_toggle(&app, &preferences.shortcut)?;
    }
    if let Err(e) = preferences.save(&app) {
        if changed {
            let _ = app.global_shortcut().unregister(parsed);
        }
        return Err(format!("Could not save preferences: {e}"));
    }
    if changed {
        app.global_shortcut()
            .unregister(old_parsed)
            .map_err(|e| e.to_string())?;
    }
    let reposition = old.toolbar_position != preferences.toolbar_position;
    app.state::<AppState>().0.lock().unwrap().preferences = preferences;
    if reposition {
        position_toolbar(&app)?;
    }
    publish(&app)
}
