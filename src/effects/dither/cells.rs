use std::collections::{HashMap, hash_map::Entry};

use crate::{DitherError, ErrorKind, Mask, Result};

/// The source-colour sampling used for each logical pixel.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum SamplingMode {
    /// Uses the mask-coverage-weighted average colour.
    #[default]
    Average,
    /// Uses the selected source pixel nearest the cell centre.
    Centre,
    /// Uses the selected source pixel with the lowest RGB luma.
    Darkest,
    /// Uses the selected source pixel with the highest RGB luma.
    Lightest,
    /// Uses the exact source colour with the greatest mask coverage.
    DominantColour,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PixelGrid {
    width: u32,
    height: u32,
    offset: (i32, i32),
    sampling: SamplingMode,
}

impl PixelGrid {
    pub(super) const fn new() -> Self {
        Self {
            width: 1,
            height: 1,
            offset: (0, 0),
            sampling: SamplingMode::Average,
        }
    }

    pub(super) fn with_width(mut self, width: u32) -> Result<Self> {
        self.width = validate_pixel_dimension(width)?;
        Ok(self)
    }

    pub(super) fn with_height(mut self, height: u32) -> Result<Self> {
        self.height = validate_pixel_dimension(height)?;
        Ok(self)
    }

    pub(super) fn with_size(mut self, width: u32, height: u32) -> Result<Self> {
        self.width = validate_pixel_dimension(width)?;
        self.height = validate_pixel_dimension(height)?;
        Ok(self)
    }

    pub(super) const fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = (x, y);
        self
    }

    pub(super) const fn with_sampling(mut self, sampling: SamplingMode) -> Self {
        self.sampling = sampling;
        self
    }

    pub(super) const fn width(self) -> u32 {
        self.width
    }

    pub(super) const fn height(self) -> u32 {
        self.height
    }

    pub(super) const fn size(self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(super) const fn offset(self) -> (i32, i32) {
        self.offset
    }

    pub(super) const fn sampling(self) -> SamplingMode {
        self.sampling
    }

    pub(super) fn cell_x(self, x: u32) -> i64 {
        cell_index(x, self.width, self.offset.0)
    }

    pub(super) fn cell_y(self, y: u32) -> i64 {
        cell_index(y, self.height, self.offset.1)
    }

    pub(super) fn origin(self, cell_x: i64, cell_y: i64) -> (i64, i64) {
        (
            i64::from(self.offset.0) + cell_x * i64::from(self.width),
            i64::from(self.offset.1) + cell_y * i64::from(self.height),
        )
    }
}

pub(super) fn quantise_cells(
    input: &[u8],
    output: &mut [u8],
    dimensions: (u32, u32),
    mask: &Mask,
    grid: PixelGrid,
    mut quantise: impl FnMut([u8; 3], i64, i64) -> [u8; 3],
) {
    let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
        return;
    };
    let image_width = dimensions.0 as usize;

    if grid.size() == (1, 1) {
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
                    grid.cell_x(x),
                    grid.cell_y(y),
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

    for cell_y in grid.cell_y(min_y)..=grid.cell_y(max_y - 1) {
        for cell_x in grid.cell_x(min_x)..=grid.cell_x(max_x - 1) {
            let bounds = cell_bounds(grid, cell_x, cell_y, dimensions);
            if let Some(colour) = sample_cell(input, mask, image_width, bounds, grid.sampling()) {
                let colour = quantise(colour, cell_x, cell_y);
                write_cell(input, output, mask, image_width, bounds, colour);
            }
        }
    }
}

/// Returns exclusive, image-clipped bounds for a logical pixel.
pub(super) fn cell_bounds(
    grid: PixelGrid,
    cell_x: i64,
    cell_y: i64,
    dimensions: (u32, u32),
) -> (u32, u32, u32, u32) {
    let (x, y) = grid.origin(cell_x, cell_y);
    (
        x.clamp(0, i64::from(dimensions.0)) as u32,
        y.clamp(0, i64::from(dimensions.1)) as u32,
        (x + i64::from(grid.width())).clamp(0, i64::from(dimensions.0)) as u32,
        (y + i64::from(grid.height())).clamp(0, i64::from(dimensions.1)) as u32,
    )
}

/// Samples selected RGB values within an image-clipped logical pixel.
pub(super) fn sample_cell(
    input: &[u8],
    mask: &Mask,
    image_width: usize,
    bounds: (u32, u32, u32, u32),
    sampling: SamplingMode,
) -> Option<[u8; 3]> {
    match sampling {
        SamplingMode::Average => {
            let mut totals = [0_u64; 3];
            let mut total_coverage = 0_u64;
            for_each_selected(
                input,
                mask,
                image_width,
                bounds,
                |_, _, colour, coverage| {
                    for channel in 0..3 {
                        totals[channel] += u64::from(colour[channel]) * u64::from(coverage);
                    }
                    total_coverage += u64::from(coverage);
                },
            );
            (total_coverage != 0)
                .then(|| totals.map(|total| ((total + total_coverage / 2) / total_coverage) as u8))
        }
        SamplingMode::Centre => {
            let centre_x = bounds.0 + (bounds.2 - bounds.0) / 2;
            let centre_y = bounds.1 + (bounds.3 - bounds.1) / 2;
            let mut sample = None;
            let mut nearest = u128::MAX;
            for_each_selected(input, mask, image_width, bounds, |x, y, colour, _| {
                let distance = u128::from(x.abs_diff(centre_x)).pow(2)
                    + u128::from(y.abs_diff(centre_y)).pow(2);
                if distance < nearest {
                    nearest = distance;
                    sample = Some(colour);
                }
            });
            sample
        }
        SamplingMode::Darkest | SamplingMode::Lightest => {
            let mut sample = None;
            let mut sampled_luma = 0;
            for_each_selected(input, mask, image_width, bounds, |_, _, colour, _| {
                let luma = rgb_luma(colour);
                if sample.is_none()
                    || matches!(sampling, SamplingMode::Darkest) && luma < sampled_luma
                    || matches!(sampling, SamplingMode::Lightest) && luma > sampled_luma
                {
                    sample = Some(colour);
                    sampled_luma = luma;
                }
            });
            sample
        }
        SamplingMode::DominantColour => {
            let mut coverage_by_colour = HashMap::<[u8; 3], (u64, usize)>::new();
            let mut sample = None;
            let mut greatest_coverage = 0;
            let mut earliest = usize::MAX;
            for_each_selected(
                input,
                mask,
                image_width,
                bounds,
                |_, _, colour, coverage| {
                    let next_order = coverage_by_colour.len();
                    let (total, order) = match coverage_by_colour.entry(colour) {
                        Entry::Occupied(entry) => entry.into_mut(),
                        Entry::Vacant(entry) => entry.insert((0, next_order)),
                    };
                    *total += u64::from(coverage);
                    if *total > greatest_coverage
                        || *total == greatest_coverage && *order < earliest
                    {
                        sample = Some(colour);
                        greatest_coverage = *total;
                        earliest = *order;
                    }
                },
            );
            sample
        }
    }
}

fn for_each_selected(
    input: &[u8],
    mask: &Mask,
    image_width: usize,
    bounds: (u32, u32, u32, u32),
    mut visit: impl FnMut(u32, u32, [u8; 3], u8),
) {
    for y in bounds.1..bounds.3 {
        for x in bounds.0..bounds.2 {
            let pixel_index = y as usize * image_width + x as usize;
            let coverage = mask.coverage_bytes()[pixel_index];
            if coverage == 0 {
                continue;
            }
            let byte_index = pixel_index * 4;
            visit(
                x,
                y,
                [
                    input[byte_index],
                    input[byte_index + 1],
                    input[byte_index + 2],
                ],
                coverage,
            );
        }
    }
}

fn rgb_luma(colour: [u8; 3]) -> u32 {
    299 * u32::from(colour[0]) + 587 * u32::from(colour[1]) + 114 * u32::from(colour[2])
}

/// Fills the selected portion of a logical pixel while preserving alpha.
pub(super) fn write_cell(
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

fn cell_index(coordinate: u32, size: u32, offset: i32) -> i64 {
    (i64::from(coordinate) - i64::from(offset)).div_euclid(i64::from(size))
}

fn validate_pixel_dimension(dimension: u32) -> Result<u32> {
    if dimension == 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "dither pixel dimensions must be greater than zero",
        ));
    }
    Ok(dimension)
}
