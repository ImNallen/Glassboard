#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings_position;
mod toolbar_position;

use serde::{Deserialize, Serialize};
use settings_position::{anchored_position, Bounds};
use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use toolbar_position::{dock_layout, ToolbarPointer, ToolbarPosition, TrackingLayout};

const SETTINGS_WIDTH: f64 = 360.0;
const SETTINGS_HEIGHT: f64 = 560.0;
const TRAY_ID: &str = "glassboard-tray";
/// Logical distance from the toolbar window within which the collapsed toolbar expands.
const TOOLBAR_REVEAL_MARGIN: f64 = 20.0;

const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+A";

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ColorMode {
    Solid,
    #[default]
    Rainbow,
    Cycle,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    tool: String,
    #[serde(deserialize_with = "deserialize_color")]
    color: String,
    #[serde(default)]
    color_mode: ColorMode,
    shortcut: String,
    #[serde(default)]
    toolbar_position: ToolbarPosition,
    #[serde(default)]
    auto_fade_seconds: u8,
    #[serde(default)]
    tutorial_completed: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            tool: "arrow".into(),
            color: "#f46b78".into(),
            color_mode: ColorMode::Rainbow,
            shortcut: DEFAULT_SHORTCUT.into(),
            toolbar_position: ToolbarPosition::Bottom,
            auto_fade_seconds: 0,
            tutorial_completed: false,
        }
    }
}
// Upgrade saved palette colors while retaining all other preferences.
fn deserialize_color<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    let color = String::deserialize(deserializer)?;
    Ok(match color.as_str() {
        "#efa5a5" | "#ff3355" => "#f46b78".into(),
        "#efb895" | "#ff8a1f" | "#e8cf91" | "#f5ff00" => "#f2c85b".into(),
        "#c6d99c" | "#a3ff12" | "#9fd5b5" | "#00f58a" => "#4dcaa0".into(),
        "#98d3cf" | "#00f0ff" => "#4fc5d5".into(),
        "#9fc5e8" | "#3388ff" => "#669df0".into(),
        "#afb5e5" | "#7855ff" | "#c9ace0" | "#c43cff" => "#a184e8".into(),
        "#e1a9cf" | "#ff33cc" => "#e580b5".into(),
        _ => color,
    })
}
impl Preferences {
    fn validate(&self) -> Result<Shortcut> {
        if ![
            "pen",
            "arrow",
            "rectangle",
            "ellipse",
            "highlighter",
            "text",
            "eraser",
        ]
        .contains(&self.tool.as_str())
            || ![0, 3, 5, 10].contains(&self.auto_fade_seconds)
            || self.color.len() != 7
            || !self.color.starts_with('#')
            || !self.color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        {
            return Err("Invalid drawing preferences".into());
        }
        let shortcut: Shortcut = self
            .shortcut
            .parse()
            .map_err(|e| format!("Invalid shortcut: {e}"))?;
        if !shortcut
            .mods
            .intersects(Modifiers::CONTROL | Modifiers::SUPER | Modifiers::ALT)
        {
            return Err("Include CommandOrControl, Control, Super, or Alt in the shortcut.".into());
        }
        Ok(shortcut)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum TutorialStep {
    Welcome,
    Draw,
    Hide,
    Done,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    mode: String,
    annotation_session: u32,
    tutorial: Option<TutorialStep>,
    settings_open: bool,
    active_overlay: String,
    cycle_index: u32,
    history_by_overlay: HashMap<String, HistoryAvailability>,
    preferences: Preferences,
    error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryAvailability {
    can_undo: bool,
    can_redo: bool,
}
struct AppState(Mutex<Session>);
struct TrayAnchor(Mutex<Option<tauri::Rect>>);
struct ToolbarExpanded(Mutex<bool>);
/// Physical window and dock bounds for focus-independent cursor tracking.
struct ToolbarLayout(Mutex<Option<TrackingLayout>>);

impl Session {
    fn new(preferences: Preferences) -> Self {
        Self {
            mode: "hidden".into(),
            annotation_session: 0,
            tutorial: if preferences.tutorial_completed {
                None
            } else {
                Some(TutorialStep::Welcome)
            },
            settings_open: false,
            active_overlay: "overlay-0".into(),
            cycle_index: 0,
            history_by_overlay: HashMap::new(),
            preferences,
            error: None,
        }
    }
    fn record_history(
        &mut self,
        overlay: &str,
        availability: HistoryAvailability,
        annotation_session: u32,
        advance_cycle: bool,
    ) {
        if annotation_session != self.annotation_session {
            return;
        }
        if self.tutorial == Some(TutorialStep::Draw) && availability.can_undo {
            self.tutorial = Some(TutorialStep::Hide);
        }
        self.history_by_overlay.insert(overlay.into(), availability);
        if advance_cycle {
            self.cycle_index = self.cycle_index.wrapping_add(1);
        }
    }
    fn transition(&mut self, action: &str) -> Result<()> {
        let previous_mode = self.mode.clone();
        match action {
            "toggle" => {
                self.mode = if self.mode == "hidden" {
                    "draw"
                } else {
                    "hidden"
                }
                .into();
            }
            "show" => self.mode = "draw".into(),
            "hide" => self.mode = "hidden".into(),
            "tutorial-start" => {
                self.mode = "draw".into();
                self.tutorial = Some(TutorialStep::Draw);
            }
            "replay-tutorial" => {
                self.mode = "hidden".into();
                self.tutorial = Some(TutorialStep::Welcome);
            }
            "dismiss-tutorial" => self.tutorial = None,
            "settings" => self.settings_open = true,
            "close-settings" => self.settings_open = false,
            "dismiss-error" => self.error = None,
            _ => return Err("Unknown action".into()),
        }
        if previous_mode == "draw" && self.mode == "hidden" {
            self.annotation_session += 1;
            self.history_by_overlay.clear();
        }
        if previous_mode == "hidden"
            && self.mode == "draw"
            && self.tutorial == Some(TutorialStep::Welcome)
        {
            self.tutorial = Some(TutorialStep::Draw);
        }
        if previous_mode == "draw" && self.mode == "hidden" && action != "replay-tutorial" {
            self.tutorial = match self.tutorial {
                Some(TutorialStep::Hide) => Some(TutorialStep::Done),
                Some(TutorialStep::Draw) => Some(TutorialStep::Welcome),
                other => other,
            };
        }
        if matches!(
            action,
            "toggle" | "show" | "hide" | "tutorial-start" | "replay-tutorial"
        ) {
            self.settings_open = false;
        }
        Ok(())
    }
}

type Result<T> = std::result::Result<T, String>;
fn snapshot(app: &tauri::AppHandle) -> Session {
    app.state::<AppState>().0.lock().unwrap().clone()
}
fn publish(app: &tauri::AppHandle) -> Result<()> {
    app.emit("session", snapshot(app))
        .map_err(|e| e.to_string())
}
fn report(app: &tauri::AppHandle, error: String) {
    eprintln!("Glassboard: {error}");
    app.state::<AppState>().0.lock().unwrap().error = Some(error);
    let _ = publish(app);
}
fn apply_windows(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            window
                .set_ignore_cursor_events(state.mode != "draw")
                .map_err(|e| e.to_string())?;
            window
                .set_focusable(state.mode == "draw")
                .map_err(|e| e.to_string())?;
            if state.mode == "hidden" {
                window.hide()
            } else {
                window.show()
            }
            .map_err(|e| e.to_string())?;
        }
    }
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        if state.mode != "hidden" {
            toolbar.show()
        } else {
            toolbar.hide()
        }
        .map_err(|e| e.to_string())?;
    }
    if !state.settings_open {
        if let Some(settings) = app.get_webview_window("settings") {
            settings.hide().map_err(|e| e.to_string())?;
        }
    }
    sync_tutorial(app)?;
    publish(app)
}
/// Keep the small guide on the active display without covering the drawing toolbar.
fn sync_tutorial(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let Some(window) = app.get_webview_window("tutorial") {
        if state.tutorial.is_none() {
            return window.hide().map_err(|e| e.to_string());
        }
        if let Some(overlay) = app.get_webview_window(&state.active_overlay) {
            if let Some(monitor) = overlay.current_monitor().map_err(|e| e.to_string())? {
                let area = monitor.work_area();
                let scale = monitor.scale_factor();
                let width = (400.0 * scale).min(area.size.width as f64);
                let height = (330.0 * scale).min(area.size.height as f64);
                window
                    .set_size(PhysicalSize::new(width as u32, height as u32))
                    .map_err(|e| e.to_string())?;
                window
                    .set_position(PhysicalPosition::new(
                        area.position.x + ((area.size.width as f64 - width) / 2.0) as i32,
                        area.position.y
                            + (20.0 * scale).min((area.size.height as f64 - height).max(0.0))
                                as i32,
                    ))
                    .map_err(|e| e.to_string())?;
            }
        }
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn focus_drawing(app: &tauri::AppHandle) {
    let state = snapshot(app);
    if state.mode == "draw" {
        if let Some(window) = app.get_webview_window(&state.active_overlay) {
            let _ = window.set_focus();
        }
    }
    raise_toolbar(app);
}
fn raise_toolbar(app: &tauri::AppHandle) {
    // Windows places a newly focused overlay above its sibling topmost windows.
    #[cfg(target_os = "windows")]
    for label in ["toolbar", "tutorial"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_always_on_top(true);
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}
fn position_toolbar(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let (Some(overlay), Some(toolbar)) = (
        app.get_webview_window(&state.active_overlay),
        app.get_webview_window("toolbar"),
    ) {
        if let Some(monitor) = overlay.current_monitor().map_err(|e| e.to_string())? {
            let area = monitor.work_area();
            let work = Bounds {
                x: area.position.x as f64,
                y: area.position.y as f64,
                width: area.size.width as f64,
                height: area.size.height as f64,
            };
            let expanded = *app.state::<ToolbarExpanded>().0.lock().unwrap();
            let layout = dock_layout(
                state.preferences.toolbar_position,
                work,
                monitor.scale_factor(),
                expanded,
            );
            toolbar
                .set_position(PhysicalPosition::new(layout.x, layout.y))
                .map_err(|e| e.to_string())?;
            toolbar
                .set_size(PhysicalSize::new(layout.width, layout.height))
                .map_err(|e| e.to_string())?;
            // Popover space must not enlarge the cursor-proximity area: otherwise
            // moving away into the transparent part of the window keeps it open.
            let dock = dock_layout(
                state.preferences.toolbar_position,
                work,
                monitor.scale_factor(),
                false,
            );
            *app.state::<ToolbarLayout>().0.lock().unwrap() = Some(TrackingLayout {
                dock,
                window: layout,
                scale: monitor.scale_factor(),
            });
        }
    }
    Ok(())
}
/// Cursor position in toolbar CSS pixels, even when the canvas owns keyboard focus.
fn toolbar_pointer(app: &tauri::AppHandle) -> Option<ToolbarPointer> {
    if snapshot(app).mode == "hidden" {
        return None;
    }
    let layout = (*app.state::<ToolbarLayout>().0.lock().unwrap())?;
    let cursor = app.cursor_position().ok()?;
    layout.pointer_at(cursor.x, cursor.y, TOOLBAR_REVEAL_MARGIN)
}
/// Poll across native windows so hover does not depend on activating the webview.
fn watch_toolbar_proximity(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut tracked = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(80));
            let pointer = toolbar_pointer(&app);
            // Repeat nearby samples: revealing/resizing the toolbar can put a
            // button under a stationary cursor without generating pointerenter.
            if pointer.is_some() || tracked {
                let _ = app.emit_to("toolbar", "toolbar-pointer", pointer);
            }
            tracked = pointer.is_some();
        }
    });
}
fn position_settings(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<()> {
    let rect = app
        .tray_by_id(TRAY_ID)
        .and_then(|tray| tray.rect().ok().flatten())
        .or_else(|| *app.state::<TrayAnchor>().0.lock().unwrap());
    // Tray rectangles are physical coordinates on both macOS and Windows.
    let icon = rect.map(|rect| {
        let p = rect.position.to_physical::<f64>(1.0);
        let s = rect.size.to_physical::<f64>(1.0);
        Bounds {
            x: p.x,
            y: p.y,
            width: s.width,
            height: s.height,
        }
    });
    let monitor = icon
        .and_then(|icon| {
            app.monitor_from_point(icon.x + icon.width / 2.0, icon.y + icon.height / 2.0)
                .ok()
                .flatten()
        })
        .or(app.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or("No display is available for settings")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let work = Bounds {
        x: area.position.x as f64,
        y: area.position.y as f64,
        width: area.size.width as f64,
        height: area.size.height as f64,
    };
    let icon = icon.unwrap_or(Bounds {
        x: work.x + work.width - 24.0 * scale,
        y: if cfg!(target_os = "macos") {
            work.y
        } else {
            work.y + work.height
        },
        width: 0.0,
        height: 0.0,
    });
    let size = (
        (SETTINGS_WIDTH * scale).min((work.width - 8.0 * scale).max(scale)),
        (SETTINGS_HEIGHT * scale).min((work.height - 8.0 * scale).max(scale)),
    );
    let (x, y) = anchored_position(icon, work, size, scale);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(
            size.0.round() as u32,
            size.1.round() as u32,
        ))
        .map_err(|e| e.to_string())
}

fn select_cursor_monitor(app: &tauri::AppHandle) {
    if let Ok(cursor) = app.cursor_position() {
        for (label, window) in app.webview_windows() {
            if !label.starts_with("overlay-") {
                continue;
            }
            if let (Ok(p), Ok(s)) = (window.outer_position(), window.outer_size()) {
                if cursor.x >= p.x as f64
                    && cursor.y >= p.y as f64
                    && cursor.x < p.x as f64 + s.width as f64
                    && cursor.y < p.y as f64 + s.height as f64
                {
                    app.state::<AppState>().0.lock().unwrap().active_overlay = label;
                    break;
                }
            }
        }
    }
}
fn perform(app: &tauri::AppHandle, action: &str) -> Result<()> {
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
    if (action == "toggle" && snapshot(app).mode == "hidden")
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
fn get_session(app: tauri::AppHandle) -> Session {
    snapshot(&app)
}
#[tauri::command]
fn action(app: tauri::AppHandle, action: String) -> Result<()> {
    perform(&app, &action)
}
#[tauri::command]
fn activate_overlay(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<()> {
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
fn report_history(
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
fn expand_toolbar(
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
fn register_toggle(app: &tauri::AppHandle, shortcut: &str) -> Result<()> {
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
fn set_preferences(app: tauri::AppHandle, preferences: Preferences) -> Result<()> {
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
    let save = (|| -> Result<()> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(
            dir.join("preferences.json"),
            serde_json::to_vec_pretty(&preferences).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    })();
    if let Err(e) = save {
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

fn configure_overlay(window: &tauri::WebviewWindow, toolbar: bool) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
        let ptr = window.ns_window()?;
        // Tauri supplies a live NSWindow. Setup runs on the main thread.
        let native = unsafe { &*(ptr as *const NSWindow) };
        native.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        native.setLevel(if toolbar { 26 } else { 25 });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, toolbar);
    Ok(())
}
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
            let preferences = app
                .path()
                .app_config_dir()
                .ok()
                .and_then(|dir| std::fs::read(dir.join("preferences.json")).ok())
                .and_then(|bytes| serde_json::from_slice::<Preferences>(&bytes).ok())
                .filter(|preferences| preferences.validate().is_ok())
                .unwrap_or_default();
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
            let monitors = app.available_monitors()?;
            for (i, monitor) in monitors.iter().enumerate() {
                let window = WebviewWindowBuilder::new(
                    app,
                    format!("overlay-{i}"),
                    WebviewUrl::App("index.html?surface=overlay".into()),
                )
                .title("Glassboard annotations")
                // A stroke must start on the click that activates this window,
                // including after selecting a tool or moving to another display.
                .accept_first_mouse(true)
                .transparent(true)
                .decorations(false)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .visible(false)
                .build()?;
                window.set_size(PhysicalSize::new(
                    monitor.size().width,
                    monitor.size().height,
                ))?;
                window.set_position(*monitor.position())?;
                configure_overlay(&window, false)?;
            }
            let toolbar = WebviewWindowBuilder::new(
                app,
                "toolbar",
                WebviewUrl::App("index.html?surface=toolbar".into()),
            )
            .title("Glassboard")
            // Tool selection should also work on the first click from the canvas.
            .accept_first_mouse(true)
            .inner_size(744.0, 76.0)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .build()?;
            configure_overlay(&toolbar, true)?;
            let settings_window = WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::App("index.html?surface=settings".into()),
            )
            .title("Glassboard Settings")
            .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
            .accept_first_mouse(true)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .build()?;
            configure_overlay(&settings_window, true)?;
            let tutorial_window = WebviewWindowBuilder::new(
                app,
                "tutorial",
                WebviewUrl::App("index.html?surface=tutorial".into()),
            )
            .title("Welcome to Glassboard")
            .inner_size(400.0, 330.0)
            .accept_first_mouse(true)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .build()?;
            configure_overlay(&tutorial_window, true)?;
            use tauri::menu::{Menu, MenuItem};
            let clear = MenuItem::with_id(app, "clear", "Clear screen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Glassboard", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&clear, &quit])?;
            let mut pixels = vec![0u8; 22 * 22 * 4];
            for y in 3..19 {
                for x in 3..19 {
                    if x == y || x == y + 1 || (y == 3 && x > 9) || (x == 18 && y < 12) {
                        let i = (y * 22 + x) * 4;
                        pixels[i..i + 4].copy_from_slice(&[220, 220, 220, 255]);
                    }
                }
            }
            tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
                .icon(tauri::image::Image::new_owned(pixels, 22, 22))
                .icon_as_template(true)
                .tooltip("Glassboard")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
                    let open_settings = matches!(
                        &event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    );
                    let rect = match event {
                        TrayIconEvent::Click { rect, .. }
                        | TrayIconEvent::DoubleClick { rect, .. }
                        | TrayIconEvent::Enter { rect, .. }
                        | TrayIconEvent::Move { rect, .. }
                        | TrayIconEvent::Leave { rect, .. } => rect,
                        _ => return,
                    };
                    *tray.app_handle().state::<TrayAnchor>().0.lock().unwrap() = Some(rect);
                    if open_settings {
                        if let Err(error) = perform(tray.app_handle(), "settings") {
                            report(tray.app_handle(), error);
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    if let Err(e) = perform(app, event.id.as_ref()) {
                        report(app, e);
                    }
                })
                .build(app)?;
            let handle = app.handle();
            if let Err(e) = register_toggle(handle, &snapshot(handle).preferences.shortcut) {
                report(handle, e);
            }
            select_cursor_monitor(handle);
            position_toolbar(handle).map_err(std::io::Error::other)?;
            apply_windows(handle).map_err(std::io::Error::other)?;
            if snapshot(handle).tutorial.is_some() {
                tutorial_window.set_focus()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session::new(Preferences::default())
    }
    #[test]
    fn hide_and_restore_resets_input_mode() {
        let mut s = session();
        s.transition("show").unwrap();
        s.transition("hide").unwrap();
        s.transition("toggle").unwrap();
        assert_eq!(s.mode, "draw");
    }
    #[test]
    fn removed_toolbar_action_is_rejected() {
        assert!(session().transition("toolbar").is_err());
    }
    #[test]
    fn settings_can_open_while_annotations_stay_hidden() {
        let mut s = session();
        s.transition("hide").unwrap();
        s.transition("settings").unwrap();
        assert!(s.settings_open);
        assert_eq!(s.mode, "hidden");
        s.transition("close-settings").unwrap();
        assert!(!s.settings_open);
        assert_eq!(s.mode, "hidden");
        s.transition("settings").unwrap();
        s.transition("toggle").unwrap();
        assert!(!s.settings_open);
        assert_eq!(s.mode, "draw");
    }
    #[test]
    fn removed_interact_action_is_rejected() {
        assert!(session().transition("interact").is_err());
    }
    #[test]
    fn leaving_annotations_resets_every_display_before_reopening() {
        for exit in ["hide", "toggle", "replay-tutorial"] {
            let mut s = session();
            s.transition("show").unwrap();
            let generation = s.annotation_session;
            for overlay in ["overlay-0", "overlay-1"] {
                s.record_history(
                    overlay,
                    HistoryAvailability {
                        can_undo: true,
                        can_redo: true,
                    },
                    generation,
                    false,
                );
            }
            s.transition("show").unwrap();
            assert_eq!(s.annotation_session, generation);
            assert_eq!(s.history_by_overlay.len(), 2);
            s.transition(exit).unwrap();
            assert_eq!(s.mode, "hidden");
            assert_eq!(s.annotation_session, generation + 1);
            assert!(s.history_by_overlay.is_empty());
            s.record_history(
                "overlay-1",
                HistoryAvailability {
                    can_undo: true,
                    can_redo: true,
                },
                generation,
                true,
            );
            assert!(s.history_by_overlay.is_empty());
            assert_eq!(s.cycle_index, 0);
            s.transition("hide").unwrap();
            s.transition("show").unwrap();
            assert_eq!(s.annotation_session, generation + 1);
            assert!(s.history_by_overlay.is_empty());
        }
    }
    #[test]
    fn tutorial_follows_a_completed_mark_and_return_to_work() {
        let mut s = session();
        assert_eq!(s.mode, "hidden");
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
        s.transition("tutorial-start").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
        s.record_history(
            "overlay-0",
            HistoryAvailability {
                can_undo: false,
                can_redo: false,
            },
            s.annotation_session,
            false,
        );
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
        s.record_history(
            "overlay-0",
            HistoryAvailability {
                can_undo: true,
                can_redo: false,
            },
            s.annotation_session,
            false,
        );
        assert_eq!(s.tutorial, Some(TutorialStep::Hide));
        s.transition("hide").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Done));
        assert_eq!(s.mode, "hidden");
        s.transition("dismiss-tutorial").unwrap();
        assert_eq!(s.tutorial, None);
        s.transition("replay-tutorial").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
    }
    #[test]
    fn completed_tutorial_does_not_reappear_on_launch() {
        let preferences = Preferences {
            tutorial_completed: true,
            ..Preferences::default()
        };
        let saved = serde_json::to_vec(&preferences).unwrap();
        let s = Session::new(serde_json::from_slice(&saved).unwrap());
        assert_eq!(s.mode, "hidden");
        assert_eq!(s.tutorial, None);
        let mut old = serde_json::to_value(Preferences::default()).unwrap();
        old.as_object_mut().unwrap().remove("tutorialCompleted");
        assert_eq!(
            Session::new(serde_json::from_value(old).unwrap()).tutorial,
            Some(TutorialStep::Welcome)
        );
    }
    #[test]
    fn leaving_before_drawing_allows_retrying_the_tutorial() {
        let mut s = session();
        s.transition("toggle").unwrap();
        s.transition("hide").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
        s.transition("toggle").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
    }
    #[test]
    fn shortcut_validation_rejects_unmodified_and_malformed_keys() {
        let mut p = Preferences::default();
        assert!(p.validate().is_ok());
        for shortcut in ["A", "Shift+A", "not a shortcut", "CommandOrControl+Shift+"] {
            p.shortcut = shortcut.into();
            assert!(p.validate().is_err(), "{shortcut}");
        }
        // Recorded shortcuts use browser key codes, which the native parser accepts.
        for shortcut in [
            "CommandOrControl+Shift+KeyA",
            "Alt+Digit1",
            "Control+Space",
            "CommandOrControl+F5",
            "Alt+Shift+ArrowUp",
            "Super+Comma",
        ] {
            p.shortcut = shortcut.into();
            assert!(p.validate().is_ok(), "{shortcut}");
        }
    }
    #[test]
    fn auto_fade_preferences_are_saved_and_validated() {
        for seconds in [0, 3, 5, 10] {
            let preferences = Preferences {
                auto_fade_seconds: seconds,
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok());
            let saved = serde_json::to_string(&preferences).unwrap();
            let restored: Preferences = serde_json::from_str(&saved).unwrap();
            assert_eq!(restored.auto_fade_seconds, seconds);
        }
        let invalid = Preferences {
            auto_fade_seconds: 4,
            ..Preferences::default()
        };
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn every_tool_is_accepted_and_unknown_tools_are_rejected() {
        for tool in [
            "pen",
            "arrow",
            "rectangle",
            "ellipse",
            "highlighter",
            "text",
            "eraser",
        ] {
            let preferences = Preferences {
                tool: tool.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok(), "{tool}");
        }
        let invalid = Preferences {
            tool: "pencil".into(),
            ..Preferences::default()
        };
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn older_preferences_default_to_rainbow_and_keep_color_and_shortcut() {
        let preferences: Preferences = serde_json::from_str(
            r##"{"tool":"pen","color":"#68aaff","width":7,"shortcut":"CommandOrControl+Shift+B"}"##,
        )
        .unwrap();
        assert_eq!(preferences.color_mode, ColorMode::Rainbow);
        assert_eq!(preferences.toolbar_position, ToolbarPosition::Bottom);
        assert_eq!(preferences.tool, "pen");
        assert_eq!(preferences.auto_fade_seconds, 0);
        assert_eq!(preferences.color, "#68aaff");
        assert_eq!(preferences.shortcut, "CommandOrControl+Shift+B");
        // Retired line-width preferences are ignored without resetting other settings.
        assert!(serde_json::to_value(&preferences)
            .unwrap()
            .get("width")
            .is_none());
        assert!(preferences.validate().is_ok());
    }
    #[test]
    fn custom_colors_are_validated_and_preserved() {
        for color in ["#000000", "#ffffff", "#123aBC", "#ff6259"] {
            let preferences = Preferences {
                color: color.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok());
            let loaded: Preferences =
                serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
            assert_eq!(loaded.color, color);
        }
        for color in ["", "#", "#fff", "#12345678", "1234567", "#gggggg", "#é1234"] {
            let preferences = Preferences {
                color: color.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_err(), "{color}");
        }
    }
    #[test]
    fn toolbar_positions_survive_saving_and_loading() {
        for position in [
            ToolbarPosition::Left,
            ToolbarPosition::Right,
            ToolbarPosition::Bottom,
        ] {
            let preferences = Preferences {
                toolbar_position: position,
                ..Preferences::default()
            };
            let loaded: Preferences =
                serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
            assert_eq!(loaded.toolbar_position, position);
            assert!(loaded.validate().is_ok());
        }
    }
    #[test]
    fn color_modes_survive_saving_and_loading() {
        for mode in [ColorMode::Solid, ColorMode::Rainbow, ColorMode::Cycle] {
            let preferences = Preferences {
                color_mode: mode,
                ..Preferences::default()
            };
            let bytes = serde_json::to_vec(&preferences).unwrap();
            let loaded: Preferences = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(loaded.color_mode, mode);
            assert!(loaded.validate().is_ok());
        }
    }
}
