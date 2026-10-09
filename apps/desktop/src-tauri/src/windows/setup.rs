use super::{
    geometry::{SETTINGS_SIZE, TOOLBAR_SIZE, TUTORIAL_SIZE},
    Surface,
};
use tauri::{WebviewUrl, WebviewWindowBuilder};

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
pub(crate) fn create_windows(app: &tauri::App) -> tauri::Result<()> {
    for (index, monitor) in app.available_monitors()?.iter().enumerate() {
        let surface = Surface::Overlay(index);
        let window = window_builder(app, surface, "Glassboard annotations").build()?;
        super::set_size(&window, *monitor.size())?;
        window.set_position(*monitor.position())?;
        configure_overlay(&window, surface)?;
    }
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

fn window_builder<'a>(
    app: &'a tauri::App,
    surface: Surface,
    title: &str,
) -> WebviewWindowBuilder<'a, tauri::Wry, tauri::App> {
    WebviewWindowBuilder::new(
        app,
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
