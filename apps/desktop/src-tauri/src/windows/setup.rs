use super::settings::{SETTINGS_HEIGHT, SETTINGS_WIDTH};
use tauri::{WebviewUrl, WebviewWindowBuilder};

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
pub(crate) fn create_windows(app: &tauri::App) -> tauri::Result<()> {
    for (index, monitor) in app.available_monitors()?.iter().enumerate() {
        let window = window_builder(
            app,
            &format!("overlay-{index}"),
            "overlay",
            "Glassboard annotations",
        )
        .build()?;
        window.set_size(*monitor.size())?;
        window.set_position(*monitor.position())?;
        configure_overlay(&window, false)?;
    }
    for (label, title, width, height) in [
        ("toolbar", "Glassboard", 744.0, 76.0),
        (
            "settings",
            "Glassboard Settings",
            SETTINGS_WIDTH,
            SETTINGS_HEIGHT,
        ),
        ("tutorial", "Welcome to Glassboard", 400.0, 330.0),
    ] {
        let window = window_builder(app, label, label, title)
            .inner_size(width, height)
            .build()?;
        configure_overlay(&window, true)?;
    }
    Ok(())
}

fn window_builder<'a>(
    app: &'a tauri::App,
    label: &str,
    surface: &str,
    title: &str,
) -> WebviewWindowBuilder<'a, tauri::Wry, tauri::App> {
    WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("index.html?surface={surface}").into()),
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
