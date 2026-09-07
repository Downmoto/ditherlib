use crate::{DitherError, Effect, ErrorKind, Mask, Result};

/// A fast three-pass approximation of Gaussian blur.
///
/// The blur reads the selected bounds plus a sampling margin before the
/// renderer composites the selected area. Nearby pixels outside a selection
/// can therefore influence the blurred pixels inside it. RGB channels are
/// blurred directly and every alpha value is preserved from the source pixel.
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
        mask: &Mask,
    ) -> Result<()> {
        if self.sigma == 0.0 {
            return Ok(());
        }

        let expected_length = rgba8_length(dimensions);
        if mask.dimensions() != dimensions
            || expected_length != Some(input.len())
            || expected_length != Some(output.len())
        {
            return Err(DitherError::new(
                ErrorKind::DimensionMismatch,
                "blur buffers and mask must share image dimensions",
            ));
        }
        let Some(bounds) = mask.coverage_bounds() else {
            return Ok(());
        };
        let crop_bounds = expand_bounds(bounds, dimensions, sampling_margin(self.sigma));
        let image = crop_image(input, dimensions.0, crop_bounds)?;
        let blurred = image::imageops::fast_blur(&image, self.sigma);

        copy_blurred_selection(
            input,
            output,
            dimensions.0,
            bounds,
            crop_bounds,
            blurred.as_raw(),
        );

        Ok(())
    }
}

type Bounds = (u32, u32, u32, u32);

/// Calculates the addressable byte length of an RGBA8 image.
fn rgba8_length(dimensions: (u32, u32)) -> Option<usize> {
    u64::from(dimensions.0)
        .checked_mul(u64::from(dimensions.1))?
        .checked_mul(4)?
        .try_into()
        .ok()
}

/// Returns a conservative sampling margin for the three box-blur passes.
fn sampling_margin(sigma: f32) -> u32 {
    ((sigma * 4.0).ceil() as u32).saturating_add(3)
}

/// Expands selection bounds by a sampling margin and clips them to the image.
fn expand_bounds(bounds: Bounds, dimensions: (u32, u32), margin: u32) -> Bounds {
    (
        bounds.0.saturating_sub(margin),
        bounds.1.saturating_sub(margin),
        bounds.2.saturating_add(margin).min(dimensions.0),
        bounds.3.saturating_add(margin).min(dimensions.1),
    )
}

/// Copies a rectangular region from an RGBA8 image into a private image buffer.
fn crop_image(input: &[u8], image_width: u32, bounds: Bounds) -> Result<image::RgbaImage> {
    let width = bounds.2 - bounds.0;
    let height = bounds.3 - bounds.1;
    let row_length = width as usize * 4;
    let mut pixels = Vec::with_capacity(row_length * height as usize);

    for y in bounds.1..bounds.3 {
        let start = (y as usize * image_width as usize + bounds.0 as usize) * 4;
        pixels.extend_from_slice(&input[start..start + row_length]);
    }

    image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
        DitherError::new(
            ErrorKind::DimensionMismatch,
            "blur input does not match its image dimensions",
        )
    })
}

/// Copies blurred RGB values for the selected bounds while preserving alpha.
fn copy_blurred_selection(
    input: &[u8],
    output: &mut [u8],
    image_width: u32,
    selection: Bounds,
    crop: Bounds,
    blurred: &[u8],
) {
    let crop_width = crop.2 - crop.0;

    for y in selection.1..selection.3 {
        for x in selection.0..selection.2 {
            let image_index = (y as usize * image_width as usize + x as usize) * 4;
            let crop_index =
                ((y - crop.1) as usize * crop_width as usize + (x - crop.0) as usize) * 4;
            output[image_index..image_index + 3]
                .copy_from_slice(&blurred[crop_index..crop_index + 3]);
            output[image_index + 3] = input[image_index + 3];
        }
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
        assert!(centre[0] > 0);
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

    #[test]
    fn cropped_blur_matches_full_image_blur_inside_the_selection() {
        let pixels = (0..900)
            .map(|index| {
                let value = (index % 251) as u8;
                [value, value.wrapping_add(40), value.wrapping_add(80), 123]
            })
            .collect::<Vec<_>>();
        let source = source(30, 30, &pixels);
        let full_image = image::RgbaImage::from_raw(30, 30, source.rgba8_bytes().to_vec()).unwrap();
        let full_blur = image::imageops::fast_blur(&full_image, 1.0);
        let polygon = Polygon::new([
            Point::new(14.0, 14.0),
            Point::new(16.0, 14.0),
            Point::new(16.0, 16.0),
            Point::new(14.0, 16.0),
        ])
        .unwrap();
        let rendered = Renderer::new()
            .render(
                &source,
                &Blur::new(1.0).unwrap(),
                &Selection::Polygon(polygon),
            )
            .unwrap();

        for y in 14..16 {
            for x in 14..16 {
                let expected = full_blur.get_pixel(x, y).0;
                assert_eq!(rendered.pixel(x, y).unwrap()[..3], expected[..3]);
                assert_eq!(rendered.pixel(x, y).unwrap()[3], 123);
            }
        }
    }
}
