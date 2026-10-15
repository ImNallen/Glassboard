use super::{
    apply_windows, focus_drawing, geometry::Rect, keep_lazy_window, select_cursor_monitor,
    toolbar::position_toolbar, LiveOverlays, Surface,
};
use crate::{
    session::Mode,
    state::{report, snapshot, AppState},
    Result,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
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
    Rect::monitor(monitor).place(window, monitor.scale_factor())
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
    build_window(app, Surface::Toolbar)?;
    Ok(())
}

static SETUP_COMPLETE: AtomicBool = AtomicBool::new(false);
static CAPTURE_JOINED: AtomicBool = AtomicBool::new(false);
static CAPTURE_GENERATION: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn capture_window_ready() -> bool {
    CAPTURE_JOINED.load(SeqCst)
}

fn mark_capture_joined(surface: Surface, joined: bool) {
    if surface == Surface::Capture {
        CAPTURE_JOINED.store(joined, SeqCst);
    }
}

/// After this, `WebviewWindowBuilder::build` must run off the event-loop thread
/// on Windows or it deadlocks inside commands and handlers.
pub(crate) fn mark_setup_complete() {
    SETUP_COMPLETE.store(true, SeqCst);
}

fn build_window(
    app: &impl Manager<Wry>,
    surface: Surface,
) -> tauri::Result<tauri::WebviewWindow<Wry>> {
    let (title, (width, height)) = surface
        .title_and_size()
        .expect("ensure_window builds Capture, Toolbar, Settings, or Tutorial");
    let window = window_builder(app, surface, title)
        .inner_size(width, height)
        .build()?;
    #[cfg(target_os = "linux")]
    super::set_size(&window, tauri::LogicalSize::new(width, height))?;
    configure_overlay(&window, surface)?;
    mark_capture_joined(surface, true);
    Ok(window)
}

#[derive(Debug, PartialEq, Eq)]
enum LazyFinish {
    DestroyWithoutBump,
    Skip,
    Drop,
    Proceed,
}

fn lazy_finish_action(stale: bool, keep: bool) -> LazyFinish {
    match (stale, keep) {
        (true, false) => LazyFinish::DestroyWithoutBump,
        (true, true) => LazyFinish::Skip,
        (false, false) => LazyFinish::Drop,
        (false, true) => LazyFinish::Proceed,
    }
}

fn join_fixed_window(
    app: &tauri::AppHandle,
    surface: Surface,
    width: f64,
    height: f64,
) -> Result<()> {
    let Some(window) = surface.window(app) else {
        mark_capture_joined(surface, false);
        return Ok(());
    };
    #[cfg(target_os = "linux")]
    super::set_size(&window, tauri::LogicalSize::new(width, height))?;
    #[cfg(not(target_os = "linux"))]
    let _ = (width, height);
    configure_overlay(&window, surface)?;
    mark_capture_joined(surface, true);
    Ok(())
}

fn finish_lazy_window(
    app: &tauri::AppHandle,
    surface: Surface,
    size: Option<(f64, f64)>,
    generation: usize,
    after: impl FnOnce(&tauri::AppHandle, &tauri::WebviewWindow<Wry>) -> Result<()>,
) -> Result<()> {
    let state = snapshot(app);
    let stale = surface == Surface::Capture && CAPTURE_GENERATION.load(SeqCst) != generation;
    match lazy_finish_action(stale, keep_lazy_window(surface, &state)) {
        LazyFinish::DestroyWithoutBump => {
            mark_capture_joined(surface, false);
            if let Some(live) = surface.window(app) {
                live.destroy()?;
            }
            return Ok(());
        }
        LazyFinish::Skip => return Ok(()),
        LazyFinish::Drop => return Ok(drop_window(app, surface)?),
        LazyFinish::Proceed => {}
    }
    let needs_join = surface != Surface::Capture || !capture_window_ready();
    if needs_join {
        if let Some((width, height)) = size {
            join_fixed_window(app, surface, width, height)?;
        } else if surface == Surface::Capture {
            let (_, (width, height)) = surface
                .title_and_size()
                .expect("ensure_window builds Capture, Toolbar, Settings, or Tutorial");
            join_fixed_window(app, surface, width, height)?;
        }
    }
    let Some(live) = surface.window(app) else {
        mark_capture_joined(surface, false);
        return Ok(());
    };
    after(app, &live)
}

/// Build Settings, Tutorial, or Capture if missing. Safe during setup on this
/// thread; after setup, `build()` runs on a worker like `add_overlays`.
pub(crate) fn ensure_window(
    app: &tauri::AppHandle,
    surface: Surface,
    after: impl FnOnce(&tauri::AppHandle, &tauri::WebviewWindow<Wry>) -> Result<()> + Send + 'static,
) -> Result<()> {
    if surface.window(app).is_some() {
        return finish_lazy_window(app, surface, None, CAPTURE_GENERATION.load(SeqCst), after);
    }
    if !SETUP_COMPLETE.load(SeqCst) {
        let window = build_window(app, surface)?;
        return after(app, &window);
    }
    let app = app.clone();
    std::thread::spawn(move || {
        static BUILDING: Mutex<()> = Mutex::new(());
        let _building = BUILDING.lock().unwrap_or_else(PoisonError::into_inner);
        let generation = CAPTURE_GENERATION.load(SeqCst);
        if surface.window(&app).is_some() {
            let handle = app.clone();
            let posted = app.run_on_main_thread(move || {
                if let Err(error) = finish_lazy_window(&handle, surface, None, generation, after) {
                    report(&handle, error);
                }
            });
            if let Err(error) = posted {
                report(&app, error);
            }
            return;
        }
        let (title, (width, height)) = surface
            .title_and_size()
            .expect("ensure_window builds Capture, Toolbar, Settings, or Tutorial");
        let handle = app.clone();
        let built = window_builder(&app, surface, title)
            .inner_size(width, height)
            .build()
            .and_then(|_| {
                app.run_on_main_thread(move || {
                    if let Err(error) = finish_lazy_window(
                        &handle,
                        surface,
                        Some((width, height)),
                        generation,
                        after,
                    ) {
                        report(&handle, error);
                    }
                })
            });
        if let Err(error) = built {
            report(&app, error);
        }
    });
    Ok(())
}

pub(crate) fn drop_window(app: &impl Manager<Wry>, surface: Surface) -> tauri::Result<()> {
    if surface == Surface::Capture {
        CAPTURE_GENERATION.fetch_add(1, SeqCst);
    }
    mark_capture_joined(surface, false);
    if let Some(window) = surface.window(app) {
        window.destroy()?;
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

#[cfg(test)]
mod tests {
    use super::{lazy_finish_action, LazyFinish};

    #[test]
    fn stale_callbacks_never_join_or_after() {
        assert_eq!(
            lazy_finish_action(true, false),
            LazyFinish::DestroyWithoutBump
        );
        assert_eq!(lazy_finish_action(true, true), LazyFinish::Skip);
        assert_eq!(lazy_finish_action(false, false), LazyFinish::Drop);
        assert_eq!(lazy_finish_action(false, true), LazyFinish::Proceed);
    }
}
