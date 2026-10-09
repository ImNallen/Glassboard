use crate::Result;

pub(crate) fn capture(
    app: &tauri::AppHandle,
    point: (i32, i32),
    position: (i32, i32),
    size: (u32, u32),
) -> Result<image::RgbaImage> {
    #[cfg(target_os = "macos")]
    {
        let _ = (app, point);
        macos::capture(position, size)
    }
    #[cfg(target_os = "linux")]
    {
        let _ = (app, point);
        std::thread::sleep(std::time::Duration::from_millis(150));
        x11::capture(position, size)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = (position, size);
        // The main-thread barrier applies hide requests before the compositor flush.
        let (send, receive) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let status = unsafe { windows_sys::Win32::Graphics::Dwm::DwmFlush() };
            let _ = send.send(status);
        })?;
        let status = receive.recv()?;
        if status < 0 {
            log::warn!("Compositor flush failed ({status:#x}); using capture delay");
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
        xcap::Monitor::from_point(point.0, point.1)
            .and_then(|monitor| monitor.capture_image())
            .map_err(|e| format!("Could not capture the screen: {e}").into())
    }
}

#[cfg(any(target_os = "linux", test))]
mod x11 {
    use super::*;
    use crate::windows::geometry::Rect;

    #[cfg(target_os = "linux")]
    pub(super) fn capture(position: (i32, i32), size: (u32, u32)) -> Result<image::RgbaImage> {
        let monitors = xcap::Monitor::all()?;
        let geometry = monitors
            .iter()
            .map(|monitor| {
                Ok((
                    Rect {
                        x: monitor.x()?.into(),
                        y: monitor.y()?.into(),
                        width: monitor.width()?.into(),
                        height: monitor.height()?.into(),
                    },
                    f64::from(monitor.scale_factor()?),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let physical = Rect {
            x: position.0.into(),
            y: position.1.into(),
            width: size.0.into(),
            height: size.1.into(),
        };
        let index = resolve_monitor(physical, geometry.into_iter())?;
        monitors[index]
            .capture_image()
            .map_err(|error| format!("Could not capture the screen: {error}").into())
    }

    fn resolve_monitor(
        physical: Rect,
        monitors: impl Iterator<Item = (Rect, f64)>,
    ) -> Result<usize> {
        let mut matching = monitors
            .enumerate()
            .filter_map(|(index, (logical, scale))| {
                if !scale.is_finite() || scale <= 0.0 {
                    return None;
                }
                // XCap truncates each logical origin and extent independently.
                let matches = [
                    (physical.x, logical.x),
                    (physical.y, logical.y),
                    (physical.width, logical.width),
                    (physical.height, logical.height),
                ]
                .into_iter()
                .all(|(physical, logical)| (physical - logical * scale).abs() < scale);
                matches.then_some(index)
            });
        let index = matching
            .next()
            .ok_or("The capture display is unavailable")?;
        if matching.next().is_some() {
            return Err(
                "The capture display is ambiguous. Try capturing on another display.".into(),
            );
        }
        Ok(index)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
            Rect {
                x,
                y,
                width,
                height,
            }
        }

        #[test]
        fn resolves_scaled_bounds_without_reinterpreting_a_physical_cursor_as_logical() {
            let displays = vec![
                (rect(0.0, 0.0, 960.0, 540.0), 2.0),
                (rect(960.0, 0.0, 1280.0, 720.0), 2.0),
            ];
            assert_eq!(
                resolve_monitor(rect(1920.0, 0.0, 2560.0, 1440.0), displays.into_iter()).unwrap(),
                1
            );
        }

        #[test]
        fn permits_fractional_scale_truncation_at_negative_origins() {
            for (physical, logical) in [
                (
                    rect(-2048.0, -77.0, 2048.0, 1152.0),
                    rect(-1365.0, -51.0, 1365.0, 768.0),
                ),
                (
                    rect(2050.0, 77.0, 2050.0, 1152.0),
                    rect(1366.0, 51.0, 1366.0, 768.0),
                ),
            ] {
                assert_eq!(
                    resolve_monitor(physical, [(logical, 1.5)].into_iter()).unwrap(),
                    0
                );
            }
        }

        #[test]
        fn refuses_missing_ambiguous_and_invalid_monitor_geometry() {
            let physical = rect(0.0, 0.0, 1920.0, 1080.0);
            assert!(resolve_monitor(physical, std::iter::empty())
                .unwrap_err()
                .to_string()
                .contains("unavailable"));
            let duplicate = (rect(0.0, 0.0, 960.0, 540.0), 2.0);
            assert!(
                resolve_monitor(physical, [duplicate, duplicate].into_iter())
                    .unwrap_err()
                    .to_string()
                    .contains("ambiguous")
            );
            for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
                assert!(resolve_monitor(physical, [(physical, scale)].into_iter()).is_err());
            }
            assert!(
                resolve_monitor(physical, [(rect(0.0, 0.0, 900.0, 540.0), 2.0)].into_iter())
                    .is_err()
            );
        }
    }
}

// Use the same CoreGraphics backend as XCap, with an explicit window list.
// Exclusion applies only to this screenshot; annotations remain visible in
// other apps' screen sharing. Retain support for macOS versions before SCK.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
mod macos {
    use super::*;
    use objc2_core_foundation::{
        CFArray, CFDictionary, CFNumber, CFNumberType, CFString, CGPoint, CGRect, CGSize,
    };
    use objc2_core_graphics::{
        kCGWindowNumber, kCGWindowOwnerPID, CGDataProvider, CGImage, CGWindowImageOption,
        CGWindowListCopyWindowInfo, CGWindowListCreateImageFromArray, CGWindowListOption,
    };
    use std::ffi::c_void;

    // ABI from Accelerate/vImage_Types.h (macOS arm64 and x86_64).
    #[repr(C)]
    struct ImageBuffer {
        data: *mut c_void,
        height: usize,
        width: usize,
        row_bytes: usize,
    }
    #[link(name = "Accelerate", kind = "framework")]
    unsafe extern "C" {
        fn vImagePermuteChannels_ARGB8888(
            source: *const ImageBuffer,
            destination: *const ImageBuffer,
            permutation: *const u8,
            flags: u32,
        ) -> isize;
    }

    fn number(dictionary: &CFDictionary, key: &CFString) -> Result<u32> {
        // CGWindowListCopyWindowInfo documents these dictionary values as CFNumbers.
        let value = unsafe { dictionary.value((key as *const CFString).cast()) };
        if value.is_null() {
            return Err("Could not read the capture window list".into());
        }
        let mut number: i64 = 0;
        let read = unsafe {
            (&*value.cast::<CFNumber>())
                .value(CFNumberType::SInt64Type, (&mut number as *mut i64).cast())
        };
        if !read {
            return Err("Could not read a capture window ID".into());
        }
        Ok(u32::try_from(number)?)
    }

    pub(super) fn capture(position: (i32, i32), size: (u32, u32)) -> Result<image::RgbaImage> {
        let windows = CGWindowListCopyWindowInfo(CGWindowListOption::OptionOnScreenOnly, 0)
            .ok_or("Could not list the screen's windows")?;
        let mut included: Vec<*const c_void> = Vec::new();
        for index in 0..windows.count() {
            // The retained array contains CFDictionary objects in front-to-back order.
            let dictionary = unsafe { &*windows.value_at_index(index).cast::<CFDictionary>() };
            if number(dictionary, unsafe { kCGWindowOwnerPID })? != std::process::id() {
                let id = number(dictionary, unsafe { kCGWindowNumber })?;
                // This API expects integer window IDs encoded as pointer values,
                // not CFNumber objects. Null callbacks prevent retain/release.
                included.push(id as usize as *const c_void);
            }
        }
        let window_array = unsafe {
            CFArray::new(
                None,
                included.as_mut_ptr(),
                included.len() as isize,
                std::ptr::null(),
            )
        }
        .ok_or("Could not create the capture window list")?;
        let bounds = CGRect::new(
            CGPoint::new(position.0 as f64, position.1 as f64),
            CGSize::new(size.0 as f64, size.1 as f64),
        );
        let image = unsafe {
            CGWindowListCreateImageFromArray(bounds, &window_array, CGWindowImageOption::Default)
        }
        .ok_or("Could not capture the screen")?;
        let width = CGImage::width(Some(&image));
        let height = CGImage::height(Some(&image));
        if CGImage::bits_per_pixel(Some(&image)) != 32
            || CGImage::bits_per_component(Some(&image)) != 8
        {
            return Err("The screen returned an unsupported pixel format".into());
        }
        let provider =
            CGImage::data_provider(Some(&image)).ok_or("The screenshot has no pixels")?;
        let data =
            CGDataProvider::data(Some(&provider)).ok_or("Could not read the screenshot pixels")?;
        // This copied CFData is immutable and stays retained through conversion.
        let bytes = unsafe { data.as_bytes_unchecked() };
        bgra_image(bytes, width, height, CGImage::bytes_per_row(Some(&image)))
    }

    fn bgra_image(
        bytes: &[u8],
        width: usize,
        height: usize,
        stride: usize,
    ) -> Result<image::RgbaImage> {
        let row = width.checked_mul(4).ok_or("The screenshot is too large")?;
        let length = row
            .checked_mul(height)
            .ok_or("The screenshot is too large")?;
        if width == 0
            || height == 0
            || length > 256 * 1024 * 1024
            || stride < row
            || stride
                .checked_mul(height)
                .is_none_or(|needed| needed > bytes.len())
        {
            return Err("The screenshot has invalid pixel dimensions".into());
        }
        let mut pixels = vec![0; length];
        let source = ImageBuffer {
            data: bytes.as_ptr().cast_mut().cast(),
            height,
            width,
            row_bytes: stride,
        };
        let destination = ImageBuffer {
            data: pixels.as_mut_ptr().cast(),
            height,
            width,
            row_bytes: row,
        };
        // vImage reads the immutable source and writes a separate, fully sized
        // destination. It strips row padding and swaps B/R in one native pass,
        // keeping debug builds fast without a Rust loop over every pixel.
        let status = unsafe {
            vImagePermuteChannels_ARGB8888(&source, &destination, [2, 1, 0, 3].as_ptr(), 0)
        };
        if status != 0 {
            return Err(format!("Could not convert screenshot pixels: {status}").into());
        }
        image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or("Could not prepare the screenshot pixels".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn converts_bgra_rows_without_copying_padding() {
            let bytes = [
                30, 20, 10, 255, 60, 50, 40, 128, 99, 99, 99, 99, 90, 80, 70, 255, 120, 110, 100,
                128, 99, 99, 99, 99,
            ];
            let image = bgra_image(&bytes, 2, 2, 12).unwrap();
            assert_eq!(
                image.as_raw(),
                &[10, 20, 30, 255, 40, 50, 60, 128, 70, 80, 90, 255, 100, 110, 120, 128]
            );
            for (width, height, stride) in [
                (0, 2, 12),
                (2, 0, 12),
                (4, 2, 12),
                (2, 3, 12),
                (usize::MAX, 2, 12),
            ] {
                assert!(bgra_image(&bytes, width, height, stride).is_err());
            }
        }
    }
}
