use crate::{
    preference_saves::PreferenceSaves,
    preferences::{Preferences, ToggleShortcut},
    session::{HistoryAvailability, Mode, Session, Transition, TutorialStep},
    state::{publish, report, snapshot, AppState},
    windows::{
        apply_windows, focus_drawing, raise_toolbar, select_cursor_monitor,
        settings::position_settings,
        sync_tutorial,
        toolbar::{position_toolbar, Toolbar},
        Surface,
    },
    Result,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_opener::OpenerExt;

pub(crate) const REPOSITORY_URL: &str = "https://github.com/ImNallen/Glassboard";

/// The `action` command vocabulary. Transitions appear on the wire flat, by their own names.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Action {
    Capture,
    Undo,
    Redo,
    Clear,
    ClearAll,
    OpenGithub,
    ReportIssue,
    CheckForUpdates,
    InstallUpdate,
    Quit,
    #[serde(untagged)]
    Session(Transition),
}

pub(crate) fn perform(app: &tauri::AppHandle, action: Action) -> Result<()> {
    // Fixed URLs keep the webviews from opening arbitrary links.
    let open = |url: String| -> Result<()> { Ok(app.opener().open_url(url, None::<&str>)?) };
    match action {
        Action::Capture => {
            crate::capture::start(app).inspect_err(|error| crate::capture::fail(app, error))
        }
        Action::OpenGithub => open(REPOSITORY_URL.into()),
        Action::ReportIssue => open(format!("{REPOSITORY_URL}/issues/new")),
        Action::CheckForUpdates => {
            crate::updates::check(app);
            Ok(())
        }
        Action::InstallUpdate => {
            crate::updates::install(app);
            Ok(())
        }
        Action::Quit => {
            app.exit(0);
            Ok(())
        }
        Action::Undo | Action::Redo | Action::Clear => Ok(app.emit_to(
            snapshot(app).active_overlay.label(),
            "drawing-action",
            action,
        )?),
        Action::ClearAll => Ok(app.emit("drawing-action", Action::Clear)?),
        Action::Session(session) => transition(app, session),
    }
}
pub(crate) fn transition(app: &tauri::AppHandle, transition: Transition) -> Result<()> {
    use Transition::*;
    if transition == DismissTutorial {
        let mut preferences = snapshot(app).preferences;
        preferences.tutorial_completed = true;
        update_preferences(app, preferences)?;
    }
    if matches!(transition, Settings | CloseSettings) {
        let settings = Surface::Settings
            .window(app)
            .ok_or("Settings window is unavailable")?;
        if transition == Settings {
            position_settings(app, &settings)?;
        }
        app.state::<AppState>()
            .0
            .lock()
            .unwrap()
            .transition(transition);
        apply_windows(app)?;
        if transition == Settings {
            settings.show()?;
            settings.set_focus()?;
        } else {
            settings.hide()?;
        }
        return Ok(());
    }
    if (transition == Toggle && snapshot(app).mode == Mode::Hidden)
        || matches!(transition, Show | TutorialStart | ReplayTutorial)
    {
        select_cursor_monitor(app);
        position_toolbar(app)?;
    }
    app.state::<AppState>()
        .0
        .lock()
        .unwrap()
        .transition(transition);
    apply_windows(app)?;
    if snapshot(app).tutorial == Some(TutorialStep::Done)
        && !snapshot(app).preferences.tutorial_completed
    {
        let mut preferences = snapshot(app).preferences;
        preferences.tutorial_completed = true;
        // Returning to work must still succeed if saving onboarding progress fails.
        if let Err(error) = update_preferences(app, preferences) {
            report(app, error);
        }
    }
    if matches!(transition, Toggle | Show | TutorialStart) {
        focus_drawing(app);
    }
    if matches!(
        snapshot(app).tutorial,
        Some(TutorialStep::Welcome | TutorialStep::Done)
    ) {
        if let Some(window) = Surface::Tutorial.window(app) {
            window.set_focus()?;
        }
    }
    Ok(())
}
#[tauri::command]
pub(crate) fn get_session(app: tauri::AppHandle) -> Session {
    snapshot(&app)
}
#[tauri::command]
pub(crate) fn action(app: tauri::AppHandle, action: Action) -> Result<()> {
    perform(&app, action)
}
#[tauri::command]
pub(crate) fn activate_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<()> {
    if let Some(overlay @ Surface::Overlay(_)) = Surface::parse(window.label()) {
        let changed = {
            let state = app.state::<AppState>();
            let mut state = state.0.lock().unwrap();
            let changed = state.active_overlay != overlay;
            state.active_overlay = overlay;
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
    let Some(surface @ (Surface::Overlay(_) | Surface::Capture)) = Surface::parse(window.label())
    else {
        return Err("Only drawing windows can report history".into());
    };
    app.state::<AppState>().0.lock().unwrap().record_history(
        surface,
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
    if Surface::parse(window.label()) != Some(Surface::Toolbar) {
        return Err("Only the toolbar can resize itself".into());
    }
    app.state::<Toolbar>().set_expanded(expanded);
    position_toolbar(&app)
}
/// Whether Glassboard opens at login. The OS holds this setting, so it can change outside the app.
#[tauri::command]
pub(crate) fn get_autostart(app: tauri::AppHandle) -> Result<bool> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("Could not read the login item: {e}").into())
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
        .map_err(|e| format!("Could not read the login item: {e}").into())
}
pub(crate) fn register_toggle(app: &tauri::AppHandle, shortcut: &ToggleShortcut) -> Result<()> {
    app.global_shortcut()
        .on_shortcut(shortcut.parsed(), |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                if let Err(e) = transition(app, Transition::Toggle) {
                    report(app, e);
                }
            }
        })
        .map_err(|e| format!("Could not register {}: {e}", shortcut.as_str()).into())
}
/// Parsed here rather than by Tauri, which would prefix the messages Settings shows.
#[tauri::command]
pub(crate) fn set_preferences(app: tauri::AppHandle, preferences: serde_json::Value) -> Result<()> {
    update_preferences(&app, Preferences::parse(preferences)?)
}
pub(crate) fn update_preferences(app: &tauri::AppHandle, preferences: Preferences) -> Result<()> {
    let old = snapshot(app).preferences;
    if preferences == old {
        return Ok(());
    }
    let parsed = preferences.shortcut.parsed();
    let old_parsed = old.shortcut.parsed();
    let changed = parsed != old_parsed;
    if changed {
        register_toggle(app, &preferences.shortcut)?;
    }
    let saves = app.state::<PreferenceSaves>();
    if changed {
        if let Err(e) = saves.save_now(preferences.clone()) {
            let _ = app.global_shortcut().unregister(parsed);
            return Err(format!("Could not save preferences: {e}").into());
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
                report(app, format!("Could not restore preferences: {error}"));
            }
            return Err(error.into());
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
        position_toolbar(app)?;
    }
    publish(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actions_keep_their_wire_names() {
        let parse = |name: &str| serde_json::from_value::<Action>(name.into());
        assert_eq!(parse("clear-all").unwrap(), Action::ClearAll);
        assert_eq!(parse("check-for-updates").unwrap(), Action::CheckForUpdates);
        assert_eq!(parse("install-update").unwrap(), Action::InstallUpdate);
        assert_eq!(
            parse("cancel-capture").unwrap(),
            Action::Session(Transition::CancelCapture)
        );
        assert!(parse("interact").is_err());
        assert_eq!(serde_json::to_value(Action::Clear).unwrap(), "clear");
    }
}
