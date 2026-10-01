use crate::Result;

/// Capture after native exclusion/synchronization, without a fixed startup delay.
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (position, size);
        #[cfg(target_os = "windows")]
        {
            // Window hide requests are queued. A main-thread barrier ensures they
            // run before the compositor flush; the capture worker waits for both.
            let (send, receive) = std::sync::mpsc::channel();
            app.run_on_main_thread(move || {
                let status = unsafe { windows_sys::Win32::Graphics::Dwm::DwmFlush() };
                let _ = send.send(status);
            })
            .map_err(|e| e.to_string())?;
            let status = receive.recv().map_err(|e| e.to_string())?;
            if status < 0 {
                // Preserve capture on systems where compositor synchronization fails.
                eprintln!("Compositor flush failed ({status:#x}); using capture delay");
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = app;
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
        // Windows monitor handles stay on this worker, rather than crossing threads.
        xcap::Monitor::from_point(point.0, point.1)
            .and_then(|monitor| monitor.capture_image())
            .map_err(|e| format!("Could not capture the screen: {e}"))
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
        u32::try_from(number).map_err(|e| e.to_string())
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
            return Err(format!("Could not convert screenshot pixels: {status}"));
        }
        image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or("Could not prepare the screenshot pixels".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "manual full-resolution native pixel conversion benchmark"]
        fn benchmark_bgra_conversion() {
            let width = 3840;
            let height = 2160;
            let bytes = vec![127; width * height * 4];
            let mut samples = Vec::new();
            for _ in 0..5 {
                let started = std::time::Instant::now();
                let image = bgra_image(&bytes, width, height, width * 4).unwrap();
                samples.push(started.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(image.as_raw(), &bytes);
            }
            samples.sort_by(f64::total_cmp);
            eprintln!("Native 4K BGRA conversion: median {:.2} ms", samples[2]);
        }
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
