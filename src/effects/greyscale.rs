use crate::{Effect, Mask, Result};

/// Converts RGB channels to greyscale while preserving alpha.
///
/// Luminance is calculated directly from 8-bit channel values using Rec. 709
/// weights and rounded to the nearest integer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Greyscale;

impl Effect for Greyscale {
    fn apply(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) -> Result<()> {
        let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
            return Ok(());
        };
        let width = dimensions.0 as usize;

        for y in min_y..max_y {
            for x in min_x..max_x {
                let pixel_index = y as usize * width + x as usize;
                if mask.coverage_bytes()[pixel_index] == 0 {
                    continue;
                }
                let byte_index = pixel_index * 4;
                let input = &input[byte_index..byte_index + 4];

                // ITU-R BT.709-6 section 3.2 defines luma weights of 0.2126,
                // 0.7152, and 0.0722. Scaling by 10,000 keeps this integer-only;
                // adding half the scale rounds the result to the nearest value.
                let luminance = (u32::from(input[0]) * 2126
                    + u32::from(input[1]) * 7152
                    + u32::from(input[2]) * 722
                    + 5000)
                    / 10_000;
                let luminance = luminance as u8;

                output[byte_index..byte_index + 4]
                    .copy_from_slice(&[luminance, luminance, luminance, input[3]]);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Greyscale;
    use crate::{Point, Polygon, Renderer, Selection, SourceImage};

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
    fn converts_exact_pixels_and_preserves_alpha() {
        let source = source(
            4,
            1,
            &[
                [255, 0, 0, 1],
                [0, 255, 0, 2],
                [0, 0, 255, 3],
                [10, 20, 30, 4],
            ],
        );
        let rendered = Renderer::new()
            .render(&source, &Greyscale, &Selection::All)
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([54, 54, 54, 1]));
        assert_eq!(rendered.pixel(1, 0), Some([182, 182, 182, 2]));
        assert_eq!(rendered.pixel(2, 0), Some([18, 18, 18, 3]));
        assert_eq!(rendered.pixel(3, 0), Some([19, 19, 19, 4]));
    }

    #[test]
    fn converts_only_the_selected_polygon() {
        let original = [[10, 20, 30, 40]; 9];
        let source = source(3, 3, &original);
        let polygon = Polygon::new([
            Point::new(1.0, 1.0),
            Point::new(2.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(1.0, 2.0),
        ])
        .unwrap();
        let original_bytes = source.rgba8_bytes().to_vec();
        let rendered = Renderer::new()
            .render(&source, &Greyscale, &Selection::Polygon(polygon))
            .unwrap();

        for y in 0..3 {
            for x in 0..3 {
                let expected = if (x, y) == (1, 1) {
                    [19, 19, 19, 40]
                } else {
                    [10, 20, 30, 40]
                };
                assert_eq!(rendered.pixel(x, y), Some(expected));
            }
        }
        assert_eq!(source.rgba8_bytes(), original_bytes);
    }
}
