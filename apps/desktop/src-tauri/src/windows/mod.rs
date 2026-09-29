pub(crate) mod settings;
mod setup;
pub(crate) mod toolbar;
#[cfg(target_os = "windows")]
mod z_order;
pub(crate) use setup::create_windows;

use crate::{
    session::Mode,
    state::{publish, snapshot, AppState},
    Result,
};
use tauri::{Manager, PhysicalPosition, PhysicalSize};

pub(crate) fn apply_windows(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if state.capture.is_none() {
        *app.state::<crate::capture::CaptureImage>()
            .0
            .lock()
            .unwrap() = None;
    }
    if let Some(capture) = app.get_webview_window("capture") {
        if state.capture.is_some_and(|capture| capture.ready) {
            capture.show()
        } else {
            capture.hide()
        }
        .map_err(|e| e.to_string())?;
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            window
                .set_ignore_cursor_events(state.mode != Mode::Draw)
                .map_err(|e| e.to_string())?;
            window
                .set_focusable(state.mode == Mode::Draw)
                .map_err(|e| e.to_string())?;
            if state.mode == Mode::Hidden {
                window.hide()
            } else {
                window.show()
            }
            .map_err(|e| e.to_string())?;
        }
    }
    if let Some(toolbar) = app.get_webview_window("toolbar") {
        if state.mode != Mode::Hidden {
            toolbar.show()
        } else {
            toolbar.hide()
        }
        .map_err(|e| e.to_string())?;
    }
    if !state.settings_open || state.capture.is_some() {
        if let Some(settings) = app.get_webview_window("settings") {
            settings.hide().map_err(|e| e.to_string())?;
        }
    }
    sync_tutorial(app)?;
    publish(app)
}
/// Keep the small guide on the active display without covering the drawing toolbar.
pub(crate) fn sync_tutorial(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let Some(window) = app.get_webview_window("tutorial") {
        if state.tutorial.is_none() || state.capture.is_some() {
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
pub(crate) fn focus_drawing(app: &tauri::AppHandle) {
    let state = snapshot(app);
    if state.mode == Mode::Draw {
        if let Some(window) = app.get_webview_window(&state.active_overlay) {
            let _ = window.set_focus();
        }
    }
    raise_toolbar(app);
}
pub(crate) fn raise_toolbar(app: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    z_order::raise_controls(app);
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}
pub(crate) fn select_cursor_monitor(app: &tauri::AppHandle) {
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
