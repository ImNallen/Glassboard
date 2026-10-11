use super::{
    apply_windows, focus_drawing,
    geometry::{SETTINGS_SIZE, TOOLBAR_SIZE, TUTORIAL_SIZE},
    select_cursor_monitor,
    toolbar::position_toolbar,
    LiveOverlays, Surface,
};
use crate::{
    session::Mode,
    state::{report, snapshot, AppState},
    Result,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    Mutex, PoisonError,
};
use tauri::{Manager, Monitor, WebviewUrl, WebviewWindowBuilder, Wry};

const OVERLAY_TITLE: &str = "Glassboard annotations";

fn configure_overlay(window: &tauri::WebviewWindow, surface: Surface) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::WidgetExt;
        let native = window.gtk_window()?;
        if matches!(surface, Surface::Overlay(_)) {
            // Tao's input shape requires a native window even while the overlay is hidden.
            native.realize();
        }
    }
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
        // Every other surface floats above the drawing.
        native.setLevel(match surface {
            Surface::Overlay(_) => 25,
            _ => 26,
        });
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let _ = (window, surface);
    Ok(())
}
fn place_overlay(window: &tauri::WebviewWindow, monitor: &Monitor) -> tauri::Result<()> {
    super::set_size(window, *monitor.size())?;
    window.set_position(*monitor.position())
}
pub(crate) fn create_windows(app: &tauri::App) -> tauri::Result<()> {
    let monitors = app.available_monitors()?;
    for (index, monitor) in monitors.iter().enumerate() {
        let surface = Surface::Overlay(index);
        let window = window_builder(app, surface, OVERLAY_TITLE).build()?;
        place_overlay(&window, monitor)?;
        configure_overlay(&window, surface)?;
    }
    app.manage(LiveOverlays(AtomicUsize::new(monitors.len())));
    for (surface, title, (width, height)) in [
        (Surface::Capture, "Glassboard Capture", (800.0, 600.0)),
        (Surface::Toolbar, "Glassboard", TOOLBAR_SIZE),
        (Surface::Settings, "Glassboard Settings", SETTINGS_SIZE),
        (Surface::Tutorial, "Welcome to Glassboard", TUTORIAL_SIZE),
    ] {
        let window = window_builder(app, surface, title)
            .inner_size(width, height)
            .build()?;
        #[cfg(target_os = "linux")]
        super::set_size(&window, tauri::LogicalSize::new(width, height))?;
        configure_overlay(&window, surface)?;
    }
    Ok(())
}

/// Fit the drawing windows to the displays, which can change while Glassboard runs.
pub(crate) fn sync_overlays(app: &tauri::AppHandle) -> Result<()> {
    let monitors = app.available_monitors()?;
    let previous = app
        .state::<LiveOverlays>()
        .0
        .fetch_min(monitors.len(), SeqCst);
    for index in monitors.len()..previous {
        if let Some(window) = Surface::Overlay(index).window(app) {
            window.destroy()?;
        }
    }
    {
        let state = app.state::<AppState>();
        let mut state = state.0.lock().unwrap();
        if matches!(state.active_overlay, Surface::Overlay(index) if index >= monitors.len()) {
            state.active_overlay = Surface::Overlay(0);
        }
    }
    let live = previous.min(monitors.len());
    for (index, monitor) in monitors.iter().enumerate().take(live) {
        if let Some(window) = Surface::Overlay(index).window(app) {
            place_overlay(&window, monitor)?;
        }
    }
    let missing: Vec<_> = monitors.into_iter().enumerate().skip(live).collect();
    if !missing.is_empty() {
        add_overlays(app.clone(), missing);
    }
    Ok(())
}
/// Building a window on the main thread deadlocks on Windows, so drawing windows
/// for new displays are built on another thread and join on the main thread.
fn add_overlays(app: tauri::AppHandle, missing: Vec<(usize, Monitor)>) {
    static BUILDING: Mutex<()> = Mutex::new(());
    std::thread::spawn(move || {
        let _building = BUILDING.lock().unwrap_or_else(PoisonError::into_inner);
        for (index, monitor) in missing {
            let surface = Surface::Overlay(index);
            if surface.window(&app).is_some() {
                continue;
            }
            let handle = app.clone();
            let built = window_builder(&app, surface, OVERLAY_TITLE)
                .build()
                .and_then(|window| {
                    app.run_on_main_thread(move || {
                        if let Err(error) = join_overlay(&handle, &window, index, &monitor) {
                            report(&handle, error);
                        }
                    })
                });
            if let Err(error) = built {
                report(&app, error);
            }
        }
    });
}
fn join_overlay(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    index: usize,
    monitor: &Monitor,
) -> Result<()> {
    place_overlay(window, monitor)?;
    configure_overlay(window, Surface::Overlay(index))?;
    let live = &app.state::<LiveOverlays>().0;
    // The displays changed again while this window was being built.
    if live
        .compare_exchange(index, index + 1, SeqCst, SeqCst)
        .is_err()
    {
        return Ok(window.destroy()?);
    }
    // A capture that started meanwhile owns the active surface.
    if snapshot(app).mode == Mode::Draw {
        select_cursor_monitor(app);
        position_toolbar(app)?;
    }
    apply_windows(app)?;
    focus_drawing(app);
    Ok(())
}

fn window_builder<'a, M: Manager<Wry>>(
    manager: &'a M,
    surface: Surface,
    title: &str,
) -> WebviewWindowBuilder<'a, Wry, M> {
    WebviewWindowBuilder::new(
        manager,
        surface.label(),
        WebviewUrl::App(format!("index.html?surface={}", surface.url()).into()),
    )
    .title(title)
    // Drawing and controls should work on the click that activates their window.
    .accept_first_mouse(true)
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
}
