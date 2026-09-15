use crate::{DitherError, ErrorKind, Mask, Result};

pub(super) fn quantise_cells(
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

pub(super) fn align_to_grid(coordinate: u32, pixel_size: u32) -> u32 {
    coordinate - coordinate % pixel_size
}

/// Returns exclusive image bounds for a logical pixel.
pub(super) fn cell_bounds(
    x: u32,
    y: u32,
    pixel_size: u32,
    dimensions: (u32, u32),
) -> (u32, u32, u32, u32) {
    (
        x,
        y,
        x.saturating_add(pixel_size).min(dimensions.0),
        y.saturating_add(pixel_size).min(dimensions.1),
    )
}

/// Averages selected RGB values within a logical pixel using mask coverage.
pub(super) fn sample_cell(
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

/// Prevents two-cell diffusion taps from jumping across an unselected cell.
pub(super) fn validate_pixel_size(pixel_size: u32) -> Result<u32> {
    if pixel_size == 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "dither pixel size must be greater than zero",
        ));
    }

    Ok(pixel_size)
}
