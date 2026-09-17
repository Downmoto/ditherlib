use super::cells::{PixelGrid, SamplingMode, cell_bounds, sample_cell, write_cell};
use super::palette::Palette;
use crate::{Effect, Mask, Result};

const FIXED_SCALE: i32 = 256;

// Appendix I of Victor Ostromoukhov, "A Simple and Efficient Error-Diffusion
// Algorithm", SIGGRAPH 2001. Entries are A10, A-11, and A01 for levels
// 0..=127; levels 128..=255 mirror these values.
const COEFFICIENTS: [[u16; 3]; 128] = [
    [13, 0, 5],
    [13, 0, 5],
    [21, 0, 10],
    [7, 0, 4],
    [8, 0, 5],
    [47, 3, 28],
    [23, 3, 13],
    [15, 3, 8],
    [22, 6, 11],
    [43, 15, 20],
    [7, 3, 3],
    [501, 224, 211],
    [249, 116, 103],
    [165, 80, 67],
    [123, 62, 49],
    [489, 256, 191],
    [81, 44, 31],
    [483, 272, 181],
    [60, 35, 22],
    [53, 32, 19],
    [237, 148, 83],
    [471, 304, 161],
    [3, 2, 1],
    [481, 314, 185],
    [354, 226, 155],
    [1389, 866, 685],
    [227, 138, 125],
    [267, 158, 163],
    [327, 188, 220],
    [61, 34, 45],
    [627, 338, 505],
    [1227, 638, 1075],
    [20, 10, 19],
    [1937, 1000, 1767],
    [977, 520, 855],
    [657, 360, 551],
    [71, 40, 57],
    [2005, 1160, 1539],
    [337, 200, 247],
    [2039, 1240, 1425],
    [257, 160, 171],
    [691, 440, 437],
    [1045, 680, 627],
    [301, 200, 171],
    [177, 120, 95],
    [2141, 1480, 1083],
    [1079, 760, 513],
    [725, 520, 323],
    [137, 100, 57],
    [2209, 1640, 855],
    [53, 40, 19],
    [2243, 1720, 741],
    [565, 440, 171],
    [759, 600, 209],
    [1147, 920, 285],
    [2311, 1880, 513],
    [97, 80, 19],
    [335, 280, 57],
    [1181, 1000, 171],
    [793, 680, 95],
    [599, 520, 57],
    [2413, 2120, 171],
    [405, 360, 19],
    [2447, 2200, 57],
    [11, 10, 0],
    [158, 151, 3],
    [178, 179, 7],
    [1030, 1091, 63],
    [248, 277, 21],
    [318, 375, 35],
    [458, 571, 63],
    [878, 1159, 147],
    [5, 7, 1],
    [172, 181, 37],
    [97, 76, 22],
    [72, 41, 17],
    [119, 47, 29],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [4, 1, 1],
    [65, 18, 17],
    [95, 29, 26],
    [185, 62, 53],
    [30, 11, 9],
    [35, 14, 11],
    [85, 37, 28],
    [55, 26, 19],
    [80, 41, 29],
    [155, 86, 59],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [5, 3, 2],
    [305, 176, 119],
    [155, 86, 59],
    [105, 56, 39],
    [80, 41, 29],
    [65, 32, 23],
    [55, 26, 19],
    [335, 152, 113],
    [85, 37, 28],
    [115, 48, 37],
    [35, 14, 11],
    [355, 136, 109],
    [30, 11, 9],
    [365, 128, 107],
    [185, 62, 53],
    [25, 8, 7],
    [95, 29, 26],
    [385, 112, 103],
    [65, 18, 17],
    [395, 104, 101],
    [4, 1, 1],
];

/// Applies Ostromoukhov's tone-adaptive variable-coefficient error diffusion.
///
/// Pixels follow a serpentine scan. Each RGB channel selects its three
/// diffusion weights from the error-adjusted channel tone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OstromoukhovDither {
    palette: Palette,
    grid: PixelGrid,
}

impl OstromoukhovDither {
    /// Creates Ostromoukhov dithering for the target palette.
    pub const fn new(palette: Palette) -> Self {
        Self {
            palette,
            grid: PixelGrid::new(),
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Sets the logical pixel width.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ErrorKind::InvalidParameter`] when `width` is zero.
    pub fn with_pixel_width(mut self, width: u32) -> Result<Self> {
        self.grid = self.grid.with_width(width)?;
        Ok(self)
    }

    /// Returns the logical pixel width.
    pub const fn pixel_width(&self) -> u32 {
        self.grid.width()
    }

    /// Sets the logical pixel height.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ErrorKind::InvalidParameter`] when `height` is zero.
    pub fn with_pixel_height(mut self, height: u32) -> Result<Self> {
        self.grid = self.grid.with_height(height)?;
        Ok(self)
    }

    /// Returns the logical pixel height.
    pub const fn pixel_height(&self) -> u32 {
        self.grid.height()
    }

    /// Sets the logical pixel width and height.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ErrorKind::InvalidParameter`] when either dimension is zero.
    pub fn with_pixel_size(mut self, width: u32, height: u32) -> Result<Self> {
        self.grid = self.grid.with_size(width, height)?;
        Ok(self)
    }

    /// Returns the logical pixel dimensions as `(width, height)`.
    pub const fn pixel_size(&self) -> (u32, u32) {
        self.grid.size()
    }

    /// Offsets the logical pixel grid in image pixels.
    pub const fn with_grid_offset(mut self, x: i32, y: i32) -> Self {
        self.grid = self.grid.with_offset(x, y);
        self
    }

    /// Returns the logical pixel grid offset as `(x, y)` image pixels.
    pub const fn grid_offset(&self) -> (i32, i32) {
        self.grid.offset()
    }

    /// Selects the source-colour sampling used for each logical pixel.
    pub const fn with_sampling(mut self, sampling: SamplingMode) -> Self {
        self.grid = self.grid.with_sampling(sampling);
        self
    }

    /// Returns the source-colour sampling mode.
    pub const fn sampling(&self) -> SamplingMode {
        self.grid.sampling()
    }
}

impl Effect for OstromoukhovDither {
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
        let image_width = dimensions.0 as usize;
        let start_cell_x = self.grid.cell_x(min_x);
        let start_cell_y = self.grid.cell_y(min_y);
        let width = (self.grid.cell_x(max_x - 1) - start_cell_x + 1) as usize;
        let height = (self.grid.cell_y(max_y - 1) - start_cell_y + 1) as usize;
        let mut current = Vec::with_capacity(width);
        let mut next = Vec::with_capacity(width);
        fill_row(
            &mut current,
            input,
            mask,
            image_width,
            self.grid,
            dimensions,
            start_cell_x,
            start_cell_y,
            width,
        );
        if height > 1 {
            fill_row(
                &mut next,
                input,
                mask,
                image_width,
                self.grid,
                dimensions,
                start_cell_x,
                start_cell_y + 1,
                width,
            );
        }
        let mut errors = vec![[0i32; 3]; width];
        let mut next_errors = vec![[0i32; 3]; width];

        for cell_y in 0..height {
            let reverse = (start_cell_y + cell_y as i64).rem_euclid(2) == 1;
            let direction = if reverse { -1 } else { 1 };
            for column in 0..width {
                let cell_x = if reverse { width - column - 1 } else { column };
                let Some(source) = current[cell_x] else {
                    continue;
                };
                let adjusted = std::array::from_fn(|channel| {
                    (i32::from(source[channel]) * FIXED_SCALE + errors[cell_x][channel])
                        .clamp(0, 255 * FIXED_SCALE)
                });
                let colour = self.palette.nearest_colour(
                    adjusted.map(|channel| ((channel + FIXED_SCALE / 2) / FIXED_SCALE) as u8),
                );
                write_cell(
                    input,
                    output,
                    mask,
                    image_width,
                    cell_bounds(
                        self.grid,
                        start_cell_x + cell_x as i64,
                        start_cell_y + cell_y as i64,
                        dimensions,
                    ),
                    colour,
                );
                let error = std::array::from_fn(|channel| {
                    adjusted[channel] - i32::from(colour[channel]) * FIXED_SCALE
                });
                let weights = adjusted
                    .map(|channel| coefficients(((channel + FIXED_SCALE / 2) / FIXED_SCALE) as u8));
                let divisors = weights.map(|weights| weights.iter().sum::<u16>());

                diffuse_to(
                    cell_x as i64 + direction,
                    &current,
                    &mut errors,
                    &error,
                    &weights,
                    &divisors,
                    0,
                );
                diffuse_to(
                    cell_x as i64 - direction,
                    &next,
                    &mut next_errors,
                    &error,
                    &weights,
                    &divisors,
                    1,
                );
                diffuse_to(
                    cell_x as i64,
                    &next,
                    &mut next_errors,
                    &error,
                    &weights,
                    &divisors,
                    2,
                );
            }

            std::mem::swap(&mut current, &mut next);
            std::mem::swap(&mut errors, &mut next_errors);
            next_errors.fill([0; 3]);
            next.clear();
            if cell_y + 2 < height {
                fill_row(
                    &mut next,
                    input,
                    mask,
                    image_width,
                    self.grid,
                    dimensions,
                    start_cell_x,
                    start_cell_y + cell_y as i64 + 2,
                    width,
                );
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_row(
    row: &mut Vec<Option<[u8; 3]>>,
    input: &[u8],
    mask: &Mask,
    image_width: usize,
    grid: PixelGrid,
    dimensions: (u32, u32),
    start_cell_x: i64,
    cell_y: i64,
    width: usize,
) {
    row.extend((0..width).map(|cell_x| {
        sample_cell(
            input,
            mask,
            image_width,
            cell_bounds(grid, start_cell_x + cell_x as i64, cell_y, dimensions),
            grid.sampling(),
        )
    }));
}

fn diffuse_to(
    target_x: i64,
    row: &[Option<[u8; 3]>],
    errors: &mut [[i32; 3]],
    error: &[i32; 3],
    weights: &[[u16; 3]; 3],
    divisors: &[u16; 3],
    coefficient: usize,
) {
    if target_x < 0 || target_x >= row.len() as i64 || row[target_x as usize].is_none() {
        return;
    }
    let target = &mut errors[target_x as usize];
    for channel in 0..3 {
        let contribution = i64::from(error[channel]) * i64::from(weights[channel][coefficient])
            / i64::from(divisors[channel]);
        target[channel] = target[channel].saturating_add(contribution as i32);
    }
}

const fn coefficients(tone: u8) -> [u16; 3] {
    let index = if tone > 127 { 255 - tone } else { tone };
    COEFFICIENTS[index as usize]
}

#[cfg(test)]
pub(super) const fn published_coefficients(tone: u8) -> [u16; 3] {
    coefficients(tone)
}
