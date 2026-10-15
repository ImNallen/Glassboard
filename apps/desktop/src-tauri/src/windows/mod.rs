pub(crate) mod geometry;
pub(crate) mod settings;
mod setup;
pub(crate) mod toolbar;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod z_order;
pub(crate) use setup::{
    capture_window_ready, create_windows, drop_window, ensure_window, mark_setup_complete,
    sync_overlays,
};

use crate::{
    session::Mode,
    state::{publish, snapshot, AppState},
    Result,
};
use geometry::{
    monitor_under_cursor, top_center, Rect, SETTINGS_SIZE, TOOLBAR_SIZE, TUTORIAL_SIZE,
};
use serde::{Serialize, Serializer};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use tauri::Manager;

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
    pub(crate) fn title_and_size(self) -> Option<(&'static str, (f64, f64))> {
        match self {
            Self::Overlay(_) => None,
            Self::Capture => Some(("Glassboard Capture", (800.0, 600.0))),
            Self::Toolbar => Some(("Glassboard", TOOLBAR_SIZE)),
            Self::Settings => Some(("Glassboard Settings", SETTINGS_SIZE)),
            Self::Tutorial => Some(("Welcome to Glassboard", TUTORIAL_SIZE)),
        }
    }
}
impl Serialize for Surface {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.label())
    }
}

/// How many drawing windows, from `overlay-0` up, cover a display and are ready to show.
/// Windows still being built for a new display, or left by a removed one, are not counted.
pub(crate) struct LiveOverlays(AtomicUsize);
fn live_overlays(
    app: &tauri::AppHandle,
) -> impl Iterator<Item = (Surface, tauri::WebviewWindow)> + '_ {
    (0..app.state::<LiveOverlays>().0.load(SeqCst)).filter_map(|index| {
        let surface = Surface::Overlay(index);
        surface.window(app).map(|window| (surface, window))
    })
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

/// Whether a lazily created surface still belongs on screen.
pub(crate) fn keep_lazy_window(surface: Surface, state: &crate::session::Session) -> bool {
    match surface {
        Surface::Settings => state.settings_open && state.capture.is_none(),
        Surface::Tutorial => state.tutorial.is_some() && state.capture.is_none(),
        Surface::Capture => state.capture.is_some(),
        Surface::Toolbar | Surface::Overlay(_) => true,
    }
}

pub(crate) fn apply_windows(app: &tauri::AppHandle) -> Result<()> {
    let state = snapshot(app);
    if state.capture.is_none() {
        drop_window(app, Surface::Capture)?;
    } else if let Some(capture) = Surface::Capture.window(app) {
        if state
            .capture
            .as_ref()
            .is_some_and(|capture| capture.ready.is_some())
        {
            capture.show()?;
        } else {
            capture.hide()?;
        }
    }
    for (_, window) in live_overlays(app) {
        window.set_ignore_cursor_events(state.mode != Mode::Draw)?;
        window.set_focusable(state.mode == Mode::Draw)?;
        if state.mode == Mode::Hidden {
            window.hide()
        } else {
            window.show()
        }?;
    }
    if let Some(toolbar) = Surface::Toolbar.window(app) {
        if state.mode != Mode::Hidden {
            toolbar.show()
        } else {
            toolbar.hide()
        }?;
    }
    if !state.settings_open || state.capture.is_some() {
        drop_window(app, Surface::Settings)?;
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
    if state.tutorial.is_none() || state.capture.is_some() {
        return Ok(drop_window(app, Surface::Tutorial)?);
    }
    ensure_window(app, Surface::Tutorial, |app, window| {
        let state = snapshot(app);
        if let Some(overlay) = state.active_overlay.window(app) {
            if let Some(monitor) = overlay.current_monitor()? {
                let work = Rect::work_area(&monitor);
                let scale = monitor.scale_factor();
                let size = work.fit(TUTORIAL_SIZE, scale, 0.0, 0.0);
                top_center(work, size, scale).place(window, scale)?;
            }
        }
        Ok(window.show()?)
    })
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
/// The scale tao applied to the cursor position.
pub(crate) fn cursor_scale(app: &tauri::AppHandle) -> f64 {
    app.primary_monitor()
        .ok()
        .flatten()
        .map_or(1.0, |monitor| monitor.scale_factor())
}
pub(crate) fn select_cursor_monitor(app: &tauri::AppHandle) {
    let (Ok(cursor), Ok(monitors)) = (app.cursor_position(), app.available_monitors()) else {
        return;
    };
    let live = app.state::<LiveOverlays>().0.load(SeqCst);
    let monitors = monitors
        .iter()
        .take(live)
        .map(|monitor| (Rect::monitor(monitor), monitor.scale_factor()));
    if let Some(index) = monitor_under_cursor((cursor.x, cursor.y), cursor_scale(app), monitors) {
        app.state::<AppState>().0.lock().unwrap().active_overlay = Surface::Overlay(index);
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

    #[test]
    fn lazy_windows_follow_the_session_that_apply_windows_uses() {
        use crate::{
            preferences::Preferences,
            session::{Session, Transition},
        };
        let keep = |surface, session: &Session| super::keep_lazy_window(surface, session);
        let mut session = Session::new(Preferences::default());
        session.transition(Transition::Settings);
        assert!(keep(Settings, &session));
        assert!(!keep(Capture, &session));
        session.transition(Transition::CloseSettings);
        assert!(!keep(Settings, &session));
        session.tutorial = Some(crate::session::TutorialStep::Welcome);
        assert!(keep(Tutorial, &session));
        session.begin_capture();
        assert!(keep(Capture, &session));
        assert!(!keep(Settings, &session));
        assert!(!keep(Tutorial, &session));
        session.transition(Transition::CancelCapture);
        assert!(!keep(Capture, &session));
    }

    #[test]
    fn fixed_windows_have_titles_and_sizes() {
        use super::geometry::{SETTINGS_SIZE, TOOLBAR_SIZE, TUTORIAL_SIZE};
        assert_eq!(Overlay(0).title_and_size(), None);
        assert_eq!(
            Capture.title_and_size(),
            Some(("Glassboard Capture", (800.0, 600.0)))
        );
        assert_eq!(Toolbar.title_and_size(), Some(("Glassboard", TOOLBAR_SIZE)));
        assert_eq!(
            Settings.title_and_size(),
            Some(("Glassboard Settings", SETTINGS_SIZE))
        );
        assert_eq!(
            Tutorial.title_and_size(),
            Some(("Welcome to Glassboard", TUTORIAL_SIZE))
        );
    }
}
