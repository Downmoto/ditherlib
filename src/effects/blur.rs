use crate::{DitherError, Effect, ErrorKind, Mask, Result};

/// A Gaussian blur measured by its standard deviation in pixels.
///
/// The blur reads from the entire input image before the renderer composites
/// the selected area. Pixels outside a selection can therefore influence the
/// blurred pixels inside it. RGB channels are blurred directly and every alpha
/// value is preserved from the source pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blur {
    sigma: f32,
}

impl Blur {
    /// Creates a Gaussian blur. A sigma of zero produces an unchanged image.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `sigma` is negative,
    /// non-finite, or subnormal.
    pub fn new(sigma: f32) -> Result<Self> {
        if sigma != 0.0 && (!sigma.is_normal() || sigma.is_sign_negative()) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "blur sigma must be zero or a positive normal number",
            ));
        }

        Ok(Self { sigma })
    }

    /// Returns the Gaussian standard deviation in pixels.
    pub const fn sigma(&self) -> f32 {
        self.sigma
    }
}

impl Effect for Blur {
    fn apply(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        _mask: &Mask,
    ) -> Result<()> {
        if self.sigma == 0.0 {
            return Ok(());
        }

        let Some(image) = image::RgbaImage::from_raw(dimensions.0, dimensions.1, input.to_vec())
        else {
            return Err(DitherError::new(
                ErrorKind::DimensionMismatch,
                "effect input does not match its image dimensions",
            ));
        };
        let blurred = image::imageops::blur(&image, self.sigma);

        if output.len() != blurred.as_raw().len() {
            return Err(DitherError::new(
                ErrorKind::DimensionMismatch,
                "effect output does not match its image dimensions",
            ));
        }

        output.copy_from_slice(blurred.as_raw());
        for (input, output) in input
            .as_chunks::<4>()
            .0
            .iter()
            .zip(output.as_chunks_mut::<4>().0)
        {
            output[3] = input[3];
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Blur;
    use crate::{ErrorKind, Point, Polygon, Renderer, Selection, SourceImage};

    /// Creates an immutable image from test pixels.
    fn source(width: u32, height: u32, pixels: &[[u8; 4]]) -> SourceImage {
        SourceImage {
            width,
            height,
            pixels: pixels
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    #[test]
    fn validates_sigma() {
        assert_eq!(Blur::new(0.0).unwrap().sigma(), 0.0);
        assert_eq!(Blur::new(1.5).unwrap().sigma(), 1.5);

        for invalid in [-1.0, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE / 2.0] {
            assert_eq!(
                Blur::new(invalid).unwrap_err().kind(),
                ErrorKind::InvalidParameter
            );
        }
    }

    #[test]
    fn zero_sigma_leaves_pixels_unchanged() {
        let source = source(2, 1, &[[1, 2, 3, 4], [5, 6, 7, 8]]);
        let rendered = Renderer::new()
            .render(&source, &Blur::new(0.0).unwrap(), &Selection::All)
            .unwrap();

        assert_eq!(rendered.rgba8_bytes(), source.rgba8_bytes());
    }

    #[test]
    fn blurs_across_image_boundaries_and_preserves_alpha() {
        let source = source(3, 1, &[[0, 0, 0, 10], [255, 255, 255, 20], [0, 0, 0, 30]]);
        let rendered = Renderer::new()
            .render(&source, &Blur::new(1.0).unwrap(), &Selection::All)
            .unwrap();
        let left = rendered.pixel(0, 0).unwrap();
        let centre = rendered.pixel(1, 0).unwrap();
        let right = rendered.pixel(2, 0).unwrap();

        assert_eq!(&left[..3], &right[..3]);
        assert!(left[0] > 0);
        assert!(centre[0] < 255);
        assert!(centre[0] > left[0]);
        assert_eq!([left[3], centre[3], right[3]], [10, 20, 30]);
    }

    #[test]
    fn uses_exterior_samples_but_preserves_exterior_pixels() {
        let mut pixels = [[255, 255, 255, 90]; 9];
        pixels[4] = [0, 0, 0, 40];
        let source = source(3, 3, &pixels);
        let original = source.rgba8_bytes().to_vec();
        let polygon = Polygon::new([
            Point::new(1.0, 1.0),
            Point::new(2.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(1.0, 2.0),
        ])
        .unwrap();
        let rendered = Renderer::new()
            .render(
                &source,
                &Blur::new(1.0).unwrap(),
                &Selection::Polygon(polygon),
            )
            .unwrap();

        assert!(rendered.pixel(1, 1).unwrap()[0] > 0);
        assert_eq!(rendered.pixel(1, 1).unwrap()[3], 40);
        for y in 0..3 {
            for x in 0..3 {
                if (x, y) != (1, 1) {
                    assert_eq!(rendered.pixel(x, y), Some([255, 255, 255, 90]));
                }
            }
        }
        assert_eq!(source.rgba8_bytes(), original);
    }
}
