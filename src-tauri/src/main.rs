#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings_position;
mod toolbar_position;

use serde::{Deserialize, Serialize};
use settings_position::{anchored_position, Bounds};
use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use toolbar_position::{dock_layout, ToolbarPosition};

const SETTINGS_WIDTH: f64 = 360.0;
const SETTINGS_HEIGHT: f64 = 560.0;
const TRAY_ID: &str = "glassboard-tray";

const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+A";
const INTERACT_SHORTCUT: &str = "CommandOrControl+Shift+I";
const TOOLBAR_SHORTCUT: &str = "CommandOrControl+Shift+H";

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
    #[serde(deserialize_with = "deserialize_tool")]
    tool: String,
    #[serde(deserialize_with = "deserialize_color")]
    color: String,
    #[serde(default)]
    color_mode: ColorMode,
    width: f64,
    shortcut: String,
    #[serde(default)]
    toolbar_position: ToolbarPosition,
    #[serde(default)]
    auto_fade_seconds: u8,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            tool: "arrow".into(),
            color: "#f46b78".into(),
            color_mode: ColorMode::Rainbow,
            width: 4.0,
            shortcut: DEFAULT_SHORTCUT.into(),
            toolbar_position: ToolbarPosition::Bottom,
            auto_fade_seconds: 0,
        }
    }
}
// Keep the remaining preferences when upgrading from the removed pencil tool.
fn deserialize_tool<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    let tool = String::deserialize(deserializer)?;
    Ok(if tool == "pen" { "arrow".into() } else { tool })
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
        if !["arrow", "rectangle", "ellipse", "highlighter"].contains(&self.tool.as_str())
            || ![0, 3, 5, 10].contains(&self.auto_fade_seconds)
            || ![2.0, 4.0, 7.0].contains(&self.width)
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
        if shortcut == INTERACT_SHORTCUT.parse::<Shortcut>().unwrap()
            || shortcut == TOOLBAR_SHORTCUT.parse::<Shortcut>().unwrap()
        {
            return Err("That shortcut is used by Interact or Hide Toolbar.".into());
        }
        Ok(shortcut)
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    mode: String,
    toolbar_visible: bool,
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

impl Session {
    fn transition_from_settings(&mut self, action: &str) -> Result<()> {
        let was_open = self.settings_open;
        self.transition(action)?;
        self.settings_open = was_open;
        Ok(())
    }
    fn transition(&mut self, action: &str) -> Result<()> {
        match action {
            "toggle" => {
                self.mode = if self.mode == "hidden" {
                    "draw"
                } else {
                    "hidden"
                }
                .into();
                self.toolbar_visible = true;
            }
            "show" => {
                self.mode = "draw".into();
                self.toolbar_visible = true;
            }
            "hide" => self.mode = "hidden".into(),
            "interact" if self.mode != "hidden" => {
                self.mode = if self.mode == "draw" {
                    "interact"
                } else {
                    "draw"
                }
                .into()
            }
            "toolbar" if self.mode != "hidden" => self.toolbar_visible = !self.toolbar_visible,
            "interact" | "toolbar" => {}
            "settings" => self.settings_open = true,
            "close-settings" => self.settings_open = false,
            "dismiss-error" => self.error = None,
            _ => return Err("Unknown action".into()),
        }
        if matches!(action, "toggle" | "show" | "hide" | "toolbar") {
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
        if state.mode != "hidden" && state.toolbar_visible {
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
    publish(app)
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
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        let _ = toolbar.set_always_on_top(true);
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
        }
    }
    Ok(())
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
    perform_action(app, action, false)
}
fn perform_action(app: &tauri::AppHandle, action: &str, keep_settings_open: bool) -> Result<()> {
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
    if action == "erase" {
        if snapshot(app).mode != "draw" {
            return Ok(());
        }
        // Only the overlay currently under the pointer handles this event.
        return app
            .emit("drawing-action", "erase")
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
    if (action == "toggle" && snapshot(app).mode == "hidden") || action == "show" {
        select_cursor_monitor(app);
        position_toolbar(app)?;
    }
    {
        let state = app.state::<AppState>();
        let mut s = state.0.lock().unwrap();
        if keep_settings_open {
            s.transition_from_settings(action)?;
        } else {
            s.transition(action)?;
        }
    }
    apply_windows(app)?;
    if keep_settings_open {
        if let Some(settings) = app.get_webview_window("settings") {
            settings.set_focus().map_err(|e| e.to_string())?;
        }
    } else if matches!(action, "toggle" | "show" | "interact") {
        focus_drawing(app);
    }
    Ok(())
}
#[tauri::command]
fn get_session(app: tauri::AppHandle) -> Session {
    snapshot(&app)
}
#[tauri::command]
fn action(app: tauri::AppHandle, window: tauri::WebviewWindow, action: String) -> Result<()> {
    let keep_settings_open = window.label() == "settings"
        && matches!(
            action.as_str(),
            "toggle" | "show" | "hide" | "interact" | "toolbar"
        );
    perform_action(&app, &action, keep_settings_open)
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
) -> Result<()> {
    if !window.label().starts_with("overlay-") {
        return Err("Only annotation windows can report drawing history".into());
    }
    {
        let state = app.state::<AppState>();
        let mut state = state.0.lock().unwrap();
        state
            .history_by_overlay
            .insert(window.label().into(), availability);
        if advance_cycle {
            state.cycle_index = state.cycle_index.wrapping_add(1);
        }
    }
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
                    if settings { "close-settings" } else { "hide" },
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
            app.manage(AppState(Mutex::new(Session {
                mode: "draw".into(),
                toolbar_visible: true,
                settings_open: false,
                active_overlay: "overlay-0".into(),
                cycle_index: 0,
                history_by_overlay: HashMap::new(),
                preferences,
                error: None,
            })));
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            app.manage(TrayAnchor(Mutex::new(None)));
            app.manage(ToolbarExpanded(Mutex::new(false)));
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
            .inner_size(638.0, 76.0)
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
            for (shortcut, action) in [
                (INTERACT_SHORTCUT, "interact"),
                (TOOLBAR_SHORTCUT, "toolbar"),
            ] {
                if let Err(e) =
                    handle
                        .global_shortcut()
                        .on_shortcut(shortcut, move |app, _, event| {
                            if event.state() == ShortcutState::Pressed {
                                if let Err(e) = perform(app, action) {
                                    report(app, e);
                                }
                            }
                        })
                {
                    report(handle, e.to_string());
                }
            }
            select_cursor_monitor(handle);
            position_toolbar(handle).map_err(std::io::Error::other)?;
            apply_windows(handle).map_err(std::io::Error::other)?;
            toolbar.set_focus()?;
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
        Session {
            mode: "draw".into(),
            toolbar_visible: true,
            settings_open: false,
            active_overlay: "overlay-0".into(),
            cycle_index: 0,
            history_by_overlay: HashMap::new(),
            preferences: Preferences::default(),
            error: None,
        }
    }
    #[test]
    fn hide_and_restore_resets_input_mode_and_restores_toolbar() {
        let mut s = session();
        s.transition("interact").unwrap();
        s.transition("toolbar").unwrap();
        s.transition("hide").unwrap();
        s.transition("toggle").unwrap();
        assert_eq!(s.mode, "draw");
        assert!(s.toolbar_visible);
    }
    #[test]
    fn settings_can_open_while_annotations_stay_hidden() {
        let mut s = session();
        s.transition("hide").unwrap();
        s.toolbar_visible = false;
        s.transition("settings").unwrap();
        assert!(s.settings_open);
        assert!(!s.toolbar_visible);
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
    fn settings_controls_change_modes_without_dismissing_settings() {
        let mut s = session();
        s.transition("settings").unwrap();
        s.transition_from_settings("toggle").unwrap();
        assert_eq!(s.mode, "hidden");
        assert!(s.settings_open);
        s.transition_from_settings("toggle").unwrap();
        assert_eq!(s.mode, "draw");
        s.transition_from_settings("interact").unwrap();
        assert_eq!(s.mode, "interact");
        assert!(s.settings_open);
        s.transition_from_settings("toolbar").unwrap();
        assert!(!s.toolbar_visible);
        assert!(s.settings_open);
        s.transition("close-settings").unwrap();
        assert!(!s.settings_open);
        assert_eq!(s.mode, "interact");
        assert!(!s.toolbar_visible);
    }
    #[test]
    fn secondary_shortcuts_do_not_reveal_hidden_annotations() {
        let mut s = session();
        s.transition("hide").unwrap();
        s.transition("interact").unwrap();
        s.transition("toolbar").unwrap();
        assert_eq!(s.mode, "hidden");
    }
    #[test]
    fn shortcut_validation_rejects_reserved_and_unmodified_keys() {
        let mut p = Preferences::default();
        assert!(p.validate().is_ok());
        for shortcut in [
            "A",
            "Shift+A",
            INTERACT_SHORTCUT,
            TOOLBAR_SHORTCUT,
            "not a shortcut",
        ] {
            p.shortcut = shortcut.into();
            assert!(p.validate().is_err(), "{shortcut}");
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
    fn older_preferences_default_to_rainbow_and_keep_color_and_shortcut() {
        let preferences: Preferences = serde_json::from_str(
            r##"{"tool":"pen","color":"#68aaff","width":7,"shortcut":"CommandOrControl+Shift+B"}"##,
        )
        .unwrap();
        assert_eq!(preferences.color_mode, ColorMode::Rainbow);
        assert_eq!(preferences.toolbar_position, ToolbarPosition::Bottom);
        assert_eq!(preferences.tool, "arrow");
        assert_eq!(preferences.auto_fade_seconds, 0);
        assert_eq!(preferences.color, "#68aaff");
        assert_eq!(preferences.shortcut, "CommandOrControl+Shift+B");
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
