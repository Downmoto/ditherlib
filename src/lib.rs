use std::{fmt, path::Path};

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
}

/// An error returned by a Ditherlib operation.
#[derive(Debug)]
pub struct DitherError {
    kind: ErrorKind,
    source: image::ImageError,
}

impl DitherError {
    /// Returns the stable category for this error.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    fn from_image(source: image::ImageError) -> Self {
        let kind = match &source {
            image::ImageError::IoError(_) => ErrorKind::FileAccess,
            image::ImageError::Unsupported(_) => ErrorKind::UnsupportedFormat,
            _ => ErrorKind::Decode,
        };

        Self { kind, source }
    }
}

impl fmt::Display for DitherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let context = match self.kind {
            ErrorKind::FileAccess => "could not access image file",
            ErrorKind::UnsupportedFormat => "unsupported image format",
            ErrorKind::Decode => "could not decode image",
        };

        write!(formatter, "{context}: {}", self.source)
    }
}

impl std::error::Error for DitherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// An image loaded into memory by Ditherlib.
pub struct SourceImage {
    inner: image::DynamicImage,
}

impl SourceImage {
    /// Returns the image width in pixels.
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Returns the image height in pixels.
    pub fn height(&self) -> u32 {
        self.inner.height()
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
        .map(|inner| SourceImage { inner })
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

        assert!(image.width() > 0);
        assert!(image.height() > 0);
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
