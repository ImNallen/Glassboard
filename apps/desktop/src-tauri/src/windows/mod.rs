pub(crate) mod geometry;
pub(crate) mod settings;
mod setup;
pub(crate) mod toolbar;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod z_order;
pub(crate) use setup::create_windows;

use crate::{
    session::Mode,
    state::{publish, snapshot, AppState},
    Result,
};
use geometry::{top_center, Rect, TUTORIAL_SIZE};
use serde::{Serialize, Serializer};
use tauri::{Manager, PhysicalPosition, PhysicalRect, PhysicalSize};

/// A webview window. The label names it to Tauri and `?surface=` tells the frontend what to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Surface {
    /// The drawing window covering one display, by monitor index.
    Overlay(usize),
    Capture,
    Toolbar,
    Settings,
    Tutorial,
}
impl Surface {
    pub(crate) fn url(self) -> &'static str {
        match self {
            Self::Overlay(_) => "overlay",
            Self::Capture => "capture",
            Self::Toolbar => "toolbar",
            Self::Settings => "settings",
            Self::Tutorial => "tutorial",
        }
    }
    pub(crate) fn label(self) -> String {
        match self {
            Self::Overlay(index) => format!("overlay-{index}"),
            fixed => fixed.url().into(),
        }
    }
    pub(crate) fn parse(label: &str) -> Option<Self> {
        if let Some(index) = label.strip_prefix("overlay-") {
            return index.parse().ok().map(Self::Overlay);
        }
        [Self::Capture, Self::Toolbar, Self::Settings, Self::Tutorial]
            .into_iter()
            .find(|surface| surface.url() == label)
    }
    pub(crate) fn window<R: tauri::Runtime>(
        self,
        app: &impl Manager<R>,
    ) -> Option<tauri::WebviewWindow<R>> {
        app.get_webview_window(&self.label())
    }
}
impl Serialize for Surface {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.label())
    }
}

pub(crate) fn set_size(
    window: &tauri::WebviewWindow,
    size: impl Into<tauri::Size>,
) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::{GtkWindowExt, WidgetExt};
        let size = size.into().to_logical::<i32>(window.scale_factor()?);
        let native_window = window.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        window.run_on_main_thread(move || {
            let result = native_window.gtk_window().map(|native| {
                // Nonresizable GTK windows fix both geometry limits to their requested size.
                native.set_size_request(size.width, size.height);
                native.resize(size.width, size.height);
            });
            let _ = sender.send(result);
        })?;
        receiver
            .recv()
            .map_err(|_| tauri::Error::FailedToReceiveMessage)?
    }
    #[cfg(not(target_os = "linux"))]
    window.set_size(size)
}

pub(crate) fn apply_windows(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let Some(capture) = Surface::Capture.window(app) {
        if state
            .capture
            .as_ref()
            .is_some_and(|capture| capture.ready.is_some())
        {
            capture.show()
        } else {
            capture.hide()
        }?;
    }
    for (label, window) in app.webview_windows() {
        if let Some(Surface::Overlay(_)) = Surface::parse(&label) {
            window.set_ignore_cursor_events(state.mode != Mode::Draw)?;
            window.set_focusable(state.mode == Mode::Draw)?;
            if state.mode == Mode::Hidden {
                window.hide()
            } else {
                window.show()
            }?;
        }
    }
    if let Some(toolbar) = Surface::Toolbar.window(app) {
        if state.mode != Mode::Hidden {
            toolbar.show()
        } else {
            toolbar.hide()
        }?;
    }
    if !state.settings_open || state.capture.is_some() {
        if let Some(settings) = Surface::Settings.window(app) {
            settings.hide()?;
        }
    }
    sync_tutorial(app)?;
    app.state::<toolbar::Toolbar>()
        .set_active(state.mode == Mode::Draw);
    raise_toolbar(app);
    publish(app)
}
/// Keep the small guide on the active display without covering the drawing toolbar.
pub(crate) fn sync_tutorial(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if let Some(window) = Surface::Tutorial.window(app) {
        if state.tutorial.is_none() || state.capture.is_some() {
            return Ok(window.hide()?);
        }
        if let Some(overlay) = state.active_overlay.window(app) {
            if let Some(monitor) = overlay.current_monitor()? {
                let work = Rect::work_area(&monitor);
                let scale = monitor.scale_factor();
                let size = work.fit(TUTORIAL_SIZE, scale, 0.0, 0.0);
                let frame = top_center(work, size, scale);
                // Resize first: macOS keeps the bottom edge when resizing, so a later resize would shift the top.
                set_size(&window, PhysicalSize::new(frame.width, frame.height))?;
                window.set_position(PhysicalPosition::new(frame.x, frame.y))?;
            }
        }
        window.show()?;
    }
    Ok(())
}
pub(crate) fn focus_drawing(app: &tauri::AppHandle) {
    let state = snapshot(app);
    if state.mode == Mode::Draw {
        if let Some(window) = state.active_overlay.window(app) {
            let _ = window.set_focus();
        }
    }
    raise_toolbar(app);
}
pub(crate) fn raise_toolbar(app: &tauri::AppHandle) {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    z_order::raise_controls(app);
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let _ = app;
}
pub(crate) fn select_cursor_monitor(app: &tauri::AppHandle) {
    if let Ok(cursor) = app.cursor_position() {
        for (label, window) in app.webview_windows() {
            let Some(overlay @ Surface::Overlay(_)) = Surface::parse(&label) else {
                continue;
            };
            if let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) {
                if Rect::from(PhysicalRect { position, size }).contains(cursor.x, cursor.y, 0.0) {
                    app.state::<AppState>().0.lock().unwrap().active_overlay = overlay;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Surface::{self, *};
    #[test]
    fn surfaces_keep_the_labels_and_urls_the_frontend_reads() {
        for (surface, label, url) in [
            (Overlay(2), "overlay-2", "overlay"),
            (Capture, "capture", "capture"),
            (Toolbar, "toolbar", "toolbar"),
            (Settings, "settings", "settings"),
            (Tutorial, "tutorial", "tutorial"),
        ] {
            assert_eq!((surface.label().as_str(), surface.url()), (label, url));
            assert_eq!(Surface::parse(label), Some(surface));
            let by_surface = std::collections::HashMap::from([(surface, surface)]);
            assert_eq!(
                serde_json::to_value(by_surface).unwrap(),
                serde_json::json!({ label: label })
            );
        }
        for label in ["overlay-", "overlay-x", "main"] {
            assert_eq!(Surface::parse(label), None);
        }
    }
}
