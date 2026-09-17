use std::collections::VecDeque;

use super::cells::{PixelGrid, SamplingMode, cell_bounds, sample_cell, write_cell};
use super::palette::Palette;
use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const DEFAULT_HISTORY_LENGTH: usize = 16;
const DEFAULT_DECAY: f32 = 16.0;

/// Applies error diffusion along a Hilbert space-filling curve.
///
/// Recent quantisation errors are carried along the curve with exponentially
/// decaying weights. Unselected cells and discontinuities caused by clipping
/// the curve to a rectangular image reset the history.
#[derive(Clone, Debug, PartialEq)]
pub struct RiemersmaDither {
    palette: Palette,
    grid: PixelGrid,
    history_length: usize,
    decay: f32,
}

impl RiemersmaDither {
    /// Creates Riemersma dithering with a 16-entry history and 16:1 decay.
    pub const fn new(palette: Palette) -> Self {
        Self {
            palette,
            grid: PixelGrid::new(),
            history_length: DEFAULT_HISTORY_LENGTH,
            decay: DEFAULT_DECAY,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Sets the number of recent quantisation errors retained.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `length` is zero.
    pub fn with_history_length(mut self, length: usize) -> Result<Self> {
        if length == 0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "Riemersma history length must be greater than zero",
            ));
        }
        self.history_length = length;
        Ok(self)
    }

    /// Returns the number of recent quantisation errors retained.
    pub const fn history_length(&self) -> usize {
        self.history_length
    }

    /// Sets the newest-to-oldest error-weight ratio.
    ///
    /// A value of one weights every retained error equally. Larger values make
    /// older errors decay more strongly.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] unless `decay` is finite and at
    /// least one.
    pub fn with_decay(mut self, decay: f32) -> Result<Self> {
        if !decay.is_finite() || decay < 1.0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "Riemersma decay must be finite and at least one",
            ));
        }
        self.decay = decay;
        Ok(self)
    }

    /// Returns the newest-to-oldest error-weight ratio.
    pub const fn decay(&self) -> f32 {
        self.decay
    }

    /// Sets the logical pixel width.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `width` is zero.
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
    /// Returns [`ErrorKind::InvalidParameter`] when `height` is zero.
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
    /// Returns [`ErrorKind::InvalidParameter`] when either dimension is zero.
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

impl Effect for RiemersmaDither {
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
        let start_cell_x = self.grid.cell_x(min_x);
        let start_cell_y = self.grid.cell_y(min_y);
        let width = (self.grid.cell_x(max_x - 1) - start_cell_x + 1) as u64;
        let height = (self.grid.cell_y(max_y - 1) - start_cell_y + 1) as u64;
        let image_width = dimensions.0 as usize;
        let weights = error_weights(self.history_length, self.decay);
        let mut history = VecDeque::<[f32; 3]>::with_capacity(self.history_length);
        let mut previous = None;

        for_each_hilbert_cell(width, height, |x, y| {
            if previous.is_some_and(|(previous_x, previous_y): (u64, u64)| {
                previous_x.abs_diff(x) + previous_y.abs_diff(y) != 1
            }) {
                history.clear();
            }
            previous = Some((x, y));

            let bounds = cell_bounds(
                self.grid,
                start_cell_x + x as i64,
                start_cell_y + y as i64,
                dimensions,
            );
            let Some(source) = sample_cell(input, mask, image_width, bounds, self.grid.sampling())
            else {
                history.clear();
                return;
            };

            let weight_offset = weights.len() - history.len();
            let mut adjusted = source.map(f32::from);
            for (error, &weight) in history.iter().zip(&weights[weight_offset..]) {
                for channel in 0..3 {
                    adjusted[channel] += error[channel] * weight;
                }
            }
            let adjusted = adjusted.map(|channel| channel.clamp(0.0, 255.0).round() as u8);
            let colour = self.palette.nearest_colour(adjusted);
            write_cell(input, output, mask, image_width, bounds, colour);

            if history.len() == self.history_length {
                history.pop_front();
            }
            history.push_back(std::array::from_fn(|channel| {
                f32::from(source[channel]) - f32::from(colour[channel])
            }));
        });
        Ok(())
    }
}

fn error_weights(length: usize, decay: f32) -> Vec<f32> {
    if length == 1 {
        return vec![1.0];
    }
    (0..length)
        .map(|index| decay.powf(index as f32 / (length - 1) as f32 - 1.0))
        .collect()
}

fn for_each_hilbert_cell(width: u64, height: u64, mut visit: impl FnMut(u64, u64)) {
    let side = width.max(height).next_power_of_two();
    // ponytail: padding avoids storing and sorting the curve; add recursive
    // pruning if extremely narrow, high-resolution images become important.
    for index in 0..u128::from(side) * u128::from(side) {
        let (x, y) = hilbert_point(side, index);
        if x < width && y < height {
            visit(x, y);
        }
    }
}

fn hilbert_point(side: u64, mut index: u128) -> (u64, u64) {
    let (mut x, mut y) = (0, 0);
    let mut scale = 1;
    while scale < side {
        let right = ((index / 2) & 1) as u64;
        let up = ((index ^ u128::from(right)) & 1) as u64;
        if up == 0 {
            if right == 1 {
                x = scale - 1 - x;
                y = scale - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        x += scale * right;
        y += scale * up;
        index /= 4;
        scale *= 2;
    }
    (x, y)
}

#[cfg(test)]
pub(super) fn hilbert_cells(width: u64, height: u64) -> Vec<(u64, u64)> {
    let mut cells = Vec::new();
    for_each_hilbert_cell(width, height, |x, y| cells.push((x, y)));
    cells
}
