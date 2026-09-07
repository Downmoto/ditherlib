use std::path::Path;

use crate::{DitherError, RenderedImage, Result, SourceImage};

/// Reads and decodes an image from disk.
///
/// The image format is determined from the path's file extension and must be
/// enabled through the corresponding crate feature. JPEG and PNG support are
/// enabled by default.
///
/// # Errors
///
/// Returns [`crate::ErrorKind::FileAccess`] when the file cannot be opened,
/// [`crate::ErrorKind::UnsupportedFormat`] when its format is unavailable, and
/// [`crate::ErrorKind::Decode`] when its contents cannot be decoded.
pub fn read(path: impl AsRef<Path>) -> Result<SourceImage> {
    image::open(path)
        .map(|image| {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();

            SourceImage {
                width,
                height,
                pixels: image.into_raw().into_boxed_slice(),
            }
        })
        .map_err(DitherError::from_image_decode)
}

/// Encodes a rendered image to a file.
///
/// The output format is determined from the path's file extension and must be
/// enabled through the corresponding crate feature. JPEG output discards the
/// alpha channel because the format does not support transparency.
///
/// # Errors
///
/// Returns [`crate::ErrorKind::FileAccess`] when the file cannot be written,
/// [`crate::ErrorKind::UnsupportedFormat`] when its format is unavailable, and
/// [`crate::ErrorKind::Encode`] when its pixels cannot be encoded.
pub fn write(path: impl AsRef<Path>, image: &RenderedImage) -> Result<()> {
    let path = path.as_ref();
    let format = image::ImageFormat::from_path(path).map_err(DitherError::from_image_encode)?;

    let result = if format == image::ImageFormat::Jpeg {
        let pixels = rgb8_bytes(image.rgba8_bytes());
        image::save_buffer_with_format(
            path,
            &pixels,
            image.width(),
            image.height(),
            image::ColorType::Rgb8,
            format,
        )
    } else {
        image::save_buffer_with_format(
            path,
            image.rgba8_bytes(),
            image.width(),
            image.height(),
            image::ColorType::Rgba8,
            format,
        )
    };

    result.map_err(DitherError::from_image_encode)
}

/// Removes alpha bytes from row-major RGBA8 pixels.
fn rgb8_bytes(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);

    for pixel in rgba.as_chunks::<4>().0 {
        rgb.extend_from_slice(&pixel[..3]);
    }

    rgb
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[cfg(any(feature = "jpeg", feature = "png"))]
    use super::read;
    use super::write;
    #[cfg(not(feature = "png"))]
    use crate::ErrorKind;
    use crate::{Greyscale, Renderer, Selection, SourceImage};

    /// Creates a unique path in the system temporary directory.
    fn temporary_path(extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "ditherlib-{}-{nonce}.{extension}",
            std::process::id()
        ))
    }

    /// Creates a rendered two-pixel image for encoding tests.
    fn rendered_image() -> crate::RenderedImage {
        let source = SourceImage {
            width: 2,
            height: 1,
            pixels: [10, 20, 30, 40, 200, 150, 100, 50].into_iter().collect(),
        };

        Renderer::new()
            .render(&source, &Greyscale, &Selection::All)
            .unwrap()
    }

    #[cfg(feature = "png")]
    #[test]
    fn round_trips_png_pixels_and_alpha() {
        let path = temporary_path("png");
        let rendered = rendered_image();

        write(&path, &rendered).unwrap();
        let decoded = read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(decoded.dimensions(), rendered.dimensions());
        assert_eq!(decoded.rgba8_bytes(), rendered.rgba8_bytes());
    }

    #[cfg(feature = "jpeg")]
    #[test]
    fn writes_and_reads_jpeg_without_alpha() {
        let path = temporary_path("jpg");
        let rendered = rendered_image();

        write(&path, &rendered).unwrap();
        let decoded = read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(decoded.dimensions(), rendered.dimensions());
        assert_eq!(decoded.pixel(0, 0).unwrap()[3], 255);
        assert_eq!(decoded.pixel(1, 0).unwrap()[3], 255);
    }

    #[cfg(not(feature = "png"))]
    #[test]
    fn reports_a_disabled_encoder() {
        let path = temporary_path("png");
        let error = write(&path, &rendered_image()).expect_err("PNG support should be disabled");
        let _ = std::fs::remove_file(path);

        assert_eq!(error.kind(), ErrorKind::UnsupportedFormat);
    }
}
