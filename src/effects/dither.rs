use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const FIXED_SCALE: i32 = 256;
const FLOYD_STEINBERG: &[(i64, i64, i32)] = &[(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)];
const ATKINSON: &[(i64, i64, i32)] = &[
    (1, 0, 1),
    (2, 0, 1),
    (-1, 1, 1),
    (0, 1, 1),
    (1, 1, 1),
    (0, 2, 1),
];
const JARVIS_JUDICE_NINKE: &[(i64, i64, i32)] = &[
    (1, 0, 7),
    (2, 0, 5),
    (-2, 1, 3),
    (-1, 1, 5),
    (0, 1, 7),
    (1, 1, 5),
    (2, 1, 3),
    (-2, 2, 1),
    (-1, 2, 3),
    (0, 2, 5),
    (1, 2, 3),
    (2, 2, 1),
];
const STUCKI: &[(i64, i64, i32)] = &[
    (1, 0, 8),
    (2, 0, 4),
    (-2, 1, 2),
    (-1, 1, 4),
    (0, 1, 8),
    (1, 1, 4),
    (2, 1, 2),
    (-2, 2, 1),
    (-1, 2, 2),
    (0, 2, 4),
    (1, 2, 2),
    (2, 2, 1),
];
const BURKES: &[(i64, i64, i32)] = &[
    (1, 0, 8),
    (2, 0, 4),
    (-2, 1, 2),
    (-1, 1, 4),
    (0, 1, 8),
    (1, 1, 4),
    (2, 1, 2),
];
const SIERRA: &[(i64, i64, i32)] = &[
    (1, 0, 5),
    (2, 0, 3),
    (-2, 1, 2),
    (-1, 1, 4),
    (0, 1, 5),
    (1, 1, 4),
    (2, 1, 2),
    (-1, 2, 2),
    (0, 2, 3),
    (1, 2, 2),
];
const TWO_ROW_SIERRA: &[(i64, i64, i32)] = &[
    (1, 0, 4),
    (2, 0, 3),
    (-2, 1, 1),
    (-1, 1, 2),
    (0, 1, 3),
    (1, 1, 2),
    (2, 1, 1),
];
const SIERRA_LITE: &[(i64, i64, i32)] = &[(1, 0, 2), (-1, 1, 1), (0, 1, 1)];

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

    /// Creates a palette containing black, white, and one RGB colour.
    ///
    /// Supplying black or white produces the same two entries as
    /// [`Palette::black_and_white`].
    pub fn monochrome(colour: [u8; 3]) -> Self {
        if colour == [0, 0, 0] || colour == [255, 255, 255] {
            return Self::black_and_white();
        }

        Self {
            colours: Box::new([[0, 0, 0], colour, [255, 255, 255]]),
        }
    }

    /// Creates a palette containing black and white.
    pub fn black_and_white() -> Self {
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
    #[inline]
    pub fn nearest_colour(&self, colour: [u8; 3]) -> [u8; 3] {
        if self.colours.as_ref() == [[0, 0, 0], [255, 255, 255]] {
            // The squared distances cross at an RGB sum of 382.5.
            let sum = u16::from(colour[0]) + u16::from(colour[1]) + u16::from(colour[2]);
            return [if sum >= 383 { 255 } else { 0 }; 3];
        }
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
    pixel_size: u32,
}

impl Threshold {
    /// Creates threshold dithering with the supplied palette.
    pub const fn new(palette: Palette) -> Self {
        Self {
            palette,
            pixel_size: 1,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Sets the width and height of each square logical pixel.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `pixel_size` is zero.
    pub fn with_pixel_size(mut self, pixel_size: u32) -> Result<Self> {
        self.pixel_size = validate_pixel_size(pixel_size)?;
        Ok(self)
    }

    /// Returns the width and height of each square logical pixel.
    pub const fn pixel_size(&self) -> u32 {
        self.pixel_size
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
        quantise_cells(
            input,
            output,
            dimensions,
            mask,
            self.pixel_size,
            |colour, _, _| self.palette.nearest_colour(colour),
        );
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
    pixel_size: u32,
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
            pixel_size: 1,
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

    /// Sets the width and height of each square logical pixel.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `pixel_size` is zero.
    pub fn with_pixel_size(mut self, pixel_size: u32) -> Result<Self> {
        self.pixel_size = validate_pixel_size(pixel_size)?;
        Ok(self)
    }

    /// Returns the width and height of each square logical pixel.
    pub const fn pixel_size(&self) -> u32 {
        self.pixel_size
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
        let size = u32::from(self.matrix_size);
        let levels = (size * size) as i32;
        let mut adjustments = [0; 64];
        for y in 0..size {
            for x in 0..size {
                let threshold = i32::from(bayer_value(x, y, self.matrix_size));
                adjustments[(y * size + x) as usize] =
                    (2 * threshold + 1 - levels) * 255 / (2 * levels);
            }
        }
        quantise_cells(
            input,
            output,
            dimensions,
            mask,
            self.pixel_size,
            |colour, x, y| {
                let adjustment = adjustments[((y % size) * size + x % size) as usize];
                let colour =
                    colour.map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8);
                self.palette.nearest_colour(colour)
            },
        );
        Ok(())
    }
}

/// An error-diffusion weight preset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiffusionAlgorithm {
    /// Floyd-Steinberg's compact two-row kernel.
    FloydSteinberg,
    /// Atkinson's light, high-contrast diffusion.
    Atkinson,
    /// Jarvis, Judice, and Ninke's broad three-row kernel.
    JarvisJudiceNinke,
    /// Stucki's broad three-row kernel.
    Stucki,
    /// Burkes' two-row simplification of Stucki diffusion.
    Burkes,
    /// Sierra's three-row kernel.
    Sierra,
    /// Sierra's smaller two-row kernel.
    TwoRowSierra,
    /// Sierra Lite's fast three-neighbour kernel.
    SierraLite,
}

/// The horizontal traversal used by error diffusion.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiffusionScan {
    /// Processes every row from left to right.
    #[default]
    Raster,
    /// Alternates direction on each logical row to reduce directional artefacts.
    Serpentine,
}

/// Applies a configurable error-diffusion algorithm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorDiffusion {
    palette: Palette,
    algorithm: DiffusionAlgorithm,
    scan: DiffusionScan,
    pixel_size: u32,
}

impl ErrorDiffusion {
    /// Creates error diffusion using the supplied palette and weight preset.
    pub const fn new(palette: Palette, algorithm: DiffusionAlgorithm) -> Self {
        Self {
            palette,
            algorithm,
            scan: DiffusionScan::Raster,
            pixel_size: 1,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the diffusion weight preset.
    pub const fn algorithm(&self) -> DiffusionAlgorithm {
        self.algorithm
    }

    /// Sets the horizontal traversal mode.
    pub const fn with_scan(mut self, scan: DiffusionScan) -> Self {
        self.scan = scan;
        self
    }

    /// Returns the horizontal traversal mode.
    pub const fn scan(&self) -> DiffusionScan {
        self.scan
    }

    /// Sets the width and height of each square logical pixel.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `pixel_size` is zero.
    pub fn with_pixel_size(mut self, pixel_size: u32) -> Result<Self> {
        self.pixel_size = validate_pixel_size(pixel_size)?;
        Ok(self)
    }

    /// Returns the width and height of each square logical pixel.
    pub const fn pixel_size(&self) -> u32 {
        self.pixel_size
    }

    fn apply_scan<const SERPENTINE: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) {
        match self.algorithm {
            DiffusionAlgorithm::FloydSteinberg => diffuse_error::<16, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                FLOYD_STEINBERG,
                self.pixel_size,
            ),
            DiffusionAlgorithm::Atkinson => diffuse_error::<8, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                ATKINSON,
                self.pixel_size,
            ),
            DiffusionAlgorithm::JarvisJudiceNinke => diffuse_error::<48, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                JARVIS_JUDICE_NINKE,
                self.pixel_size,
            ),
            DiffusionAlgorithm::Stucki => diffuse_error::<42, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                STUCKI,
                self.pixel_size,
            ),
            DiffusionAlgorithm::Burkes => diffuse_error::<32, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                BURKES,
                self.pixel_size,
            ),
            DiffusionAlgorithm::Sierra => diffuse_error::<32, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                SIERRA,
                self.pixel_size,
            ),
            DiffusionAlgorithm::TwoRowSierra => diffuse_error::<16, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                TWO_ROW_SIERRA,
                self.pixel_size,
            ),
            DiffusionAlgorithm::SierraLite => diffuse_error::<4, SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                &self.palette,
                SIERRA_LITE,
                self.pixel_size,
            ),
        }
    }
}

impl Effect for ErrorDiffusion {
    fn apply(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) -> Result<()> {
        match self.scan {
            DiffusionScan::Raster => self.apply_scan::<false>(input, output, dimensions, mask),
            DiffusionScan::Serpentine => self.apply_scan::<true>(input, output, dimensions, mask),
        }
        Ok(())
    }
}

/// Quantises selected logical cells without error diffusion.
fn quantise_cells(
    input: &[u8],
    output: &mut [u8],
    dimensions: (u32, u32),
    mask: &Mask,
    pixel_size: u32,
    mut quantise: impl FnMut([u8; 3], u32, u32) -> [u8; 3],
) {
    let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
        return;
    };
    let image_width = dimensions.0 as usize;

    if pixel_size == 1 {
        for y in min_y..max_y {
            for x in min_x..max_x {
                let pixel_index = y as usize * image_width + x as usize;
                if mask.coverage_bytes()[pixel_index] == 0 {
                    continue;
                }
                let byte_index = pixel_index * 4;
                let colour = quantise(
                    [
                        input[byte_index],
                        input[byte_index + 1],
                        input[byte_index + 2],
                    ],
                    x,
                    y,
                );
                output[byte_index..byte_index + 4].copy_from_slice(&[
                    colour[0],
                    colour[1],
                    colour[2],
                    input[byte_index + 3],
                ]);
            }
        }
        return;
    }

    let start_x = align_to_grid(min_x, pixel_size);
    let start_y = align_to_grid(min_y, pixel_size);
    let mut y = start_y;

    while y < max_y {
        let mut x = start_x;
        while x < max_x {
            let bounds = cell_bounds(x, y, pixel_size, dimensions);
            if let Some(colour) = sample_cell(input, mask, image_width, bounds) {
                let colour = quantise(colour, x / pixel_size, y / pixel_size);
                write_cell(input, output, mask, image_width, bounds, colour);
            }
            x = x.saturating_add(pixel_size);
        }
        y = y.saturating_add(pixel_size);
    }
}

/// Quantises selected logical cells and distributes errors to later cells.
fn diffuse_error<const DIVISOR: i32, const SERPENTINE: bool>(
    input: &[u8],
    output: &mut [u8],
    dimensions: (u32, u32),
    mask: &Mask,
    palette: &Palette,
    neighbours: &[(i64, i64, i32)],
    pixel_size: u32,
) {
    let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
        return;
    };
    let image_width = dimensions.0 as usize;
    let start_x = align_to_grid(min_x, pixel_size);
    let start_y = align_to_grid(min_y, pixel_size);
    let working_width = (max_x - start_x).div_ceil(pixel_size) as usize;
    let working_height = (max_y - start_y).div_ceil(pixel_size) as usize;
    let mut selected = Vec::with_capacity(working_width * working_height);
    let mut working = Vec::with_capacity(working_width * working_height);

    for cell_y in 0..working_height {
        for cell_x in 0..working_width {
            let x = start_x + cell_x as u32 * pixel_size;
            let y = start_y + cell_y as u32 * pixel_size;
            let bounds = cell_bounds(x, y, pixel_size, dimensions);
            let colour = sample_cell(input, mask, image_width, bounds);
            selected.push(colour.is_some());
            working.push(
                colour
                    .unwrap_or([0; 3])
                    .map(|channel| i32::from(channel) * FIXED_SCALE),
            );
        }
    }

    for cell_y in 0..working_height {
        let reverse = SERPENTINE && (start_y / pixel_size + cell_y as u32) % 2 == 1;
        for column in 0..working_width {
            let cell_x = if reverse {
                working_width - column - 1
            } else {
                column
            };
            let working_index = cell_y * working_width + cell_x;
            if !selected[working_index] {
                continue;
            }

            let adjusted =
                working[working_index].map(|channel| channel.clamp(0, 255 * FIXED_SCALE));
            let colour = palette.nearest_colour(
                adjusted.map(|channel| ((channel + FIXED_SCALE / 2) / FIXED_SCALE) as u8),
            );
            let x = start_x + cell_x as u32 * pixel_size;
            let y = start_y + cell_y as u32 * pixel_size;
            write_cell(
                input,
                output,
                mask,
                image_width,
                cell_bounds(x, y, pixel_size, dimensions),
                colour,
            );
            let error = [
                adjusted[0] - i32::from(colour[0]) * FIXED_SCALE,
                adjusted[1] - i32::from(colour[1]) * FIXED_SCALE,
                adjusted[2] - i32::from(colour[2]) * FIXED_SCALE,
            ];

            for &(offset_x, offset_y, weight) in neighbours {
                let offset_x = if reverse { -offset_x } else { offset_x };
                let neighbour_x = cell_x as i64 + offset_x;
                let neighbour_y = cell_y as i64 + offset_y;
                if neighbour_x < 0
                    || neighbour_x >= working_width as i64
                    || neighbour_y < 0
                    || neighbour_y >= working_height as i64
                {
                    continue;
                }

                let neighbour_x = neighbour_x as usize;
                let neighbour_y = neighbour_y as usize;
                let neighbour_index = neighbour_y * working_width + neighbour_x;
                if !selected[neighbour_index] {
                    continue;
                }
                if !diffusion_path_is_selected(
                    &selected,
                    working_width,
                    cell_x,
                    cell_y,
                    offset_x,
                    offset_y,
                ) {
                    continue;
                }

                for channel in 0..3 {
                    working[neighbour_index][channel] += error[channel] * weight / DIVISOR;
                }
            }
        }
    }
}

/// Returns the image-origin-aligned coordinate containing `coordinate`.
fn align_to_grid(coordinate: u32, pixel_size: u32) -> u32 {
    coordinate - coordinate % pixel_size
}

/// Returns exclusive image bounds for a logical pixel.
fn cell_bounds(x: u32, y: u32, pixel_size: u32, dimensions: (u32, u32)) -> (u32, u32, u32, u32) {
    (
        x,
        y,
        x.saturating_add(pixel_size).min(dimensions.0),
        y.saturating_add(pixel_size).min(dimensions.1),
    )
}

/// Averages selected RGB values within a logical pixel using mask coverage.
fn sample_cell(
    input: &[u8],
    mask: &Mask,
    image_width: usize,
    bounds: (u32, u32, u32, u32),
) -> Option<[u8; 3]> {
    if bounds.2 - bounds.0 == 1 && bounds.3 - bounds.1 == 1 {
        let pixel_index = bounds.1 as usize * image_width + bounds.0 as usize;
        if mask.coverage_bytes()[pixel_index] == 0 {
            return None;
        }
        let byte_index = pixel_index * 4;
        return Some([
            input[byte_index],
            input[byte_index + 1],
            input[byte_index + 2],
        ]);
    }

    let mut totals = [0_u64; 3];
    let mut total_coverage = 0_u64;

    for y in bounds.1..bounds.3 {
        for x in bounds.0..bounds.2 {
            let pixel_index = y as usize * image_width + x as usize;
            let coverage = u64::from(mask.coverage_bytes()[pixel_index]);
            if coverage == 0 {
                continue;
            }

            let byte_index = pixel_index * 4;
            for channel in 0..3 {
                totals[channel] += u64::from(input[byte_index + channel]) * coverage;
            }
            total_coverage += coverage;
        }
    }

    (total_coverage != 0)
        .then(|| totals.map(|total| ((total + total_coverage / 2) / total_coverage) as u8))
}

/// Fills the selected portion of a logical pixel while preserving alpha.
fn write_cell(
    input: &[u8],
    output: &mut [u8],
    mask: &Mask,
    image_width: usize,
    bounds: (u32, u32, u32, u32),
    colour: [u8; 3],
) {
    for y in bounds.1..bounds.3 {
        for x in bounds.0..bounds.2 {
            let pixel_index = y as usize * image_width + x as usize;
            if mask.coverage_bytes()[pixel_index] == 0 {
                continue;
            }

            let byte_index = pixel_index * 4;
            output[byte_index..byte_index + 4].copy_from_slice(&[
                colour[0],
                colour[1],
                colour[2],
                input[byte_index + 3],
            ]);
        }
    }
}

/// Prevents two-cell diffusion taps from jumping across an unselected cell.
fn diffusion_path_is_selected(
    selected: &[bool],
    width: usize,
    x: usize,
    y: usize,
    offset_x: i64,
    offset_y: i64,
) -> bool {
    let middle = match (offset_x, offset_y) {
        (-2 | 2, 0) => Some((x as i64 + offset_x / 2, y as i64)),
        (0, -2 | 2) => Some((x as i64, y as i64 + offset_y / 2)),
        _ => None,
    };

    middle.is_none_or(|(x, y)| selected[y as usize * width + x as usize])
}

/// Validates a logical pixel size shared by all dithering effects.
fn validate_pixel_size(pixel_size: u32) -> Result<u32> {
    if pixel_size == 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "dither pixel size must be greater than zero",
        ));
    }

    Ok(pixel_size)
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
    use super::{
        DiffusionAlgorithm, DiffusionScan, ErrorDiffusion, OrderedDither, Palette, Threshold,
        bayer_value,
    };
    use crate::{Effect, ErrorKind, Mask, Point, Polygon, Renderer, Selection, SourceImage};

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

    fn diffusion(algorithm: DiffusionAlgorithm) -> ErrorDiffusion {
        ErrorDiffusion::new(Palette::black_and_white(), algorithm)
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

        let black_and_white = Palette::black_and_white();
        assert_eq!(black_and_white.nearest_colour([255, 127, 0]), [0; 3]);
        assert_eq!(black_and_white.nearest_colour([0, 128, 255]), [255; 3]);

        assert_eq!(
            Palette::monochrome([1, 2, 3]).colours(),
            &[[0, 0, 0], [1, 2, 3], [255, 255, 255]]
        );
        assert_eq!(
            Palette::monochrome([0, 0, 0]).colours(),
            &[[0, 0, 0], [255, 255, 255]]
        );
        assert_eq!(
            Palette::monochrome([255, 255, 255]).colours(),
            &[[0, 0, 0], [255, 255, 255]]
        );
        assert_eq!(
            Palette::black_and_white().colours(),
            &[[0, 0, 0], [255, 255, 255]]
        );
    }

    #[test]
    fn thresholds_exact_black_and_white_pixels_and_preserves_alpha() {
        let source = source(
            3,
            1,
            &[[10, 20, 30, 1], [128, 128, 128, 2], [245, 235, 225, 3]],
        );
        let rendered = Renderer::new()
            .render(
                &source,
                &Threshold::new(Palette::black_and_white()),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([0, 0, 0, 1]));
        assert_eq!(rendered.pixel(1, 0), Some([255, 255, 255, 2]));
        assert_eq!(rendered.pixel(2, 0), Some([255, 255, 255, 3]));
    }

    #[test]
    fn thresholds_monochrome_with_black_colour_and_white() {
        let source = source(
            3,
            1,
            &[[10, 10, 10, 1], [240, 10, 10, 2], [250, 250, 250, 3]],
        );
        let rendered = Renderer::new()
            .render(
                &source,
                &Threshold::new(Palette::monochrome([255, 0, 0])),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([0, 0, 0, 1]));
        assert_eq!(rendered.pixel(1, 0), Some([255, 0, 0, 2]));
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
                &Threshold::new(Palette::black_and_white()),
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
                OrderedDither::new(Palette::black_and_white(), size)
                    .unwrap()
                    .matrix_size(),
                size
            );
        }

        for size in [0, 1, 3, 16] {
            assert_eq!(
                OrderedDither::new(Palette::black_and_white(), size)
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
            let effect = OrderedDither::new(Palette::black_and_white(), size).unwrap();
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
        let effect = OrderedDither::new(Palette::black_and_white(), 4).unwrap();
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

    #[test]
    fn floyd_steinberg_produces_exact_pixels() {
        let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
        let rendered = Renderer::new()
            .render(
                &source,
                &diffusion(DiffusionAlgorithm::FloydSteinberg),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(
            rendered.rgba8_bytes(),
            &[0, 0, 0, 70, 255, 255, 255, 70, 0, 0, 0, 70, 0, 0, 0, 70,]
        );
    }

    #[test]
    fn floyd_steinberg_does_not_import_error_across_a_polygon_boundary() {
        let source = source(2, 1, &[[100, 100, 100, 20]; 2]);
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
                &diffusion(DiffusionAlgorithm::FloydSteinberg),
                &Selection::Polygon(polygon),
            )
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), Some([100, 100, 100, 20]));
        assert_eq!(rendered.pixel(1, 0), Some([0, 0, 0, 20]));
    }

    #[test]
    fn floyd_steinberg_handles_narrow_edge_selections() {
        let source = source(
            1,
            3,
            &[[80, 80, 80, 1], [120, 120, 120, 2], [160, 160, 160, 3]],
        );
        let rendered = Renderer::new()
            .render(
                &source,
                &diffusion(DiffusionAlgorithm::FloydSteinberg),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.dimensions(), (1, 3));
        assert_eq!(
            rendered
                .rgba8_bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[3])
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn atkinson_produces_exact_pixels() {
        let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
        let rendered = Renderer::new()
            .render(
                &source,
                &diffusion(DiffusionAlgorithm::Atkinson),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(
            rendered.rgba8_bytes(),
            &[0, 0, 0, 70, 0, 0, 0, 70, 0, 0, 0, 70, 255, 255, 255, 70,]
        );
    }

    #[test]
    fn atkinson_does_not_jump_unselected_gaps() {
        let input = [100, 100, 100, 1, 0, 0, 0, 2, 120, 120, 120, 3];
        let mut output = input;
        let mask = Mask::new(3, 1, vec![255, 0, 255]).unwrap();

        diffusion(DiffusionAlgorithm::Atkinson)
            .apply(&input, &mut output, (3, 1), &mask)
            .unwrap();

        assert_eq!(output, [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3]);
    }

    #[test]
    fn atkinson_handles_narrow_image_edges() {
        let source = source(1, 2, &[[100, 100, 100, 4], [140, 140, 140, 5]]);
        let rendered = Renderer::new()
            .render(
                &source,
                &diffusion(DiffusionAlgorithm::Atkinson),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.dimensions(), (1, 2));
        assert_eq!(rendered.pixel(0, 0).unwrap()[3], 4);
        assert_eq!(rendered.pixel(0, 1).unwrap()[3], 5);
    }

    #[test]
    fn error_diffusion_is_repeatable() {
        let source = source(
            3,
            2,
            &[
                [30, 60, 90, 1],
                [70, 100, 130, 2],
                [110, 140, 170, 3],
                [150, 180, 210, 4],
                [190, 220, 250, 5],
                [230, 200, 170, 6],
            ],
        );

        let floyd = diffusion(DiffusionAlgorithm::FloydSteinberg);
        let first = Renderer::new()
            .render(&source, &floyd, &Selection::All)
            .unwrap();
        let second = Renderer::new()
            .render(&source, &floyd, &Selection::All)
            .unwrap();
        assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());

        let atkinson = diffusion(DiffusionAlgorithm::Atkinson);
        let first = Renderer::new()
            .render(&source, &atkinson, &Selection::All)
            .unwrap();
        let second = Renderer::new()
            .render(&source, &atkinson, &Selection::All)
            .unwrap();
        assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
    }

    #[test]
    fn validates_dither_pixel_sizes() {
        assert_eq!(Threshold::new(Palette::black_and_white()).pixel_size(), 1);
        assert_eq!(
            Threshold::new(Palette::black_and_white())
                .with_pixel_size(3)
                .unwrap()
                .pixel_size(),
            3
        );

        let errors = [
            Threshold::new(Palette::black_and_white())
                .with_pixel_size(0)
                .unwrap_err(),
            OrderedDither::new(Palette::black_and_white(), 2)
                .unwrap()
                .with_pixel_size(0)
                .unwrap_err(),
            diffusion(DiffusionAlgorithm::FloydSteinberg)
                .with_pixel_size(0)
                .unwrap_err(),
            diffusion(DiffusionAlgorithm::Atkinson)
                .with_pixel_size(0)
                .unwrap_err(),
        ];
        assert!(
            errors
                .iter()
                .all(|error| error.kind() == ErrorKind::InvalidParameter)
        );
    }

    #[test]
    fn threshold_fills_logical_pixels_with_their_average_colour() {
        let source = source(
            4,
            1,
            &[
                [0, 0, 0, 1],
                [200, 200, 200, 2],
                [100, 100, 100, 3],
                [255, 255, 255, 4],
            ],
        );
        let effect = Threshold::new(Palette::black_and_white())
            .with_pixel_size(2)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(
            rendered.rgba8_bytes(),
            &[0, 0, 0, 1, 0, 0, 0, 2, 255, 255, 255, 3, 255, 255, 255, 4]
        );
    }

    #[test]
    fn ordered_dithering_scales_bayer_cells() {
        let source = source(4, 1, &[[128, 128, 128, 9]; 4]);
        let effect = OrderedDither::new(Palette::black_and_white(), 2)
            .unwrap()
            .with_pixel_size(2)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(
            rendered.rgba8_bytes(),
            &[0, 0, 0, 9, 0, 0, 0, 9, 255, 255, 255, 9, 255, 255, 255, 9]
        );
    }

    #[test]
    fn scaled_dithering_keeps_cells_anchored_and_clipped_to_a_polygon() {
        let source = source(4, 1, &[[128, 128, 128, 7]; 4]);
        let polygon = Polygon::new([
            Point::new(3.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(4.0, 1.0),
            Point::new(3.0, 1.0),
        ])
        .unwrap();
        let effect = OrderedDither::new(Palette::black_and_white(), 2)
            .unwrap()
            .with_pixel_size(2)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::Polygon(polygon))
            .unwrap();

        assert_eq!(rendered.pixel(0, 0), source.pixel(0, 0));
        assert_eq!(rendered.pixel(1, 0), source.pixel(1, 0));
        assert_eq!(rendered.pixel(2, 0), source.pixel(2, 0));
        assert_eq!(rendered.pixel(3, 0), Some([255, 255, 255, 7]));
    }

    #[test]
    fn error_diffusion_operates_between_logical_pixels() {
        let pixels = (1..=8)
            .map(|alpha| [100, 100, 100, alpha])
            .collect::<Vec<_>>();
        let source = source(8, 1, &pixels);
        let floyd = diffusion(DiffusionAlgorithm::FloydSteinberg)
            .with_pixel_size(2)
            .unwrap();
        let atkinson = diffusion(DiffusionAlgorithm::Atkinson)
            .with_pixel_size(2)
            .unwrap();

        let floyd = Renderer::new()
            .render(&source, &floyd, &Selection::All)
            .unwrap();
        let atkinson = Renderer::new()
            .render(&source, &atkinson, &Selection::All)
            .unwrap();

        let floyd_colours = [0, 0, 255, 255, 0, 0, 0, 0];
        let atkinson_colours = [0, 0, 0, 0, 0, 0, 255, 255];
        for (index, pixel) in floyd.rgba8_bytes().as_chunks::<4>().0.iter().enumerate() {
            let value = floyd_colours[index];
            assert_eq!(*pixel, [value, value, value, (index + 1) as u8]);
        }
        for (index, pixel) in atkinson.rgba8_bytes().as_chunks::<4>().0.iter().enumerate() {
            let value = atkinson_colours[index];
            assert_eq!(*pixel, [value, value, value, (index + 1) as u8]);
        }
    }

    #[test]
    fn supports_every_diffusion_algorithm_and_configuration() {
        let pixels = (0..35)
            .map(|index| {
                [
                    (index * 47) as u8,
                    (index * 83) as u8,
                    (index * 131) as u8,
                    (index * 19) as u8,
                ]
            })
            .collect::<Vec<_>>();
        let source = source(7, 5, &pixels);
        let algorithms = [
            DiffusionAlgorithm::FloydSteinberg,
            DiffusionAlgorithm::Atkinson,
            DiffusionAlgorithm::JarvisJudiceNinke,
            DiffusionAlgorithm::Stucki,
            DiffusionAlgorithm::Burkes,
            DiffusionAlgorithm::Sierra,
            DiffusionAlgorithm::TwoRowSierra,
            DiffusionAlgorithm::SierraLite,
        ];

        for algorithm in algorithms {
            let effect = ErrorDiffusion::new(Palette::black_and_white(), algorithm)
                .with_scan(DiffusionScan::Serpentine)
                .with_pixel_size(2)
                .unwrap();
            let rendered = Renderer::new()
                .render(&source, &effect, &Selection::All)
                .unwrap();

            assert_eq!(effect.algorithm(), algorithm);
            assert_eq!(effect.scan(), DiffusionScan::Serpentine);
            assert_eq!(effect.pixel_size(), 2);
            assert_eq!(effect.palette(), &Palette::black_and_white());
            for (before, after) in source
                .rgba8_bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .zip(rendered.rgba8_bytes().as_chunks::<4>().0)
            {
                assert!(after[..3] == [0; 3] || after[..3] == [255; 3]);
                assert_eq!(after[3], before[3]);
            }
        }

        assert_eq!(
            ErrorDiffusion::new(Palette::black_and_white(), DiffusionAlgorithm::Stucki)
                .with_pixel_size(0)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidParameter
        );
    }

    #[test]
    fn serpentine_scan_is_anchored_to_image_rows() {
        let selected_row = [
            [20, 20, 20, 1],
            [70, 70, 70, 2],
            [110, 110, 110, 3],
            [140, 140, 140, 4],
            [170, 170, 170, 5],
            [220, 220, 220, 6],
        ];
        let mut pixels = vec![[0, 0, 0, 255]; 6];
        pixels.extend(selected_row);
        let image = source(6, 2, &pixels);
        let mask = Mask::new(6, 2, [vec![0; 6], vec![255; 6]].concat()).unwrap();
        let effect = ErrorDiffusion::new(
            Palette::black_and_white(),
            DiffusionAlgorithm::FloydSteinberg,
        )
        .with_scan(DiffusionScan::Serpentine);
        let mut actual = image.rgba8_bytes().to_vec();
        effect
            .apply(image.rgba8_bytes(), &mut actual, (6, 2), &mask)
            .unwrap();

        let reversed = selected_row.into_iter().rev().collect::<Vec<_>>();
        let reversed_source = source(6, 1, &reversed);
        let reference = Renderer::new()
            .render(
                &reversed_source,
                &diffusion(DiffusionAlgorithm::FloydSteinberg),
                &Selection::All,
            )
            .unwrap();
        let expected = reference
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .rev()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(&actual[6 * 4..], expected);
    }
}
