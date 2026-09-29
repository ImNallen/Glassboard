use crate::{
    commands::perform,
    state::{report, snapshot, AppState},
    windows::apply_windows,
    Result,
};
#[cfg(test)]
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    ImageEncoder,
};
use std::{borrow::Cow, io::Cursor, sync::Mutex};
use tauri::{Manager, PhysicalPosition, PhysicalSize};
// Captures live only in memory and are discarded on copy, cancellation, or a mode change.
pub(crate) struct CaptureImage(pub(crate) Mutex<Option<(u32, Vec<u8>)>>);

// Binary frame: little-endian u32 width and height, followed by full-resolution RGBA.
// Avoid compression, base64, JSON strings, and image decoding before region selection.
fn capture_frame(image: image::RgbaImage) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + image.as_raw().len());
    frame.extend_from_slice(&image.width().to_le_bytes());
    frame.extend_from_slice(&image.height().to_le_bytes());
    frame.extend_from_slice(&image.into_raw());
    frame
}

#[cfg(test)]
fn encode_capture_png(image: &image::RgbaImage) -> Result<Vec<u8>> {
    let mut png = Vec::new();
    // Adaptive filtering tries several predictors for every full-resolution row.
    // Sub handles the flat areas of desktop screenshots in one pass, losslessly.
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Sub)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    Ok(png)
}

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

pub(crate) fn start(app: &tauri::AppHandle) -> Result<()> {
    #[cfg(debug_assertions)]
    let started = std::time::Instant::now();
    if snapshot(app).capture.is_some() {
        if let Some(window) = app.get_webview_window("capture") {
            let _ = window.set_focus();
        }
        return Ok(());
    }
    check_permission()?;
    let cursor = app.cursor_position().map_err(|e| e.to_string())?;
    // Tao reports macOS cursor coordinates using the primary display's scale;
    // CoreGraphics/XCap expects global logical coordinates, including negative origins.
    let coordinate_scale = if cfg!(target_os = "macos") {
        app.primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or("No display is available")?
            .scale_factor()
    } else {
        1.0
    };
    let monitor = xcap::Monitor::from_point(
        (cursor.x / coordinate_scale).round() as i32,
        (cursor.y / coordinate_scale).round() as i32,
    )
    .map_err(|e| format!("Could not find the display under the cursor: {e}"))?;
    let position = (
        monitor.x().map_err(|e| e.to_string())?,
        monitor.y().map_err(|e| e.to_string())?,
    );
    let size = (
        monitor.width().map_err(|e| e.to_string())?,
        monitor.height().map_err(|e| e.to_string())?,
    );
    {
        let state = app.state::<AppState>();
        state.0.lock().unwrap().begin_capture();
    }
    let id = snapshot(app).capture.unwrap().id;
    apply_windows(app)?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let capture_app = handle.clone();
        let result = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>> {
            // Give the compositor time to remove Glassboard's controls and annotations.
            std::thread::sleep(std::time::Duration::from_millis(150));
            if snapshot(&capture_app).capture.map(|capture| capture.id) != Some(id) {
                return Err("Capture cancelled".into());
            }
            let image = monitor
                .capture_image()
                .map_err(|e| format!("Could not capture the screen: {e}"))?;
            Ok(capture_frame(image))
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|result| result);
        let app = handle.clone();
        if let Err(error) = handle.run_on_main_thread(move || {
            if snapshot(&app).capture.map(|capture| capture.id) != Some(id) {
                return;
            }
            let finish = (|| -> Result<()> {
                let data = result?;
                let window = app
                    .get_webview_window("capture")
                    .ok_or("Capture window is unavailable")?;
                if cfg!(target_os = "macos") {
                    window
                        .set_position(tauri::LogicalPosition::new(position.0, position.1))
                        .map_err(|e| e.to_string())?;
                    window
                        .set_size(tauri::LogicalSize::new(size.0, size.1))
                        .map_err(|e| e.to_string())?;
                } else {
                    window
                        .set_position(PhysicalPosition::new(position.0, position.1))
                        .map_err(|e| e.to_string())?;
                    window
                        .set_size(PhysicalSize::new(size.0, size.1))
                        .map_err(|e| e.to_string())?;
                }
                *app.state::<CaptureImage>().0.lock().unwrap() = Some((id, data));
                app.state::<AppState>()
                    .0
                    .lock()
                    .unwrap()
                    .capture
                    .as_mut()
                    .unwrap()
                    .ready = true;
                #[cfg(debug_assertions)]
                eprintln!(
                    "Screenshot frame ready in {:.1} ms",
                    started.elapsed().as_secs_f64() * 1000.0
                );
                apply_windows(&app)?;
                window.set_focus().map_err(|e| e.to_string())
            })();
            if let Err(error) = finish {
                let _ = perform(&app, "cancel-capture");
                let _ = perform(&app, "settings");
                report(&app, error);
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

fn check_capture(app: &tauri::AppHandle, window: &tauri::WebviewWindow, id: u32) -> Result<()> {
    if window.label() != "capture"
        || !snapshot(app)
            .capture
            .is_some_and(|capture| capture.id == id && capture.ready)
    {
        return Err("This capture is no longer open".into());
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn get_capture_image(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: u32,
) -> Result<tauri::ipc::Response> {
    check_capture(&app, &window, id)?;
    app.state::<CaptureImage>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .filter(|(generation, _)| *generation == id)
        .map(|(_, data)| tauri::ipc::Response::new(data.clone()))
        .ok_or("Capture image is unavailable".into())
}

fn decode_clipboard_image(png: &[u8]) -> Result<image::RgbaImage> {
    if png.len() > 96 * 1024 * 1024 {
        return Err("The capture is too large to copy".into());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(png), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|image| image.into_rgba8())
        .map_err(|e| format!("Could not read the annotated image: {e}"))
}

#[tauri::command]
pub(crate) async fn copy_capture(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: u32,
    png: Vec<u8>,
) -> Result<()> {
    check_capture(&app, &window, id)?;
    let clipboard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let image = decode_clipboard_image(&png)?;
        if snapshot(&clipboard_app).capture.map(|capture| capture.id) != Some(id) {
            return Err("Capture cancelled".into());
        }
        arboard::Clipboard::new()
            .map_err(|e| format!("Could not open the clipboard: {e}"))?
            .set_image(arboard::ImageData {
                width: image.width() as usize,
                height: image.height() as usize,
                bytes: Cow::Owned(image.into_raw()),
            })
            .map_err(|e| format!("Could not copy the image. Try again: {e}"))
    })
    .await
    .map_err(|e| e.to_string())??;
    // A delayed clipboard completion must not dismiss a newer capture or drawing session.
    if snapshot(&app).capture.map(|capture| capture.id) == Some(id) {
        perform(&app, "cancel-capture")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "manual full-resolution frame preparation benchmark"]
    fn benchmark_capture_preparation() {
        let source = image::RgbaImage::from_fn(3840, 2160, |x, y| {
            // Desktop-like flat panels, thin text rows, and a photo-like gradient.
            if x > 3000 && y > 150 {
                image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x * y) % 256) as u8, 255])
            } else if y % 36 < 10 && x % 180 < 140 && x > 240 {
                image::Rgba([38, 47, 53, 255])
            } else if x < 240 {
                image::Rgba([225, 230, 225, 255])
            } else {
                image::Rgba([247, 249, 247, 255])
            }
        });
        for raw in [false, true] {
            let mut samples = Vec::new();
            let mut bytes = 0;
            for _ in 0..3 {
                // Live capture already owns the image; clone outside the timed section.
                let image = source.clone();
                let started = std::time::Instant::now();
                let data = if raw {
                    capture_frame(image)
                } else {
                    encode_capture_png(&image).unwrap()
                };
                samples.push(started.elapsed().as_micros());
                bytes = data.len();
                if raw {
                    assert_eq!(&data[8..], source.as_raw());
                } else {
                    assert_eq!(decode_clipboard_image(&data).unwrap(), source);
                }
            }
            samples.sort_unstable();
            let name = if raw {
                "Binary frame"
            } else {
                "Previous fast PNG"
            };
            eprintln!(
                "{name}: median {:.2} ms, {bytes} bytes",
                samples[1] as f64 / 1000.0
            );
        }
    }

    #[test]
    fn binary_frame_preserves_dimensions_and_rgba_byte_order() {
        let original = image::RgbaImage::from_fn(17, 11, |x, y| {
            image::Rgba([
                (x * 15) as u8,
                (y * 23) as u8,
                (x * y) as u8,
                ((x + y) * 9) as u8,
            ])
        });
        let frame = capture_frame(original.clone());
        assert_eq!(&frame[..4], &17u32.to_le_bytes());
        assert_eq!(&frame[4..8], &11u32.to_le_bytes());
        assert_eq!(&frame[8..], original.as_raw());
    }

    #[test]
    fn fast_capture_png_preserves_pixel_dimensions_and_rgba() {
        let original = image::RgbaImage::from_fn(17, 11, |x, y| {
            image::Rgba([
                (x * 15) as u8,
                (y * 23) as u8,
                (x * y) as u8,
                ((x + y) * 9) as u8,
            ])
        });
        let png = encode_capture_png(&original).unwrap();
        assert_eq!(decode_clipboard_image(&png).unwrap(), original);
        assert!(decode_clipboard_image(b"not an image").is_err());
    }
}
