use super::{
    geometry::{dock, Rect, ToolbarPointer, TrackingLayout},
    Surface,
};
use crate::{state::snapshot, Result};
use std::sync::{Condvar, Mutex, MutexGuard};
use tauri::{Emitter, Manager};

/// Logical distance from the dock within which the collapsed toolbar expands.
const TOOLBAR_REVEAL_MARGIN: f64 = 20.0;

/// Toolbar window state shared with the cursor-tracking thread.
#[derive(Default)]
pub(crate) struct Toolbar {
    state: Mutex<ToolbarState>,
    /// Wakes the sleeping tracker when drawing starts.
    wake: Condvar,
}
#[derive(Default)]
struct ToolbarState {
    /// The window has grown to make room for an error panel.
    expanded: bool,
    /// Physical window and dock bounds for focus-independent cursor tracking.
    layout: Option<TrackingLayout>,
    /// Tracking runs only while drawing.
    active: bool,
}
impl Toolbar {
    fn lock(&self) -> MutexGuard<'_, ToolbarState> {
        self.state.lock().unwrap()
    }
    pub(crate) fn set_active(&self, active: bool) {
        let mut state = self.lock();
        if state.active != active {
            state.active = active;
            self.wake.notify_one();
        }
    }
    pub(crate) fn set_expanded(&self, expanded: bool) {
        self.lock().expanded = expanded;
    }
}

pub(crate) fn position_toolbar(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let (Some(overlay), Some(window)) = (
        state.active_overlay.window(app),
        Surface::Toolbar.window(app),
    ) {
        if let Some(monitor) = overlay.current_monitor()? {
            let work = Rect::work_area(&monitor);
            let scale = monitor.scale_factor();
            let position = state.preferences.toolbar_position;
            let toolbar = app.state::<Toolbar>();
            let layout = dock(position, work, scale, toolbar.lock().expanded);
            layout.place(&window)?;
            // Popover space must not enlarge the cursor-proximity area: otherwise
            // moving away into the transparent part of the window keeps it open.
            toolbar.lock().layout = Some(TrackingLayout {
                dock: dock(position, work, scale, false),
                window: layout,
                scale,
            });
        }
    }
    Ok(())
}
/// Poll across native windows so hover does not depend on activating the webview.
pub(crate) fn watch_toolbar_proximity(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let toolbar = app.state::<Toolbar>();
        let mut tracked = false;
        loop {
            let mut state = toolbar.lock();
            while !state.active {
                if tracked {
                    let _ = app.emit_to(
                        Surface::Toolbar.label(),
                        "toolbar-pointer",
                        Option::<ToolbarPointer>::None,
                    );
                    tracked = false;
                }
                state = toolbar.wake.wait(state).unwrap();
            }
            let layout = state.layout;
            drop(state);
            // Cursor position in toolbar CSS pixels, even when the canvas owns keyboard focus.
            let pointer = layout.and_then(|layout| {
                let cursor = app.cursor_position().ok()?;
                layout.pointer_at(cursor.x, cursor.y, TOOLBAR_REVEAL_MARGIN)
            });
            // Repeat nearby samples: revealing/resizing the toolbar can put a
            // button under a stationary cursor without generating pointerenter.
            if pointer.is_some() || tracked {
                let _ = app.emit_to(Surface::Toolbar.label(), "toolbar-pointer", pointer);
            }
            tracked = pointer.is_some();
            let state = toolbar.lock();
            if state.active {
                let _ = toolbar
                    .wake
                    .wait_timeout(state, std::time::Duration::from_millis(16))
                    .unwrap();
            }
        }
    });
}
