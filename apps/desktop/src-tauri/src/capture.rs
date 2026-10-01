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
use std::{
    borrow::Cow,
    io::Cursor,
    sync::{Arc, Mutex},
};
use tauri::{Manager, PhysicalPosition, PhysicalSize};
// Captures live only in memory and are discarded on copy, cancellation, or a mode change.
pub(crate) struct CaptureImage(pub(crate) Mutex<Option<(u32, Arc<Vec<u8>>)>>);

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
    let capture_point = (
        (cursor.x / coordinate_scale).round() as i32,
        (cursor.y / coordinate_scale).round() as i32,
    );
    let monitor = xcap::Monitor::from_point(capture_point.0, capture_point.1)
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
            if snapshot(&capture_app).capture.map(|capture| capture.id) != Some(id) {
                return Err("Capture cancelled".into());
            }
            #[cfg(debug_assertions)]
            let capture_started = std::time::Instant::now();
            let image =
                crate::capture_screen::capture(&capture_app, capture_point, position, size)?;
            #[cfg(debug_assertions)]
            eprintln!(
                "Native screenshot acquired in {:.1} ms",
                capture_started.elapsed().as_secs_f64() * 1000.0
            );
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
                // Reuse the native toolbar's work-area layout rather than docking
                // the embedded capture toolbar against the full display edges.
                let monitor = window
                    .current_monitor()
                    .map_err(|e| e.to_string())?
                    .ok_or("The capture display is unavailable")?;
                let area = monitor.work_area();
                let work = crate::settings_position::Bounds {
                    x: area.position.x as f64,
                    y: area.position.y as f64,
                    width: area.size.width as f64,
                    height: area.size.height as f64,
                };
                let scale = monitor.scale_factor();
                let layout = crate::toolbar_position::dock_layout(
                    snapshot(&app).preferences.toolbar_position,
                    work,
                    scale,
                    false,
                );
                let origin = window.outer_position().map_err(|e| e.to_string())?;
                let dock = layout.in_capture((origin.x, origin.y), scale);
                *app.state::<CaptureImage>().0.lock().unwrap() = Some((id, Arc::new(data)));
                {
                    let state = app.state::<AppState>();
                    let mut state = state.0.lock().unwrap();
                    let capture = state.capture.as_mut().unwrap();
                    capture.toolbar_dock = Some(dock);
                    capture.ready = true;
                }
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
        .map(|(_, data)| tauri::ipc::Response::new(data.as_ref().clone()))
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
    request: tauri::ipc::Request<'_>,
) -> Result<()> {
    let id = capture_request_id(
        request
            .headers()
            .get("x-glassboard-capture-id")
            .and_then(|id| id.to_str().ok()),
    )?;
    check_capture(&app, &window, id)?;
    let png = capture_request_bytes(request.body())?.to_vec();
    let clipboard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        #[cfg(debug_assertions)]
        let started = std::time::Instant::now();
        let image = decode_clipboard_image(&png)?;
        #[cfg(debug_assertions)]
        eprintln!(
            "Annotated screenshot decoded in {:.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        write_clipboard(&clipboard_app, id, image)
    })
    .await
    .map_err(|e| e.to_string())??;
    finish_copy(&app, id)
}

#[derive(Clone, Copy, serde::Deserialize)]
pub(crate) struct CaptureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn crop_frame(frame: &[u8], region: CaptureRegion) -> Result<image::RgbaImage> {
    let invalid = || "The selected capture region is invalid".to_string();
    if frame.len() < 8 {
        return Err(invalid());
    }
    let width = u32::from_le_bytes(frame[..4].try_into().unwrap());
    let height = u32::from_le_bytes(frame[4..8].try_into().unwrap());
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4));
    if width == 0
        || height == 0
        || expected != Some(frame.len() - 8)
        || region.width == 0
        || region.height == 0
        || region
            .x
            .checked_add(region.width)
            .is_none_or(|right| right > width)
        || region
            .y
            .checked_add(region.height)
            .is_none_or(|bottom| bottom > height)
    {
        return Err(invalid());
    }
    let length = region.width as usize * region.height as usize * 4;
    if length > 256 * 1024 * 1024 {
        return Err("The capture is too large to copy".into());
    }
    let stride = width as usize * 4;
    let row_length = region.width as usize * 4;
    let mut pixels = Vec::with_capacity(length);
    for y in region.y..region.y + region.height {
        let start = 8 + y as usize * stride + region.x as usize * 4;
        pixels.extend_from_slice(&frame[start..start + row_length]);
    }
    image::RgbaImage::from_raw(region.width, region.height, pixels).ok_or_else(invalid)
}

#[tauri::command]
pub(crate) async fn copy_capture_region(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: u32,
    region: CaptureRegion,
) -> Result<()> {
    check_capture(&app, &window, id)?;
    // Retain the source through cancellation without copying the full display.
    let frame = app
        .state::<CaptureImage>()
        .0
        .lock()
        .unwrap()
        .as_ref()
        .filter(|(generation, _)| *generation == id)
        .map(|(_, frame)| Arc::clone(frame))
        .ok_or("Capture image is unavailable")?;
    let clipboard_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(debug_assertions)]
        let started = std::time::Instant::now();
        let image = crop_frame(&frame, region)?;
        #[cfg(debug_assertions)]
        eprintln!(
            "Plain screenshot crop prepared in {:.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        write_clipboard(&clipboard_app, id, image)
    })
    .await
    .map_err(|e| e.to_string())??;
    finish_copy(&app, id)
}

fn write_clipboard(app: &tauri::AppHandle, id: u32, image: image::RgbaImage) -> Result<()> {
    if snapshot(app).capture.map(|capture| capture.id) != Some(id) {
        return Err("Capture cancelled".into());
    }
    #[cfg(debug_assertions)]
    let started = std::time::Instant::now();
    let result = arboard::Clipboard::new()
        .map_err(|e| format!("Could not open the clipboard: {e}"))?
        .set_image(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|e| format!("Could not copy the image. Try again: {e}"));
    #[cfg(debug_assertions)]
    eprintln!(
        "Screenshot clipboard write finished in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    result
}

fn finish_copy(app: &tauri::AppHandle, id: u32) -> Result<()> {
    // A delayed clipboard completion must not dismiss a newer capture or drawing session.
    if snapshot(app).capture.map(|capture| capture.id) == Some(id) {
        perform(app, "cancel-capture")?;
    }
    Ok(())
}

fn capture_request_id(value: Option<&str>) -> Result<u32> {
    value
        .and_then(|id| id.parse().ok())
        .ok_or("The capture ID is invalid".into())
}

fn capture_request_bytes(body: &tauri::ipc::InvokeBody) -> Result<&[u8]> {
    let tauri::ipc::InvokeBody::Raw(bytes) = body else {
        return Err("Expected a binary capture image".into());
    };
    if bytes.len() > 96 * 1024 * 1024 {
        return Err("The capture is too large to copy".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_crop_preserves_pixels_and_includes_the_last_row_and_column() {
        let source = image::RgbaImage::from_fn(7, 5, |x, y| {
            image::Rgba([x as u8, y as u8, (x * y) as u8, 128])
        });
        let frame = capture_frame(source.clone());
        let region = CaptureRegion {
            x: 3,
            y: 2,
            width: 4,
            height: 3,
        };
        let crop = crop_frame(&frame, region).unwrap();
        assert_eq!(crop.dimensions(), (4, 3));
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(crop.get_pixel(x, y), source.get_pixel(x + 3, y + 2));
            }
        }
        assert_eq!(
            crop_frame(
                &frame,
                CaptureRegion {
                    x: 0,
                    y: 0,
                    width: 7,
                    height: 5
                }
            )
            .unwrap(),
            source
        );
    }

    #[test]
    fn plain_crop_rejects_empty_overflowing_or_outside_regions_and_invalid_frames() {
        let frame = capture_frame(image::RgbaImage::new(7, 5));
        for region in [
            CaptureRegion {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            CaptureRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 0,
            },
            CaptureRegion {
                x: 6,
                y: 4,
                width: 2,
                height: 1,
            },
            CaptureRegion {
                x: 6,
                y: 4,
                width: 1,
                height: 2,
            },
            CaptureRegion {
                x: u32::MAX,
                y: 0,
                width: 1,
                height: 1,
            },
            CaptureRegion {
                x: 0,
                y: u32::MAX,
                width: 1,
                height: 1,
            },
        ] {
            assert!(crop_frame(&frame, region).is_err());
        }
        let region = CaptureRegion {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        for bytes in [&frame[..7], &frame[..frame.len() - 1]] {
            assert!(crop_frame(bytes, region).is_err());
        }
        for json in [
            r#"{"x":-1,"y":0,"width":1,"height":1}"#,
            r#"{"x":0.5,"y":0,"width":1,"height":1}"#,
        ] {
            assert!(serde_json::from_str::<CaptureRegion>(json).is_err());
        }
    }

    #[test]
    #[ignore = "manual native crop versus PNG round-trip benchmark"]
    fn benchmark_plain_capture_copy() {
        let source = image::RgbaImage::from_fn(3840, 2160, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x * y) % 256) as u8, 255])
        });
        let frame = capture_frame(source);
        let region = CaptureRegion {
            x: 400,
            y: 300,
            width: 1920,
            height: 1080,
        };
        let expected = crop_frame(&frame, region).unwrap();
        for png in [false, true] {
            let mut times = Vec::new();
            for _ in 0..5 {
                let started = std::time::Instant::now();
                let mut image = crop_frame(&frame, region).unwrap();
                if png {
                    image = decode_clipboard_image(&encode_capture_png(&image).unwrap()).unwrap();
                }
                times.push(started.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(image, expected);
            }
            times.sort_by(f64::total_cmp);
            eprintln!(
                "{}: median {:.2} ms",
                if png {
                    "Crop plus PNG encode/decode"
                } else {
                    "Direct crop"
                },
                times[2]
            );
        }
    }

    #[test]
    fn binary_clipboard_payload_preserves_png_pixels_and_rejects_json() {
        let original = image::RgbaImage::from_pixel(2, 3, image::Rgba([11, 22, 33, 128]));
        let body = tauri::ipc::InvokeBody::Raw(encode_capture_png(&original).unwrap());
        assert_eq!(
            decode_clipboard_image(capture_request_bytes(&body).unwrap()).unwrap(),
            original
        );
        assert!(capture_request_bytes(&tauri::ipc::InvokeBody::Json(
            serde_json::json!({ "png": [1, 2] })
        ))
        .is_err());
        assert_eq!(capture_request_id(Some("42")).unwrap(), 42);
        for id in [
            None,
            Some(""),
            Some("-1"),
            Some("4294967296"),
            Some("capture"),
        ] {
            assert!(capture_request_id(id).is_err());
        }
    }

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
