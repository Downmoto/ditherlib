use crate::{Mask, Pipeline, Result, Selection, SourceImage};

/// An image effect that can be evaluated by a [`Renderer`].
pub trait Effect {
    /// Writes effected RGBA8 pixels into `output`.
    ///
    /// `input` and `output` contain four row-major bytes per pixel and have the
    /// supplied dimensions. `output` initially contains a copy of `input`.
    /// Implementations must leave pixels with zero mask coverage unchanged and
    /// write the fully effected value for pixels with non-zero coverage. The
    /// renderer blends partial mask coverage after this method returns.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::DitherError`] when the effect cannot be evaluated.
    fn apply(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) -> Result<()>;
}

/// A separately owned image produced by a [`Renderer`].
pub struct RenderedImage {
    width: u32,
    height: u32,
    pixels: Box<[u8]>,
}

impl RenderedImage {
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

impl std::fmt::Debug for RenderedImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RenderedImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// Evaluates effects without modifying their source image.
#[derive(Default)]
pub struct Renderer {
    current: Vec<u8>,
    scratch: Vec<u8>,
}

impl Renderer {
    /// Creates a renderer with empty reusable working buffers.
    pub const fn new() -> Self {
        Self {
            current: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Applies one effect to a selection and returns a separately owned image.
    ///
    /// # Errors
    ///
    /// Returns an error when the selection cannot be rasterised or the effect
    /// cannot be evaluated.
    pub fn render(
        &mut self,
        source: &SourceImage,
        effect: &impl Effect,
        selection: &Selection,
    ) -> Result<RenderedImage> {
        let dimensions = source.dimensions();
        self.begin(source);
        self.apply_step(effect, selection, dimensions)?;

        Ok(self.rendered_image(dimensions))
    }

    /// Applies every pipeline step in order and returns a separately owned image.
    ///
    /// An empty pipeline returns an unchanged copy of `source`.
    ///
    /// # Errors
    ///
    /// Returns an error when a selection cannot be rasterised or an effect
    /// cannot be evaluated.
    pub fn render_pipeline(
        &mut self,
        source: &SourceImage,
        pipeline: &Pipeline,
    ) -> Result<RenderedImage> {
        let dimensions = source.dimensions();
        self.begin(source);

        for step in pipeline.steps() {
            self.apply_step(step.effect(), step.selection(), dimensions)?;
        }

        Ok(self.rendered_image(dimensions))
    }

    /// Resets the reusable buffers to a new immutable source.
    fn begin(&mut self, source: &SourceImage) {
        self.current.clear();
        self.current.extend_from_slice(source.rgba8_bytes());
    }

    /// Applies one step and swaps the reusable input and output buffers.
    fn apply_step(
        &mut self,
        effect: &dyn Effect,
        selection: &Selection,
        dimensions: (u32, u32),
    ) -> Result<()> {
        let mask = selection.rasterise(dimensions.0, dimensions.1)?;
        self.scratch.clear();
        self.scratch.extend_from_slice(&self.current);

        effect.apply(&self.current, &mut self.scratch, dimensions, &mask)?;
        if matches!(selection, Selection::Polygon(_)) {
            composite_selection(&self.current, &mut self.scratch, &mask);
        }
        std::mem::swap(&mut self.current, &mut self.scratch);

        Ok(())
    }

    /// Copies the final reusable buffer into a caller-owned image.
    fn rendered_image(&self, dimensions: (u32, u32)) -> RenderedImage {
        RenderedImage {
            width: dimensions.0,
            height: dimensions.1,
            pixels: self.current.clone().into_boxed_slice(),
        }
    }
}

/// Blends effected pixels along an anti-aliased selection edge.
fn composite_selection(input: &[u8], output: &mut [u8], mask: &Mask) {
    let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
        return;
    };
    let width = mask.width() as usize;

    for y in min_y..max_y {
        for x in min_x..max_x {
            let pixel_index = y as usize * width + x as usize;
            let coverage = mask.coverage_bytes()[pixel_index];
            if coverage == 0 || coverage == u8::MAX {
                continue;
            }

            let byte_index = pixel_index * 4;
            for channel in 0..4 {
                output[byte_index + channel] = blend(
                    input[byte_index + channel],
                    output[byte_index + channel],
                    coverage,
                );
            }
        }
    }
}

/// Blends one effected channel over its input using mask coverage.
fn blend(input: u8, effected: u8, coverage: u8) -> u8 {
    let coverage = u32::from(coverage);
    let inverse = u32::from(u8::MAX) - coverage;

    ((u32::from(input) * inverse + u32::from(effected) * coverage + 127) / u32::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::{Effect, Renderer};
    use crate::{DitherError, ErrorKind, Pipeline, Point, Polygon, Result, Selection, SourceImage};
    use std::{cell::RefCell, rc::Rc};

    struct PaintRed;

    impl Effect for PaintRed {
        fn apply(
            &self,
            _input: &[u8],
            output: &mut [u8],
            _dimensions: (u32, u32),
            mask: &crate::Mask,
        ) -> Result<()> {
            for (pixel, &coverage) in output
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(mask.coverage_bytes())
            {
                if coverage != 0 {
                    *pixel = [255, 0, 0, 255];
                }
            }

            Ok(())
        }
    }

    struct Fail;

    impl Effect for Fail {
        fn apply(
            &self,
            _input: &[u8],
            _output: &mut [u8],
            _dimensions: (u32, u32),
            _mask: &crate::Mask,
        ) -> Result<()> {
            Err(DitherError::new(ErrorKind::Effect, "test effect failed"))
        }
    }

    struct AddRed(u8);

    impl Effect for AddRed {
        fn apply(
            &self,
            _input: &[u8],
            output: &mut [u8],
            _dimensions: (u32, u32),
            mask: &crate::Mask,
        ) -> Result<()> {
            for (pixel, &coverage) in output
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(mask.coverage_bytes())
            {
                if coverage != 0 {
                    pixel[0] = pixel[0].saturating_add(self.0);
                }
            }

            Ok(())
        }
    }

    struct MultiplyRed(u8);

    impl Effect for MultiplyRed {
        fn apply(
            &self,
            _input: &[u8],
            output: &mut [u8],
            _dimensions: (u32, u32),
            mask: &crate::Mask,
        ) -> Result<()> {
            for (pixel, &coverage) in output
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(mask.coverage_bytes())
            {
                if coverage != 0 {
                    pixel[0] = pixel[0].saturating_mul(self.0);
                }
            }

            Ok(())
        }
    }

    struct RecordBuffers(Rc<RefCell<Vec<(usize, usize)>>>);

    impl Effect for RecordBuffers {
        fn apply(
            &self,
            input: &[u8],
            output: &mut [u8],
            _dimensions: (u32, u32),
            _mask: &crate::Mask,
        ) -> Result<()> {
            self.0
                .borrow_mut()
                .push((input.as_ptr() as usize, output.as_ptr() as usize));
            Ok(())
        }
    }

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
    fn renders_an_effect_across_the_whole_image() {
        let source = source(2, 1, &[[1, 2, 3, 4], [5, 6, 7, 8]]);
        let original = source.rgba8_bytes().to_vec();
        let rendered = Renderer::new()
            .render(&source, &PaintRed, &Selection::All)
            .unwrap();

        assert_eq!(rendered.dimensions(), (2, 1));
        assert_eq!(rendered.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(rendered.pixel(1, 0), Some([255, 0, 0, 255]));
        assert_eq!(source.rgba8_bytes(), original);
        assert_ne!(
            source.rgba8_bytes().as_ptr(),
            rendered.rgba8_bytes().as_ptr()
        );
    }

    #[test]
    fn preserves_pixels_outside_a_polygon() {
        let original = [[10, 20, 30, 40]; 9];
        let source = source(3, 3, &original);
        let polygon = Polygon::new([
            Point::new(1.0, 1.0),
            Point::new(2.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(1.0, 2.0),
        ])
        .unwrap();
        let rendered = Renderer::new()
            .render(&source, &PaintRed, &Selection::Polygon(polygon))
            .unwrap();

        for y in 0..3 {
            for x in 0..3 {
                let expected = if (x, y) == (1, 1) {
                    [255, 0, 0, 255]
                } else {
                    [10, 20, 30, 40]
                };
                assert_eq!(rendered.pixel(x, y), Some(expected));
            }
        }
    }

    #[test]
    fn blends_anti_aliased_selection_edges() {
        let source = source(1, 1, &[[0, 0, 0, 0]]);
        let polygon = Polygon::new([
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
        ])
        .unwrap();
        let rendered = Renderer::new()
            .render(&source, &PaintRed, &Selection::Polygon(polygon))
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([159, 0, 0, 159]));
    }

    #[test]
    fn propagates_effect_errors() {
        let source = source(1, 1, &[[0, 0, 0, 255]]);
        let error = Renderer::new()
            .render(&source, &Fail, &Selection::All)
            .expect_err("the effect should fail");

        assert_eq!(error.kind(), ErrorKind::Effect);
    }

    #[test]
    fn pipeline_applies_steps_in_order_and_supports_reordering() {
        let source = source(1, 1, &[[10, 20, 30, 255]]);
        let mut pipeline = Pipeline::new();
        pipeline.add(AddRed(10), Selection::All);
        pipeline.add(MultiplyRed(2), Selection::All);

        let mut renderer = Renderer::new();
        let added_then_multiplied = renderer.render_pipeline(&source, &pipeline).unwrap();
        assert_eq!(added_then_multiplied.pixel(0, 0), Some([40, 20, 30, 255]));

        pipeline.move_step(1, 0).unwrap();
        let multiplied_then_added = renderer.render_pipeline(&source, &pipeline).unwrap();
        assert_eq!(multiplied_then_added.pixel(0, 0), Some([30, 20, 30, 255]));
    }

    #[test]
    fn empty_pipeline_returns_a_separate_source_identical_image() {
        let source = source(2, 1, &[[1, 2, 3, 4], [5, 6, 7, 8]]);
        let rendered = Renderer::new()
            .render_pipeline(&source, &Pipeline::new())
            .unwrap();

        assert_eq!(rendered.rgba8_bytes(), source.rgba8_bytes());
        assert_ne!(
            rendered.rgba8_bytes().as_ptr(),
            source.rgba8_bytes().as_ptr()
        );
    }

    #[test]
    fn removing_a_step_matches_a_pipeline_without_that_step() {
        let source = source(1, 1, &[[10, 20, 30, 255]]);
        let mut edited = Pipeline::new();
        edited.add(AddRed(10), Selection::All);
        edited.add(MultiplyRed(2), Selection::All);
        edited.remove(0).unwrap();

        let mut expected = Pipeline::new();
        expected.add(MultiplyRed(2), Selection::All);

        let mut renderer = Renderer::new();
        let edited = renderer.render_pipeline(&source, &edited).unwrap();
        let expected = renderer.render_pipeline(&source, &expected).unwrap();
        assert_eq!(edited.rgba8_bytes(), expected.rgba8_bytes());
    }

    #[test]
    fn alternates_buffers_for_odd_and_even_step_counts() {
        let source = source(1, 1, &[[10, 20, 30, 255]]);
        let records = Rc::new(RefCell::new(Vec::new()));
        let mut pipeline = Pipeline::new();
        for _ in 0..3 {
            pipeline.add(RecordBuffers(Rc::clone(&records)), Selection::All);
        }

        let mut renderer = Renderer::new();
        renderer.render_pipeline(&source, &pipeline).unwrap();
        let odd = records.borrow();
        assert_eq!(odd.len(), 3);
        assert_ne!(odd[0].0, odd[0].1);
        assert_eq!(odd[0].1, odd[1].0);
        assert_eq!(odd[1].1, odd[2].0);
        drop(odd);

        pipeline.remove(2).unwrap();
        records.borrow_mut().clear();
        renderer.render_pipeline(&source, &pipeline).unwrap();
        let even = records.borrow();
        assert_eq!(even.len(), 2);
        assert_ne!(even[0].0, even[0].1);
        assert_eq!(even[0].1, even[1].0);
    }
}
