use crate::{
    session::Mode,
    settings_position::Bounds,
    state::snapshot,
    toolbar_position::{dock_layout, ToolbarPointer, TrackingLayout},
    Result,
};
use std::sync::Mutex;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

/// Logical distance from the dock within which the collapsed toolbar expands.
const TOOLBAR_REVEAL_MARGIN: f64 = 20.0;
pub(crate) struct ToolbarExpanded(pub(crate) Mutex<bool>);
/// Physical window and dock bounds for focus-independent cursor tracking.
pub(crate) struct ToolbarLayout(pub(crate) Mutex<Option<TrackingLayout>>);

pub(crate) fn position_toolbar(app: &tauri::AppHandle) -> Result<()> {
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
    if snapshot(app).mode == Mode::Hidden {
        return None;
    }
    let layout = (*app.state::<ToolbarLayout>().0.lock().unwrap())?;
    let cursor = app.cursor_position().ok()?;
    layout.pointer_at(cursor.x, cursor.y, TOOLBAR_REVEAL_MARGIN)
}
/// Poll across native windows so hover does not depend on activating the webview.
pub(crate) fn watch_toolbar_proximity(app: tauri::AppHandle) {
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
