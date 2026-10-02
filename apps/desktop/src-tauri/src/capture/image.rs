use crate::Result;
use image::RgbaImage;
use std::io::Cursor;

/// Decoded pixels a capture or an annotated copy may occupy.
pub(super) const MAX_IMAGE_BYTES: usize = 256 * 1024 * 1024;

// Binary frame: little-endian u32 width and height, followed by full-resolution RGBA.
// Avoid compression, base64, JSON strings, and image decoding before region selection.
pub(super) fn frame(image: &RgbaImage) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + image.as_raw().len());
    frame.extend_from_slice(&image.width().to_le_bytes());
    frame.extend_from_slice(&image.height().to_le_bytes());
    frame.extend_from_slice(image.as_raw());
    frame
}

#[derive(Clone, Copy, serde::Deserialize)]
pub(crate) struct CaptureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// `crop_imm` clamps a region that overruns the image, so the bounds are checked first.
pub(super) fn crop(image: &RgbaImage, region: CaptureRegion) -> Result<RgbaImage> {
    let fits = |start: u32, length: u32, max: u32| {
        length > 0 && start.checked_add(length).is_some_and(|end| end <= max)
    };
    if !fits(region.x, region.width, image.width())
        || !fits(region.y, region.height, image.height())
    {
        return Err("The selected capture region is invalid".into());
    }
    if u64::from(region.width) * u64::from(region.height) * 4 > MAX_IMAGE_BYTES as u64 {
        return Err("The capture is too large to copy".into());
    }
    Ok(
        image::imageops::crop_imm(image, region.x, region.y, region.width, region.height)
            .to_image(),
    )
}

pub(super) fn decode_png(png: &[u8]) -> Result<RgbaImage> {
    let mut reader = image::ImageReader::with_format(Cursor::new(png), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_IMAGE_BYTES as u64);
    reader.limits(limits);
    reader
        .decode()
        .map(|image| image.into_rgba8())
        .map_err(|e| format!("Could not read the annotated image: {e}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_preserves_pixels_and_includes_the_last_row_and_column() {
        let source = RgbaImage::from_fn(7, 5, |x, y| {
            image::Rgba([x as u8, y as u8, (x * y) as u8, 128])
        });
        let region = CaptureRegion {
            x: 3,
            y: 2,
            width: 4,
            height: 3,
        };
        let cropped = crop(&source, region).unwrap();
        assert_eq!(cropped.dimensions(), (4, 3));
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(cropped.get_pixel(x, y), source.get_pixel(x + 3, y + 2));
            }
        }
        assert_eq!(
            crop(
                &source,
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
    fn crop_rejects_empty_overflowing_or_outside_regions() {
        let source = RgbaImage::new(7, 5);
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
            assert!(crop(&source, region).is_err());
        }
        for json in [
            r#"{"x":-1,"y":0,"width":1,"height":1}"#,
            r#"{"x":0.5,"y":0,"width":1,"height":1}"#,
        ] {
            assert!(serde_json::from_str::<CaptureRegion>(json).is_err());
        }
    }

    #[test]
    fn crop_rejects_regions_larger_than_the_image_limit() {
        // Zeroed buffers are mapped lazily, so this source costs almost nothing until read.
        let source = RgbaImage::new(8193, 8193);
        let region = CaptureRegion {
            x: 0,
            y: 0,
            width: 8193,
            height: 8193,
        };
        assert_eq!(
            crop(&source, region)
                .err()
                .map(|e| e.to_string())
                .as_deref(),
            Some("The capture is too large to copy")
        );
    }

    #[test]
    fn png_decoding_preserves_pixels_and_rejects_garbage() {
        let original = RgbaImage::from_pixel(2, 3, image::Rgba([11, 22, 33, 128]));
        let mut png = Cursor::new(Vec::new());
        original
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        assert_eq!(decode_png(&png.into_inner()).unwrap(), original);
        assert!(decode_png(b"not an image").is_err());
    }

    #[test]
    fn binary_frame_preserves_dimensions_and_rgba_byte_order() {
        let original = RgbaImage::from_fn(17, 11, |x, y| {
            image::Rgba([
                (x * 15) as u8,
                (y * 23) as u8,
                (x * y) as u8,
                ((x + y) * 9) as u8,
            ])
        });
        let frame = frame(&original);
        assert_eq!(&frame[..4], &17u32.to_le_bytes());
        assert_eq!(&frame[4..8], &11u32.to_le_bytes());
        assert_eq!(&frame[8..], original.as_raw());
    }
}
