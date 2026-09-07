use crate::{DitherError, Effect, ErrorKind, Mask, Result};

/// A non-empty collection of RGB colours available to a dithering effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Palette {
    colours: Box<[[u8; 3]]>,
}

impl Palette {
    /// Creates a palette from at least one RGB colour.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `colours` is empty.
    pub fn new(colours: impl Into<Box<[[u8; 3]]>>) -> Result<Self> {
        let colours = colours.into();

        if colours.is_empty() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "a palette requires at least one colour",
            ));
        }

        Ok(Self { colours })
    }

    /// Creates a palette containing black and white.
    pub fn monochrome() -> Self {
        Self {
            colours: Box::new([[0, 0, 0], [255, 255, 255]]),
        }
    }

    /// Returns the palette's RGB colours in matching order.
    pub fn colours(&self) -> &[[u8; 3]] {
        &self.colours
    }

    /// Returns the nearest palette colour to `colour`.
    ///
    /// Matching uses squared Euclidean distance in RGB byte space. When two
    /// entries are equally close, the earlier palette entry wins.
    pub fn nearest_colour(&self, colour: [u8; 3]) -> [u8; 3] {
        *self
            .colours
            .iter()
            .min_by_key(|candidate| colour_distance(colour, **candidate))
            .expect("a palette is always non-empty")
    }
}

/// Deterministically maps each selected pixel to its nearest palette colour.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Threshold {
    palette: Palette,
}

impl Threshold {
    /// Creates threshold dithering with the supplied palette.
    pub const fn new(palette: Palette) -> Self {
        Self { palette }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }
}

impl Effect for Threshold {
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

                let colour = self.palette.nearest_colour([input[0], input[1], input[2]]);
                output[byte_index..byte_index + 4]
                    .copy_from_slice(&[colour[0], colour[1], colour[2], input[3]]);
            }
        }

        Ok(())
    }
}

/// Applies ordered dithering using a Bayer threshold matrix.
///
/// The matrix is anchored to the image origin, including when rendering a
/// polygon selection. Supported matrix sizes are 2, 4, and 8 pixels square.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedDither {
    palette: Palette,
    matrix_size: u8,
}

impl OrderedDither {
    /// Creates ordered dithering with a supported Bayer matrix size.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] unless `matrix_size` is 2, 4,
    /// or 8.
    pub fn new(palette: Palette, matrix_size: u8) -> Result<Self> {
        if !matches!(matrix_size, 2 | 4 | 8) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "Bayer matrix size must be 2, 4, or 8",
            ));
        }

        Ok(Self {
            palette,
            matrix_size,
        })
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the width and height of the square Bayer matrix.
    pub const fn matrix_size(&self) -> u8 {
        self.matrix_size
    }
}

impl Effect for OrderedDither {
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
                let colour = adjust_colour(
                    [input[0], input[1], input[2]],
                    bayer_value(x, y, self.matrix_size),
                    self.matrix_size,
                );
                let colour = self.palette.nearest_colour(colour);

                output[byte_index..byte_index + 4]
                    .copy_from_slice(&[colour[0], colour[1], colour[2], input[3]]);
            }
        }

        Ok(())
    }
}

/// Adjusts RGB channels around the Bayer threshold before palette matching.
fn adjust_colour(colour: [u8; 3], threshold: u8, matrix_size: u8) -> [u8; 3] {
    let levels = i32::from(matrix_size).pow(2);
    let adjustment = (2 * i32::from(threshold) + 1 - levels) * 255 / (2 * levels);

    colour.map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8)
}

/// Returns a standard recursive Bayer threshold anchored at `(0, 0)`.
fn bayer_value(x: u32, y: u32, size: u8) -> u8 {
    if size == 1 {
        return 0;
    }

    let x = x % u32::from(size);
    let y = y % u32::from(size);
    let half = u32::from(size / 2);
    let offset = match (x / half, y / half) {
        (0, 0) => 0,
        (1, 0) => 2,
        (0, 1) => 3,
        (1, 1) => 1,
        _ => unreachable!("coordinates are reduced to the Bayer matrix size"),
    };

    4 * bayer_value(x % half, y % half, size / 2) + offset
}

/// Calculates squared Euclidean distance between two RGB colours.
fn colour_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| {
            let difference = i32::from(left) - i32::from(right);
            (difference * difference) as u32
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{OrderedDither, Palette, Threshold, bayer_value};
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
    fn validates_palette_and_matches_custom_colours() {
        let error = Palette::new(Vec::<[u8; 3]>::new()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidParameter);

        let palette = Palette::new([[255, 0, 0], [0, 0, 255]]).unwrap();
        assert_eq!(palette.colours(), &[[255, 0, 0], [0, 0, 255]]);
        assert_eq!(palette.nearest_colour([200, 10, 40]), [255, 0, 0]);
        assert_eq!(palette.nearest_colour([40, 10, 200]), [0, 0, 255]);
        assert_eq!(palette.nearest_colour([127, 0, 127]), [255, 0, 0]);
    }

    #[test]
    fn thresholds_exact_monochrome_pixels_and_preserves_alpha() {
        let source = source(
            3,
            1,
            &[[10, 20, 30, 1], [128, 128, 128, 2], [245, 235, 225, 3]],
        );
        let rendered = Renderer::new()
            .render(
                &source,
                &Threshold::new(Palette::monochrome()),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([0, 0, 0, 1]));
        assert_eq!(rendered.pixel(1, 0), Some([255, 255, 255, 2]));
        assert_eq!(rendered.pixel(2, 0), Some([255, 255, 255, 3]));
    }

    #[test]
    fn thresholds_custom_palette() {
        let source = source(2, 1, &[[240, 20, 10, 4], [20, 10, 240, 5]]);
        let effect = Threshold::new(Palette::new([[255, 0, 0], [0, 0, 255]]).unwrap());
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([255, 0, 0, 4]));
        assert_eq!(rendered.pixel(1, 0), Some([0, 0, 255, 5]));
    }

    #[test]
    fn thresholds_only_the_selected_polygon() {
        let source = source(3, 1, &[[100, 100, 100, 10]; 3]);
        let polygon = Polygon::new([
            Point::new(1.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 1.0),
            Point::new(1.0, 1.0),
        ])
        .unwrap();
        let rendered = Renderer::new()
            .render(
                &source,
                &Threshold::new(Palette::monochrome()),
                &Selection::Polygon(polygon),
            )
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([100, 100, 100, 10]));
        assert_eq!(rendered.pixel(1, 0), Some([0, 0, 0, 10]));
        assert_eq!(rendered.pixel(2, 0), Some([100, 100, 100, 10]));
    }

    #[test]
    fn validates_bayer_matrix_sizes() {
        for size in [2, 4, 8] {
            assert_eq!(
                OrderedDither::new(Palette::monochrome(), size)
                    .unwrap()
                    .matrix_size(),
                size
            );
        }

        for size in [0, 1, 3, 16] {
            assert_eq!(
                OrderedDither::new(Palette::monochrome(), size)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidParameter
            );
        }
    }

    #[test]
    fn uses_standard_bayer_matrices() {
        let expected = [
            (2, vec![0, 2, 3, 1]),
            (
                4,
                vec![0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5],
            ),
            (
                8,
                vec![
                    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14,
                    46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19,
                    59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29,
                    53, 21,
                ],
            ),
        ];

        for (size, expected) in expected {
            let actual = (0..size)
                .flat_map(|y| (0..size).map(move |x| bayer_value(x, y, size as u8)))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn renders_exact_pixels_for_each_bayer_size() {
        for size in [2, 4, 8] {
            let source = source(size.into(), 1, &vec![[128, 128, 128, 90]; size.into()]);
            let effect = OrderedDither::new(Palette::monochrome(), size).unwrap();
            let rendered = Renderer::new()
                .render(&source, &effect, &Selection::All)
                .unwrap();

            for x in 0..u32::from(size) {
                let value = if x % 2 == 0 { 0 } else { 255 };
                assert_eq!(rendered.pixel(x, 0), Some([value, value, value, 90]));
            }
        }
    }

    #[test]
    fn keeps_polygon_patterns_anchored_to_image_coordinates() {
        let source = source(4, 1, &[[128, 128, 128, 80]; 4]);
        let effect = OrderedDither::new(Palette::monochrome(), 4).unwrap();
        let all = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        let polygon = Polygon::new([
            Point::new(1.0, 0.0),
            Point::new(3.0, 0.0),
            Point::new(3.0, 1.0),
            Point::new(1.0, 1.0),
        ])
        .unwrap();
        let selected = Renderer::new()
            .render(&source, &effect, &Selection::Polygon(polygon))
            .unwrap();

        assert_eq!(selected.pixel(0, 0), source.pixel(0, 0));
        assert_eq!(selected.pixel(1, 0), all.pixel(1, 0));
        assert_eq!(selected.pixel(2, 0), all.pixel(2, 0));
        assert_eq!(selected.pixel(3, 0), source.pixel(3, 0));
    }

    #[test]
    fn ordered_dithering_is_repeatable() {
        let source = source(
            3,
            1,
            &[[40, 80, 120, 1], [100, 140, 180, 2], [160, 200, 240, 3]],
        );
        let effect = OrderedDither::new(
            Palette::new([[0, 20, 40], [100, 120, 140], [220, 240, 255]]).unwrap(),
            4,
        )
        .unwrap();
        let first = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        let second = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
    }
}
