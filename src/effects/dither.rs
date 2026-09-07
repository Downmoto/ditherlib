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
        _dimensions: (u32, u32),
        mask: &Mask,
    ) -> Result<()> {
        for ((input, output), &coverage) in input
            .as_chunks::<4>()
            .0
            .iter()
            .zip(output.as_chunks_mut::<4>().0)
            .zip(mask.coverage_bytes())
        {
            if coverage == 0 {
                continue;
            }

            let colour = self.palette.nearest_colour([input[0], input[1], input[2]]);
            *output = [colour[0], colour[1], colour[2], input[3]];
        }

        Ok(())
    }
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
    use super::{Palette, Threshold};
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
}
