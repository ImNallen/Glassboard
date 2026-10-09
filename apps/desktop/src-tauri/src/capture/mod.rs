mod image;
mod screen;

use crate::{
    commands::transition,
    session::{ReadyCapture, Transition},
    state::{report, snapshot, AppState},
    windows::{
        apply_windows,
        geometry::{dock, Rect},
        Surface,
    },
    Result,
};
pub(crate) use image::CaptureRegion;
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};
use tauri::{LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize};

#[derive(Default)]
pub(crate) struct CaptureClipboard(Mutex<Option<arboard::Clipboard>>);

/// The IPC request cap for an annotated PNG, checked once where the bytes arrive.
const MAX_REQUEST_BYTES: usize = 96 * 1024 * 1024;

#[cfg(target_os = "macos")]
fn check_permission() -> Result<()> {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    // CoreGraphics can return wallpaper without app content when permission is denied.
    // Request the OS permission before capturing so that result cannot look like success.
    if unsafe { CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess() } {
        Ok(())
    } else {
        Err("Allow Glassboard in System Settings → Privacy & Security → Screen Recording, then try capturing again. macOS may require restarting Glassboard.".into())
    }
}
#[cfg(not(target_os = "macos"))]
fn check_permission() -> Result<()> {
    Ok(())
}

/// The display being captured, found by its origin. The capture window can't
/// be asked: macOS moves it asynchronously, so right after `set_position` it
/// still reports its previous frame, and the toolbar would dock against that.
fn capture_monitor(app: &tauri::AppHandle, origin: (i32, i32)) -> Result<tauri::Monitor> {
    app.available_monitors()?
        .into_iter()
        .find(|monitor| {
            // XCap reports macOS displays in points and Windows displays in pixels.
            let scale = if cfg!(target_os = "macos") {
                monitor.scale_factor()
            } else {
                1.0
            };
            let position = monitor.position();
            let x = (position.x as f64 / scale).round() as i32;
            let y = (position.y as f64 / scale).round() as i32;
            (x, y) == origin
        })
        .ok_or_else(|| "The capture display is unavailable".into())
}

/// Whether `id` is still the open capture. Work started for an older capture stops here.
fn current(app: &tauri::AppHandle, id: u32) -> bool {
    app.state::<AppState>()
        .0
        .lock()
        .unwrap()
        .capture
        .as_ref()
        .is_some_and(|capture| capture.id == id)
}

/// The frame a capture webview may read: capture `id`, once it is ready.
fn ready(app: &tauri::AppHandle, window: &tauri::WebviewWindow, id: u32) -> Result<ReadyCapture> {
    let state = app.state::<AppState>();
    let state = state.0.lock().unwrap();
    state
        .capture
        .as_ref()
        .filter(|capture| {
            capture.id == id && Surface::parse(window.label()) == Some(Surface::Capture)
        })
        .and_then(|capture| capture.ready.clone())
        .ok_or("This capture is no longer open".into())
}

/// Capture failures surface in Settings: the caller can be a hidden toolbar
/// webview, so the shared session carries the explanation to a visible window.
pub(crate) fn fail(app: &tauri::AppHandle, error: impl std::fmt::Display) {
    let _ = transition(app, Transition::Settings);
    report(app, error);
}

pub(crate) fn start(app: &tauri::AppHandle) -> Result<()> {
    if snapshot(app).capture.is_some() {
        if let Some(window) = Surface::Capture.window(app) {
            let _ = window.set_focus();
        }
        return Ok(());
    }
    check_permission()?;
    let cursor = app.cursor_position()?;
    // Tao reports macOS cursor coordinates using the primary display's scale;
    // CoreGraphics/XCap expects global logical coordinates, including negative origins.
    let coordinate_scale = if cfg!(target_os = "macos") {
        app.primary_monitor()?
            .ok_or("No display is available")?
            .scale_factor()
    } else {
        1.0
    };
    let capture_point = (
        (cursor.x / coordinate_scale).round() as i32,
        (cursor.y / coordinate_scale).round() as i32,
    );
    #[cfg(target_os = "linux")]
    let (position, size) = {
        let monitor = app
            .available_monitors()?
            .into_iter()
            .find(|monitor| {
                Rect::from(tauri::PhysicalRect {
                    position: *monitor.position(),
                    size: *monitor.size(),
                })
                .contains(cursor.x, cursor.y, 0.0)
            })
            .ok_or("Could not find the display under the cursor")?;
        let position = monitor.position();
        let size = monitor.size();
        ((position.x, position.y), (size.width, size.height))
    };
    #[cfg(not(target_os = "linux"))]
    let (position, size) = {
        let monitor = xcap::Monitor::from_point(capture_point.0, capture_point.1)
            .map_err(|e| format!("Could not find the display under the cursor: {e}"))?;
        let position = (monitor.x()?, monitor.y()?);
        let size = (monitor.width()?, monitor.height()?);
        (position, size)
    };
    let id = app.state::<AppState>().0.lock().unwrap().begin_capture();
    apply_windows(app)?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let capture_app = handle.clone();
        let captured = tauri::async_runtime::spawn_blocking(move || {
            if !current(&capture_app, id) {
                return Err("Capture cancelled".into());
            }
            screen::capture(&capture_app, capture_point, position, size)
        })
        .await
        .map_err(Into::into)
        .and_then(|result| result);
        let app = handle.clone();
        if let Err(error) = handle.run_on_main_thread(move || {
            if !current(&app, id) {
                return;
            }
            if let Err(error) = captured.and_then(|image| present(&app, id, position, size, image))
            {
                fail(&app, error);
            }
        }) {
            report(
                &handle,
                format!("Could not open the capture editor: {error}"),
            );
        }
    });
    Ok(())
}

/// Sizes the capture window to the captured display and attaches the frame to
/// capture `id`. A frame for a capture that was cancelled meanwhile is dropped.
fn present(
    app: &tauri::AppHandle,
    id: u32,
    position: (i32, i32),
    size: (u32, u32),
    image: ::image::RgbaImage,
) -> Result<()> {
    let window = Surface::Capture
        .window(app)
        .ok_or("Capture window is unavailable")?;
    let (origin, extent): (tauri::Position, tauri::Size) = if cfg!(target_os = "macos") {
        (
            LogicalPosition::new(position.0, position.1).into(),
            LogicalSize::new(size.0, size.1).into(),
        )
    } else {
        (
            PhysicalPosition::new(position.0, position.1).into(),
            PhysicalSize::new(size.0, size.1).into(),
        )
    };
    window.set_position(origin)?;
    crate::windows::set_size(&window, extent)?;
    // Reuse the native toolbar's work-area layout rather than docking
    // the embedded capture toolbar against the full display edges.
    let monitor = capture_monitor(app, position)?;
    let work = Rect::work_area(&monitor);
    let scale = monitor.scale_factor();
    let display = monitor.position();
    let attached = {
        let state = app.state::<AppState>();
        let mut state = state.0.lock().unwrap();
        let layout = dock(state.preferences.toolbar_position, work, scale, false);
        let ready = ReadyCapture {
            toolbar_dock: layout.in_css(*display, scale),
            image: Arc::new(image),
        };
        state.finish_capture(id, ready)
    };
    if !attached {
        return Ok(());
    }
    apply_windows(app)?;
    Ok(window.set_focus()?)
}

#[tauri::command]
pub(crate) fn get_capture_image(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: u32,
) -> Result<tauri::ipc::Response> {
    let image = ready(&app, &window, id)?.image;
    Ok(tauri::ipc::Response::new(image::frame(&image)))
}

#[tauri::command]
pub(crate) async fn copy_capture(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: tauri::ipc::Request<'_>,
) -> Result<()> {
    let id = request_id(
        request
            .headers()
            .get("x-glassboard-capture-id")
            .and_then(|id| id.to_str().ok()),
    )?;
    ready(&app, &window, id)?;
    let png = request_bytes(request.body())?.to_vec();
    let clipboard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        write_clipboard(&clipboard_app, id, image::decode_png(&png)?)
    })
    .await??;
    finish_copy(&app, id)
}

#[tauri::command]
pub(crate) async fn copy_capture_region(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: u32,
    region: CaptureRegion,
) -> Result<()> {
    let image = ready(&app, &window, id)?.image;
    let clipboard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        write_clipboard(&clipboard_app, id, image::crop(&image, region)?)
    })
    .await??;
    finish_copy(&app, id)
}

fn write_clipboard(app: &tauri::AppHandle, id: u32, image: ::image::RgbaImage) -> Result<()> {
    let state = app.state::<CaptureClipboard>();
    let mut clipboard = state.0.lock().unwrap();
    if !current(app, id) {
        return Err("Capture cancelled".into());
    }
    let clipboard = match clipboard.as_mut() {
        Some(clipboard) => clipboard,
        None => clipboard.insert(
            arboard::Clipboard::new().map_err(|e| format!("Could not open the clipboard: {e}"))?,
        ),
    };
    clipboard
        .set_image(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|e| format!("Could not copy the image. Try again: {e}").into())
}

fn finish_copy(app: &tauri::AppHandle, id: u32) -> Result<()> {
    // A delayed clipboard completion must not dismiss a newer capture or drawing session.
    if current(app, id) {
        transition(app, Transition::CancelCapture)?;
    }
    Ok(())
}

fn request_id(value: Option<&str>) -> Result<u32> {
    value
        .and_then(|id| id.parse().ok())
        .ok_or("The capture ID is invalid".into())
}

fn request_bytes(body: &tauri::ipc::InvokeBody) -> Result<&[u8]> {
    let tauri::ipc::InvokeBody::Raw(bytes) = body else {
        return Err("Expected a binary capture image".into());
    };
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err("The capture is too large to copy".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_requests_need_a_binary_body_and_a_numeric_capture_id() {
        let body = tauri::ipc::InvokeBody::Raw(vec![1, 2, 3]);
        assert_eq!(request_bytes(&body).unwrap(), &[1, 2, 3]);
        assert!(request_bytes(&tauri::ipc::InvokeBody::Json(
            serde_json::json!({ "png": [1, 2] })
        ))
        .is_err());
        assert_eq!(request_id(Some("42")).unwrap(), 42);
        for id in [
            None,
            Some(""),
            Some("-1"),
            Some("4294967296"),
            Some("capture"),
        ] {
            assert!(request_id(id).is_err());
        }
    }
}
