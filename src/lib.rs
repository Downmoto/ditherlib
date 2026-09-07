use std::{fmt, path::Path};

mod effects;
mod renderer;
mod selection;

pub use effects::Greyscale;
pub use renderer::{Effect, RenderedImage, Renderer};
pub use selection::{Mask, Point, Polygon, Selection};

/// A result returned by Ditherlib operations.
pub type Result<T> = std::result::Result<T, DitherError>;

/// A category describing why a Ditherlib operation failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An image file could not be accessed.
    FileAccess,
    /// The requested image format is unsupported or disabled.
    UnsupportedFormat,
    /// An image could not be decoded.
    Decode,
    /// A polygon is malformed.
    InvalidPolygon,
    /// A mask's dimensions and coverage length do not agree.
    DimensionMismatch,
    /// An effect could not be rendered.
    Effect,
}

/// An error returned by a Ditherlib operation.
#[derive(Debug)]
pub struct DitherError {
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl DitherError {
    /// Returns the stable category for this error.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Creates an error with a stable category and descriptive message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    fn from_image(source: image::ImageError) -> Self {
        let kind = match &source {
            image::ImageError::IoError(_) => ErrorKind::FileAccess,
            image::ImageError::Unsupported(_) => ErrorKind::UnsupportedFormat,
            _ => ErrorKind::Decode,
        };
        let message = match kind {
            ErrorKind::FileAccess => "could not access image file",
            ErrorKind::UnsupportedFormat => "unsupported image format",
            ErrorKind::Decode => "could not decode image",
            _ => unreachable!(),
        };

        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for DitherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(formatter, "{}: {source}", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for DitherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// An image loaded into memory by Ditherlib.
pub struct SourceImage {
    width: u32,
    height: u32,
    pixels: Box<[u8]>,
}

impl SourceImage {
    /// Returns the image width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the image height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the image dimensions as `(width, height)`.
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the row-major RGBA8 pixel bytes.
    pub fn rgba8_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns the RGBA8 value at `(x, y)`, or `None` when it is out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = (y as usize * self.width as usize + x as usize) * 4;
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ])
    }
}

impl fmt::Debug for SourceImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceImage")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

/// Reads and decodes an image from disk.
///
/// The image format is determined from the path's file extension and must be
/// enabled through the corresponding crate feature. JPEG and PNG support are
/// enabled by default.
///
/// # Errors
///
/// Returns [`ErrorKind::FileAccess`] when the file cannot be opened,
/// [`ErrorKind::UnsupportedFormat`] when its format is unavailable, and
/// [`ErrorKind::Decode`] when its contents cannot be decoded.
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
        .map_err(DitherError::from_image)
}

#[cfg(test)]
mod tests {
    use super::{ErrorKind, read};
    use std::path::Path;

    #[cfg(feature = "jpeg")]
    #[test]
    fn reads_jpeg() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/valid.jpg");
        let image = read(path).expect("valid.jpg should be a valid JPEG image");

        assert_eq!(image.width(), 3000);
        assert_eq!(image.height(), 4496);
        assert_eq!(image.dimensions(), (3000, 4496));

        let pixels: &[u8] = image.rgba8_bytes();
        assert_eq!(pixels.len(), 3000 * 4496 * 4);
        assert_eq!(image.pixel(0, 0), Some([164, 59, 4, 255]));
        assert_eq!(image.pixel(1500, 2248), Some([148, 164, 226, 255]));
        assert_eq!(image.pixel(2999, 4495), Some([255, 254, 215, 255]));
        assert_eq!(image.pixel(3000, 0), None);
        assert_eq!(image.pixel(0, 4496), None);
    }

    #[test]
    fn reports_file_access_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing.jpg");
        let error = read(path).expect_err("missing.jpg should not exist");

        assert_eq!(error.kind(), ErrorKind::FileAccess);
    }

    #[test]
    fn reports_unsupported_format_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let error = read(path).expect_err("Cargo.toml should not be a supported image format");

        assert_eq!(error.kind(), ErrorKind::UnsupportedFormat);
    }

    #[cfg(feature = "jpeg")]
    #[test]
    fn reports_decode_errors() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/invalid.jpg");
        let error = read(path).expect_err("invalid.jpg should not decode");

        assert_eq!(error.kind(), ErrorKind::Decode);
    }
}
