use super::cells::{quantise_cells, validate_pixel_size};
use super::noise::{BLUE_NOISE_16X16, BLUE_NOISE_SIZE};
use super::palette::Palette;
use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const THRESHOLD_STRENGTH_SCALE: u16 = 256;

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

/// A validated rectangular pattern of threshold ranks.
///
/// Each rank must be less than the number of entries in the map. Ranks may be
/// repeated, which permits patterns beyond dispersed-dot matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdMap {
    width: u32,
    height: u32,
    thresholds: Box<[u32]>,
}

impl ThresholdMap {
    /// Creates a custom row-major threshold map.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when either dimension is zero,
    /// the number of thresholds does not match the dimensions, or a threshold
    /// is outside the map's rank range.
    pub fn new(width: u32, height: u32, thresholds: impl Into<Box<[u32]>>) -> Result<Self> {
        let thresholds = thresholds.into();
        let Some(length) = width.checked_mul(height) else {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "threshold map dimensions are too large",
            ));
        };
        if length == 0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "threshold map dimensions must be greater than zero",
            ));
        }
        if thresholds.len() != length as usize {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "threshold count must match the map dimensions",
            ));
        }
        if thresholds.iter().any(|&threshold| threshold >= length) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "threshold ranks must be less than the map length",
            ));
        }

        Ok(Self {
            width,
            height,
            thresholds,
        })
    }

    /// Creates the standard 2x2 Bayer threshold map.
    pub fn bayer_2x2() -> Self {
        Self::bayer(2)
    }

    /// Creates the standard 4x4 Bayer threshold map.
    pub fn bayer_4x4() -> Self {
        Self::bayer(4)
    }

    /// Creates the standard 8x8 Bayer threshold map.
    pub fn bayer_8x8() -> Self {
        Self::bayer(8)
    }

    /// Creates a 16x16 blue-noise threshold map.
    pub fn blue_noise_16x16() -> Self {
        Self::preset(BLUE_NOISE_SIZE, BLUE_NOISE_SIZE, BLUE_NOISE_16X16)
    }

    /// Creates a 6x6 clustered-dot threshold map.
    pub fn clustered_dots() -> Self {
        Self::preset(
            6,
            6,
            [
                34, 25, 21, 17, 29, 33, 30, 13, 9, 5, 12, 24, 18, 6, 1, 0, 8, 20, 22, 10, 2, 3, 4,
                16, 26, 14, 7, 11, 15, 28, 35, 31, 19, 23, 27, 32,
            ],
        )
    }

    /// Creates a 4x4 horizontal-line threshold map.
    pub fn horizontal_lines() -> Self {
        Self::preset(4, 4, [0, 0, 0, 0, 8, 8, 8, 8, 12, 12, 12, 12, 4, 4, 4, 4])
    }

    /// Creates a 4x4 vertical-line threshold map.
    pub fn vertical_lines() -> Self {
        Self::preset(4, 4, [0, 8, 12, 4, 0, 8, 12, 4, 0, 8, 12, 4, 0, 8, 12, 4])
    }

    /// Creates a 4x4 diagonal-line threshold map.
    pub fn diagonal_lines() -> Self {
        Self::preset(4, 4, [0, 4, 8, 12, 4, 8, 12, 0, 8, 12, 0, 4, 12, 0, 4, 8])
    }

    /// Creates a 4x4 crosshatch threshold map.
    pub fn crosshatch() -> Self {
        Self::preset(4, 4, [0, 0, 0, 0, 0, 8, 8, 4, 0, 8, 12, 4, 0, 4, 4, 4])
    }

    /// Creates a 4x4 checkerboard threshold map.
    pub fn checkerboard() -> Self {
        Self::preset(
            4,
            4,
            [0, 0, 12, 12, 0, 0, 12, 12, 12, 12, 4, 4, 12, 12, 4, 4],
        )
    }

    /// Creates a 3x3 dispersed-dot threshold map.
    pub fn dispersed_dots_3x3() -> Self {
        Self::preset(3, 3, [6, 8, 4, 1, 0, 3, 5, 2, 7])
    }

    /// Creates a 5x5 dispersed-dot threshold map.
    pub fn dispersed_dots_5x5() -> Self {
        Self::preset(
            5,
            5,
            [
                0, 12, 3, 15, 6, 17, 9, 21, 1, 13, 4, 16, 7, 19, 10, 22, 2, 14, 5, 18, 8, 20, 11,
                23, 24,
            ],
        )
    }

    /// Returns the map width.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the map height.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the row-major threshold ranks.
    pub fn thresholds(&self) -> &[u32] {
        &self.thresholds
    }

    fn bayer(size: u32) -> Self {
        Self {
            width: size,
            height: size,
            thresholds: (0..size)
                .flat_map(|y| (0..size).map(move |x| u32::from(bayer_value(x, y, size as u8))))
                .collect(),
        }
    }

    fn preset<const LENGTH: usize>(width: u32, height: u32, thresholds: [u32; LENGTH]) -> Self {
        debug_assert_eq!(width as usize * height as usize, LENGTH);
        debug_assert!(
            thresholds
                .iter()
                .all(|&threshold| threshold < LENGTH as u32)
        );
        Self {
            width,
            height,
            thresholds: Box::new(thresholds),
        }
    }

    fn bayer_size(&self) -> Option<u32> {
        matches!(self.width, 2 | 4 | 8)
            .then_some(self.width)
            .filter(|&size| self.height == size)
            .filter(|&size| {
                self.thresholds.iter().enumerate().all(|(index, &value)| {
                    let index = index as u32;
                    value == u32::from(bayer_value(index % size, index / size, size as u8))
                })
            })
    }
}

/// A clockwise rotation applied to a threshold map.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ThresholdRotation {
    /// Leaves the map unrotated.
    #[default]
    None,
    /// Rotates the map 90 degrees clockwise.
    Clockwise90,
    /// Rotates the map 180 degrees clockwise.
    Clockwise180,
    /// Rotates the map 270 degrees clockwise.
    Clockwise270,
}

/// Applies ordered dithering using a configurable threshold map.
///
/// The map is anchored to image-origin logical pixel coordinates, including
/// when rendering a polygon selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedDither {
    palette: Palette,
    map: ThresholdMap,
    pixel_size: u32,
    strength: u16,
    offset: (i32, i32),
    rotation: ThresholdRotation,
    mirror_x: bool,
    mirror_y: bool,
}

impl OrderedDither {
    /// Creates ordered dithering with a validated threshold map.
    pub const fn new(palette: Palette, map: ThresholdMap) -> Self {
        Self {
            palette,
            map,
            pixel_size: 1,
            strength: THRESHOLD_STRENGTH_SCALE,
            offset: (0, 0),
            rotation: ThresholdRotation::None,
            mirror_x: false,
            mirror_y: false,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the threshold map.
    pub const fn map(&self) -> &ThresholdMap {
        &self.map
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

    /// Sets the strength of the threshold adjustment.
    ///
    /// Strength is rounded to the nearest 1/256. Zero disables the map's
    /// adjustment, one uses it unchanged, and two doubles it.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] unless `strength` is finite and
    /// between zero and two inclusive.
    pub fn with_strength(mut self, strength: f32) -> Result<Self> {
        if !strength.is_finite() || !(0.0..=2.0).contains(&strength) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "threshold strength must be between zero and two",
            ));
        }
        self.strength = (strength * f32::from(THRESHOLD_STRENGTH_SCALE)).round() as u16;
        Ok(self)
    }

    /// Returns the threshold adjustment strength.
    pub fn strength(&self) -> f32 {
        f32::from(self.strength) / f32::from(THRESHOLD_STRENGTH_SCALE)
    }

    /// Offsets the map by logical pixels along the x and y axes.
    pub const fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = (x, y);
        self
    }

    /// Returns the map offset as `(x, y)` logical pixels.
    pub const fn offset(&self) -> (i32, i32) {
        self.offset
    }

    /// Rotates the threshold map clockwise around its origin.
    pub const fn with_rotation(mut self, rotation: ThresholdRotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Returns the map rotation.
    pub const fn rotation(&self) -> ThresholdRotation {
        self.rotation
    }

    /// Mirrors the transformed map horizontally and vertically.
    pub const fn with_mirroring(mut self, horizontal: bool, vertical: bool) -> Self {
        self.mirror_x = horizontal;
        self.mirror_y = vertical;
        self
    }

    /// Returns whether horizontal mirroring is enabled.
    pub const fn mirror_x(&self) -> bool {
        self.mirror_x
    }

    /// Returns whether vertical mirroring is enabled.
    pub const fn mirror_y(&self) -> bool {
        self.mirror_y
    }

    pub(super) fn transformed_dimensions(&self) -> (u32, u32) {
        match self.rotation {
            ThresholdRotation::None | ThresholdRotation::Clockwise180 => {
                (self.map.width, self.map.height)
            }
            ThresholdRotation::Clockwise90 | ThresholdRotation::Clockwise270 => {
                (self.map.height, self.map.width)
            }
        }
    }

    fn transformed_threshold(&self, mut x: u32, mut y: u32) -> u32 {
        let (width, height) = self.transformed_dimensions();
        debug_assert!(x < width && y < height);
        if self.mirror_x {
            x = width - x - 1;
        }
        if self.mirror_y {
            y = height - y - 1;
        }
        let (x, y) = match self.rotation {
            ThresholdRotation::None => (x, y),
            ThresholdRotation::Clockwise90 => (y, self.map.height - x - 1),
            ThresholdRotation::Clockwise180 => (self.map.width - x - 1, self.map.height - y - 1),
            ThresholdRotation::Clockwise270 => (self.map.width - y - 1, x),
        };
        self.map.thresholds[(y * self.map.width + x) as usize]
    }

    #[cfg(test)]
    pub(super) fn threshold(&self, x: u32, y: u32) -> u32 {
        let (width, height) = self.transformed_dimensions();
        let x = (i64::from(x) - i64::from(self.offset.0)).rem_euclid(i64::from(width)) as u32;
        let y = (i64::from(y) - i64::from(self.offset.1)).rem_euclid(i64::from(height)) as u32;
        self.transformed_threshold(x, y)
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
        let bayer_size = self.map.bayer_size();
        if self.strength == THRESHOLD_STRENGTH_SCALE
            && self.offset == (0, 0)
            && self.rotation == ThresholdRotation::None
            && !self.mirror_x
            && !self.mirror_y
            && let Some(size) = bayer_size
        {
            let levels = (size * size) as i32;
            let mut adjustments = [0; 64];
            for y in 0..size {
                for x in 0..size {
                    let threshold = i32::from(bayer_value(x, y, size as u8));
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
            return Ok(());
        }

        let (width, height) = self.transformed_dimensions();
        let levels = i64::from(self.map.width * self.map.height);
        let adjustments = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let threshold = i64::from(self.transformed_threshold(x, y));
                    let adjustment = (2 * threshold + 1 - levels) * 255 / (2 * levels);
                    (adjustment * i64::from(self.strength) / i64::from(THRESHOLD_STRENGTH_SCALE))
                        as i32
                })
            })
            .collect::<Box<[_]>>();
        let offset_x = i64::from(self.offset.0).rem_euclid(i64::from(width)) as u32;
        let offset_y = i64::from(self.offset.1).rem_euclid(i64::from(height)) as u32;
        if offset_x == 0 && offset_y == 0 {
            quantise_cells(
                input,
                output,
                dimensions,
                mask,
                self.pixel_size,
                |colour, x, y| {
                    let adjustment = adjustments[((y % height) * width + x % width) as usize];
                    let colour =
                        colour.map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8);
                    self.palette.nearest_colour(colour)
                },
            );
            return Ok(());
        }
        quantise_cells(
            input,
            output,
            dimensions,
            mask,
            self.pixel_size,
            |colour, x, y| {
                let x = x % width;
                let y = y % height;
                let x = if x >= offset_x {
                    x - offset_x
                } else {
                    width - (offset_x - x)
                };
                let y = if y >= offset_y {
                    y - offset_y
                } else {
                    height - (offset_y - y)
                };
                let adjustment = adjustments[(y * width + x) as usize];
                let colour =
                    colour.map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8);
                self.palette.nearest_colour(colour)
            },
        );
        Ok(())
    }
}

/// The dot geometry used by a [`Halftone`] screen.
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
