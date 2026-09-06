use std::path::Path;

/// Reads and decodes an image from disk.
pub fn read(path: impl AsRef<Path>) -> image::ImageResult<image::DynamicImage> {
    image::open(path)
}

#[cfg(all(test, feature = "jpeg"))]
mod tests {
    use super::read;
    use std::path::Path;

    #[test]
    fn reads_jpeg() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test.JPG");
        let image = read(path).expect("test.JPEG should be a valid JPEG image");

        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }
}
