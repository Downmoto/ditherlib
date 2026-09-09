use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const FIXED_SCALE: i32 = 256;
const DIFFUSION_STRENGTH_SCALE: u16 = 256;
const THRESHOLD_STRENGTH_SCALE: u16 = 256;
const FLOYD_STEINBERG: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 7),
    DiffusionTap::new(-1, 1, 3),
    DiffusionTap::new(0, 1, 5),
    DiffusionTap::new(1, 1, 1),
];
const ATKINSON: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 1),
    DiffusionTap::new(2, 0, 1),
    DiffusionTap::new(-1, 1, 1),
    DiffusionTap::new(0, 1, 1),
    DiffusionTap::new(1, 1, 1),
    DiffusionTap::new(0, 2, 1),
];
const JARVIS_JUDICE_NINKE: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 7),
    DiffusionTap::new(2, 0, 5),
    DiffusionTap::new(-2, 1, 3),
    DiffusionTap::new(-1, 1, 5),
    DiffusionTap::new(0, 1, 7),
    DiffusionTap::new(1, 1, 5),
    DiffusionTap::new(2, 1, 3),
    DiffusionTap::new(-2, 2, 1),
    DiffusionTap::new(-1, 2, 3),
    DiffusionTap::new(0, 2, 5),
    DiffusionTap::new(1, 2, 3),
    DiffusionTap::new(2, 2, 1),
];
const STUCKI: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 8),
    DiffusionTap::new(2, 0, 4),
    DiffusionTap::new(-2, 1, 2),
    DiffusionTap::new(-1, 1, 4),
    DiffusionTap::new(0, 1, 8),
    DiffusionTap::new(1, 1, 4),
    DiffusionTap::new(2, 1, 2),
    DiffusionTap::new(-2, 2, 1),
    DiffusionTap::new(-1, 2, 2),
    DiffusionTap::new(0, 2, 4),
    DiffusionTap::new(1, 2, 2),
    DiffusionTap::new(2, 2, 1),
];
const BURKES: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 8),
    DiffusionTap::new(2, 0, 4),
    DiffusionTap::new(-2, 1, 2),
    DiffusionTap::new(-1, 1, 4),
    DiffusionTap::new(0, 1, 8),
    DiffusionTap::new(1, 1, 4),
    DiffusionTap::new(2, 1, 2),
];
const SIERRA: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 5),
    DiffusionTap::new(2, 0, 3),
    DiffusionTap::new(-2, 1, 2),
    DiffusionTap::new(-1, 1, 4),
    DiffusionTap::new(0, 1, 5),
    DiffusionTap::new(1, 1, 4),
    DiffusionTap::new(2, 1, 2),
    DiffusionTap::new(-1, 2, 2),
    DiffusionTap::new(0, 2, 3),
    DiffusionTap::new(1, 2, 2),
];
const TWO_ROW_SIERRA: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 4),
    DiffusionTap::new(2, 0, 3),
    DiffusionTap::new(-2, 1, 1),
    DiffusionTap::new(-1, 1, 2),
    DiffusionTap::new(0, 1, 3),
    DiffusionTap::new(1, 1, 2),
    DiffusionTap::new(2, 1, 1),
];
const SIERRA_LITE: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 2),
    DiffusionTap::new(-1, 1, 1),
    DiffusionTap::new(0, 1, 1),
];
const FALSE_FLOYD_STEINBERG: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 3),
    DiffusionTap::new(-1, 1, 3),
    DiffusionTap::new(0, 1, 2),
];
const FAN: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 7),
    DiffusionTap::new(-2, 1, 1),
    DiffusionTap::new(-1, 1, 3),
    DiffusionTap::new(0, 1, 5),
];
const SHIAU_FAN: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 4),
    DiffusionTap::new(-2, 1, 1),
    DiffusionTap::new(-1, 1, 1),
    DiffusionTap::new(0, 1, 2),
];
const SHIAU_FAN_2: &[DiffusionTap] = &[
    DiffusionTap::new(1, 0, 8),
    DiffusionTap::new(-3, 1, 1),
    DiffusionTap::new(-2, 1, 1),
    DiffusionTap::new(-1, 1, 2),
    DiffusionTap::new(0, 1, 4),
];
const STEVENSON_ARCE: &[DiffusionTap] = &[
    DiffusionTap::new(2, 0, 32),
    DiffusionTap::new(-3, 1, 12),
    DiffusionTap::new(-1, 1, 26),
    DiffusionTap::new(1, 1, 30),
    DiffusionTap::new(3, 1, 16),
    DiffusionTap::new(-2, 2, 12),
    DiffusionTap::new(0, 2, 26),
    DiffusionTap::new(2, 2, 12),
    DiffusionTap::new(-3, 3, 5),
    DiffusionTap::new(-1, 3, 12),
    DiffusionTap::new(1, 3, 12),
    DiffusionTap::new(3, 3, 5),
];
const TWO_DIMENSIONAL_KNUTH: &[DiffusionTap] =
    &[DiffusionTap::new(1, 0, 1), DiffusionTap::new(0, 1, 1)];

/// An RGB colour with 8-bit channels.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Colour {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
}

impl Colour {
    /// Black.
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White.
    pub const WHITE: Self = Self::new(255, 255, 255);
    /// Red.
    pub const RED: Self = Self::new(255, 0, 0);
    /// Green.
    pub const GREEN: Self = Self::new(0, 255, 0);
    /// Blue.
    pub const BLUE: Self = Self::new(0, 0, 255);
    /// Cyan.
    pub const CYAN: Self = Self::new(0, 255, 255);
    /// Magenta.
    pub const MAGENTA: Self = Self::new(255, 0, 255);
    /// Yellow.
    pub const YELLOW: Self = Self::new(255, 255, 0);

    /// Creates an RGB colour.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Returns the colour as an RGB byte array.
    pub const fn rgb(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }
}

impl From<[u8; 3]> for Colour {
    fn from([red, green, blue]: [u8; 3]) -> Self {
        Self::new(red, green, blue)
    }
}

impl From<&[u8; 3]> for Colour {
    fn from(colour: &[u8; 3]) -> Self {
        Self::from(*colour)
    }
}

impl From<&Colour> for Colour {
    fn from(colour: &Colour) -> Self {
        *colour
    }
}

impl From<Colour> for [u8; 3] {
    fn from(colour: Colour) -> Self {
        colour.rgb()
    }
}

/// American English alias for [`Colour`].
pub type Color = Colour;

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
    pub fn new<C>(colours: impl IntoIterator<Item = C>) -> Result<Self>
    where
        C: Into<Colour>,
    {
        let colours = colours
            .into_iter()
            .map(|colour| colour.into().rgb())
            .collect::<Vec<_>>()
            .into_boxed_slice();

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
    pub fn monochrome(colour: impl Into<Colour>) -> Self {
        let colour = colour.into().rgb();
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

    /// Creates an eight-level greyscale palette.
    pub fn greyscale() -> Self {
        Self {
            colours: Box::new([
                [0, 0, 0],
                [36, 36, 36],
                [73, 73, 73],
                [109, 109, 109],
                [146, 146, 146],
                [182, 182, 182],
                [219, 219, 219],
                [255, 255, 255],
            ]),
        }
    }

    /// Creates a four-colour green palette inspired by the original Game Boy.
    pub fn game_boy() -> Self {
        Self {
            colours: Box::new([[15, 56, 15], [48, 98, 48], [139, 172, 15], [155, 188, 15]]),
        }
    }

    /// Creates the standard 16-colour CGA palette.
    pub fn cga() -> Self {
        Self {
            colours: Box::new([
                [0, 0, 0],
                [0, 0, 170],
                [0, 170, 0],
                [0, 170, 170],
                [170, 0, 0],
                [170, 0, 170],
                [170, 85, 0],
                [170, 170, 170],
                [85, 85, 85],
                [85, 85, 255],
                [85, 255, 85],
                [85, 255, 255],
                [255, 85, 85],
                [255, 85, 255],
                [255, 255, 85],
                [255, 255, 255],
            ]),
        }
    }

    /// Creates the standard 16-colour PICO-8 palette.
    pub fn pico_8() -> Self {
        Self {
            colours: Box::new([
                [0, 0, 0],
                [29, 43, 83],
                [126, 37, 83],
                [0, 135, 81],
                [171, 82, 54],
                [95, 87, 79],
                [194, 195, 199],
                [255, 241, 232],
                [255, 0, 77],
                [255, 163, 0],
                [255, 236, 39],
                [0, 228, 54],
                [41, 173, 255],
                [131, 118, 156],
                [255, 119, 168],
                [255, 204, 170],
            ]),
        }
    }

    /// Returns the palette's RGB colours in matching order.
    pub fn colours(&self) -> &[[u8; 3]] {
        &self.colours
    }

    /// Iterates over the palette as [`Colour`] values.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Colour> + '_ {
        self.colours.iter().copied().map(Colour::from)
    }

    /// Returns the nearest palette entry as a [`Colour`].
    pub fn nearest(&self, colour: impl Into<Colour>) -> Colour {
        Colour::from(self.nearest_colour(colour.into().rgb()))
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

    fn transformed_dimensions(&self) -> (u32, u32) {
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
    fn threshold(&self, x: u32, y: u32) -> u32 {
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

/// A weighted destination in an error-diffusion kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffusionTap {
    offset_x: i32,
    offset_y: i32,
    weight: u32,
}

impl DiffusionTap {
    /// Creates a diffusion tap relative to the pixel being quantised.
    ///
    /// [`DiffusionKernel::new`] validates that the tap points to a pixel later
    /// in the raster scan and that `weight` is greater than zero.
    pub const fn new(offset_x: i32, offset_y: i32, weight: u32) -> Self {
        Self {
            offset_x,
            offset_y,
            weight,
        }
    }

    /// Returns the horizontal offset from the current pixel.
    pub const fn offset_x(&self) -> i32 {
        self.offset_x
    }

    /// Returns the vertical offset from the current pixel.
    pub const fn offset_y(&self) -> i32 {
        self.offset_y
    }

    /// Returns the error weight applied at this offset.
    pub const fn weight(&self) -> u32 {
        self.weight
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
    /// False Floyd-Steinberg's coarse, strongly directional kernel.
    FalseFloydSteinberg,
    /// Fan's compact kernel with a left-leaning lower row.
    Fan,
    /// Shiau-Fan's short-tailed kernel for reducing worm artefacts.
    ShiauFan,
    /// Shiau-Fan's longer-tailed second kernel for smoother extremes.
    ShiauFan2,
    /// Stevenson-Arce's broad hexagonal kernel with fine, dispersed grain.
    StevensonArce,
    /// Knuth's minimal two-dimensional kernel with a regular diagonal texture.
    TwoDimensionalKnuth,
}

impl DiffusionAlgorithm {
    /// Returns this preset as a configurable diffusion kernel.
    pub fn kernel(self) -> DiffusionKernel {
        let (taps, divisor) = self.specification();
        DiffusionKernel {
            taps: taps.into(),
            divisor,
            preset: Some(self),
        }
    }

    fn specification(self) -> (&'static [DiffusionTap], u32) {
        match self {
            Self::FloydSteinberg => (FLOYD_STEINBERG, 16),
            Self::Atkinson => (ATKINSON, 8),
            Self::JarvisJudiceNinke => (JARVIS_JUDICE_NINKE, 48),
            Self::Stucki => (STUCKI, 42),
            Self::Burkes => (BURKES, 32),
            Self::Sierra => (SIERRA, 32),
            Self::TwoRowSierra => (TWO_ROW_SIERRA, 16),
            Self::SierraLite => (SIERRA_LITE, 4),
            Self::FalseFloydSteinberg => (FALSE_FLOYD_STEINBERG, 8),
            Self::Fan => (FAN, 16),
            Self::ShiauFan => (SHIAU_FAN, 8),
            Self::ShiauFan2 => (SHIAU_FAN_2, 16),
            Self::StevensonArce => (STEVENSON_ARCE, 200),
            Self::TwoDimensionalKnuth => (TWO_DIMENSIONAL_KNUTH, 2),
        }
    }
}

/// A validated set of weights used to distribute quantisation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffusionKernel {
    taps: Box<[DiffusionTap]>,
    divisor: u32,
    preset: Option<DiffusionAlgorithm>,
}

impl DiffusionKernel {
    /// Creates a custom diffusion kernel.
    ///
    /// Taps on the current row must have a positive horizontal offset. Taps on
    /// later rows may use any horizontal offset. All weights and the divisor
    /// must be greater than zero.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when the kernel has no taps, its
    /// divisor or a weight is zero, or a tap does not point forward in a raster
    /// scan.
    pub fn new(taps: impl Into<Box<[DiffusionTap]>>, divisor: u32) -> Result<Self> {
        let taps = taps.into();
        if taps.is_empty() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "a diffusion kernel requires at least one tap",
            ));
        }
        if divisor == 0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "a diffusion kernel divisor must be greater than zero",
            ));
        }
        if taps.iter().any(|tap| tap.weight == 0) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "diffusion tap weights must be greater than zero",
            ));
        }
        if taps
            .iter()
            .any(|tap| tap.offset_y < 0 || (tap.offset_y == 0 && tap.offset_x <= 0))
        {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "diffusion taps must point forward in a raster scan",
            ));
        }

        Ok(Self {
            taps,
            divisor,
            preset: None,
        })
    }

    /// Returns the weighted destinations in the kernel.
    pub fn taps(&self) -> &[DiffusionTap] {
        &self.taps
    }

    /// Returns the divisor applied to the tap weights.
    pub const fn divisor(&self) -> u32 {
        self.divisor
    }

    /// Returns the built-in preset represented by this kernel, if any.
    pub const fn preset(&self) -> Option<DiffusionAlgorithm> {
        self.preset
    }
}

impl From<DiffusionAlgorithm> for DiffusionKernel {
    fn from(algorithm: DiffusionAlgorithm) -> Self {
        algorithm.kernel()
    }
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
    kernel: DiffusionKernel,
    scan: DiffusionScan,
    pixel_size: u32,
    strength: u16,
    error_clamp: Option<u8>,
}

impl ErrorDiffusion {
    /// Creates error diffusion using a built-in algorithm or custom kernel.
    pub fn new(palette: Palette, kernel: impl Into<DiffusionKernel>) -> Self {
        Self {
            palette,
            kernel: kernel.into(),
            scan: DiffusionScan::Raster,
            pixel_size: 1,
            strength: DIFFUSION_STRENGTH_SCALE,
            error_clamp: None,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the diffusion kernel.
    pub const fn kernel(&self) -> &DiffusionKernel {
        &self.kernel
    }

    /// Returns the built-in diffusion preset, if one was supplied.
    pub const fn algorithm(&self) -> Option<DiffusionAlgorithm> {
        self.kernel.preset()
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

    /// Sets the proportion of quantisation error distributed to later pixels.
    ///
    /// Strength is rounded to the nearest 1/256. A strength of zero disables
    /// diffusion, one uses the kernel weights unchanged, and two doubles the
    /// distributed error.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] unless `strength` is finite and
    /// between zero and two inclusive.
    pub fn with_strength(mut self, strength: f32) -> Result<Self> {
        if !strength.is_finite() || !(0.0..=2.0).contains(&strength) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "diffusion strength must be between zero and two",
            ));
        }

        self.strength = (strength * f32::from(DIFFUSION_STRENGTH_SCALE)).round() as u16;
        Ok(self)
    }

    /// Returns the proportion of quantisation error distributed to later pixels.
    pub fn strength(&self) -> f32 {
        f32::from(self.strength) / f32::from(DIFFUSION_STRENGTH_SCALE)
    }

    /// Limits each channel's distributed error to `maximum` byte levels.
    pub const fn with_error_clamp(mut self, maximum: u8) -> Self {
        self.error_clamp = Some(maximum);
        self
    }

    /// Removes a previously configured error limit.
    pub const fn without_error_clamp(mut self) -> Self {
        self.error_clamp = None;
        self
    }

    /// Returns the per-channel error limit in byte levels, if configured.
    pub const fn error_clamp(&self) -> Option<u8> {
        self.error_clamp
    }

    fn parameters<'a>(
        &'a self,
        taps: &'a [DiffusionTap],
        custom_divisor: u32,
    ) -> DiffusionParameters<'a> {
        DiffusionParameters {
            palette: &self.palette,
            taps,
            pixel_size: self.pixel_size,
            strength: self.strength,
            error_clamp: self.error_clamp,
            custom_divisor,
        }
    }

    fn apply_scan<const SERPENTINE: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) {
        if self.strength == DIFFUSION_STRENGTH_SCALE
            && self.error_clamp.is_none()
            && self.kernel.preset().is_some()
        {
            self.apply_kernel::<SERPENTINE, true>(input, output, dimensions, mask);
        } else {
            self.apply_kernel::<SERPENTINE, false>(input, output, dimensions, mask);
        }
    }

    fn apply_kernel<const SERPENTINE: bool, const DEFAULT: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
        dimensions: (u32, u32),
        mask: &Mask,
    ) {
        match self.kernel.preset() {
            Some(DiffusionAlgorithm::FloydSteinberg) => diffuse_error::<16, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(FLOYD_STEINBERG, 0),
            ),
            Some(DiffusionAlgorithm::Atkinson) => diffuse_error::<8, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(ATKINSON, 0),
            ),
            Some(DiffusionAlgorithm::JarvisJudiceNinke) => {
                diffuse_error::<48, SERPENTINE, DEFAULT>(
                    input,
                    output,
                    dimensions,
                    mask,
                    self.parameters(JARVIS_JUDICE_NINKE, 0),
                )
            }
            Some(DiffusionAlgorithm::Stucki) => diffuse_error::<42, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(STUCKI, 0),
            ),
            Some(DiffusionAlgorithm::Burkes) => diffuse_error::<32, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(BURKES, 0),
            ),
            Some(DiffusionAlgorithm::Sierra) => diffuse_error::<32, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(SIERRA, 0),
            ),
            Some(DiffusionAlgorithm::TwoRowSierra) => diffuse_error::<16, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(TWO_ROW_SIERRA, 0),
            ),
            Some(DiffusionAlgorithm::SierraLite) => diffuse_error::<4, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(SIERRA_LITE, 0),
            ),
            Some(DiffusionAlgorithm::FalseFloydSteinberg) => {
                diffuse_error::<8, SERPENTINE, DEFAULT>(
                    input,
                    output,
                    dimensions,
                    mask,
                    self.parameters(FALSE_FLOYD_STEINBERG, 0),
                )
            }
            Some(DiffusionAlgorithm::Fan) => diffuse_error::<16, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(FAN, 0),
            ),
            Some(DiffusionAlgorithm::ShiauFan) => diffuse_error::<8, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(SHIAU_FAN, 0),
            ),
            Some(DiffusionAlgorithm::ShiauFan2) => diffuse_error::<16, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(SHIAU_FAN_2, 0),
            ),
            Some(DiffusionAlgorithm::StevensonArce) => diffuse_error::<200, SERPENTINE, DEFAULT>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(STEVENSON_ARCE, 0),
            ),
            Some(DiffusionAlgorithm::TwoDimensionalKnuth) => {
                diffuse_error::<2, SERPENTINE, DEFAULT>(
                    input,
                    output,
                    dimensions,
                    mask,
                    self.parameters(TWO_DIMENSIONAL_KNUTH, 0),
                )
            }
            None => diffuse_error::<0, SERPENTINE, false>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(self.kernel.taps(), self.kernel.divisor),
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

struct DiffusionParameters<'a> {
    palette: &'a Palette,
    taps: &'a [DiffusionTap],
    pixel_size: u32,
    strength: u16,
    error_clamp: Option<u8>,
    custom_divisor: u32,
}

/// Quantises selected logical cells and distributes errors to later cells.
fn diffuse_error<const DIVISOR: i32, const SERPENTINE: bool, const DEFAULT: bool>(
    input: &[u8],
    output: &mut [u8],
    dimensions: (u32, u32),
    mask: &Mask,
    parameters: DiffusionParameters<'_>,
) {
    let DiffusionParameters {
        palette,
        taps,
        pixel_size,
        strength,
        error_clamp,
        custom_divisor,
    } = parameters;
    let Some((min_x, min_y, max_x, max_y)) = mask.coverage_bounds() else {
        return;
    };
    let divisor = if DIVISOR == 0 {
        i64::from(custom_divisor)
    } else {
        i64::from(DIVISOR)
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
            let error = if DEFAULT {
                error
            } else if let Some(maximum) = error_clamp {
                let maximum = i32::from(maximum) * FIXED_SCALE;
                error.map(|channel| channel.clamp(-maximum, maximum))
            } else {
                error
            };

            for tap in taps {
                let offset_x = i64::from(tap.offset_x);
                let offset_x = if reverse { -offset_x } else { offset_x };
                let offset_y = i64::from(tap.offset_y);
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
                    if DEFAULT {
                        working[neighbour_index][channel] +=
                            error[channel] * tap.weight as i32 / DIVISOR;
                    } else {
                        let contribution =
                            i64::from(error[channel]) * i64::from(tap.weight) * i64::from(strength)
                                / (divisor * i64::from(DIFFUSION_STRENGTH_SCALE));
                        let contribution =
                            contribution.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
                        working[neighbour_index][channel] =
                            working[neighbour_index][channel].saturating_add(contribution as i32);
                    }
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
        Color, Colour, DiffusionAlgorithm, DiffusionKernel, DiffusionScan, DiffusionTap,
        ErrorDiffusion, OrderedDither, Palette, Threshold, ThresholdMap, ThresholdRotation,
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
    fn renders_exact_pixels_for_every_diffusion_preset() {
        let pixels = (0..35)
            .map(|index| {
                let value = ((index * 47 + index * index * 3) % 256) as u8;
                [value, value.wrapping_add(53), value.wrapping_mul(3), 200]
            })
            .collect::<Vec<_>>();
        let source = source(7, 5, &pixels);
        // These maps were recorded after checking each preset's taps and divisor.
        // `#` is black, `.` is white, and every source alpha is 200.
        let cases = [
            (
                DiffusionAlgorithm::FloydSteinberg,
                ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
            ),
            (
                DiffusionAlgorithm::Atkinson,
                ["###..#.", "...##.#", "##..#.#", "#.##..#", ".##.##."],
            ),
            (
                DiffusionAlgorithm::JarvisJudiceNinke,
                ["###..#.", ".#.##.#", ".#..#.#", "#.##...", ".##.##."],
            ),
            (
                DiffusionAlgorithm::Stucki,
                ["##...#.", ".#.##.#", "##..#.#", "#.##.#.", ".##.##."],
            ),
            (
                DiffusionAlgorithm::Burkes,
                ["##..#..", ".#.##.#", "#..##.#", "#.##...", ".##.##."],
            ),
            (
                DiffusionAlgorithm::Sierra,
                ["###..#.", "...##.#", "##..#.#", "#.##..#", ".##.##."],
            ),
            (
                DiffusionAlgorithm::TwoRowSierra,
                ["##..#..", ".#.##.#", "##..#.#", "#.##..#", ".##..#."],
            ),
            (
                DiffusionAlgorithm::SierraLite,
                ["##..#.#", ".#.#.#.", ".#.##.#", "#.#.#..", ".##..#."],
            ),
            (
                DiffusionAlgorithm::FalseFloydSteinberg,
                ["##..#.#", ".#.##.#", ".#..#.#", "####...", "..#.##."],
            ),
            (
                DiffusionAlgorithm::Fan,
                ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
            ),
            (
                DiffusionAlgorithm::ShiauFan,
                ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##.#.#"],
            ),
            (
                DiffusionAlgorithm::ShiauFan2,
                ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
            ),
            (
                DiffusionAlgorithm::StevensonArce,
                ["###..#.", ".#.##.#", ".#..#.#", "#.##...", ".##.##."],
            ),
            (
                DiffusionAlgorithm::TwoDimensionalKnuth,
                ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
            ),
        ];

        for (algorithm, rows) in cases {
            let rendered = Renderer::new()
                .render(&source, &diffusion(algorithm), &Selection::All)
                .unwrap();
            let expected = rows
                .concat()
                .bytes()
                .flat_map(|pixel| match pixel {
                    b'#' => [0, 0, 0, 200],
                    b'.' => [255, 255, 255, 200],
                    _ => unreachable!("pixel maps contain only `#` and `.`"),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                rendered.rgba8_bytes(),
                expected,
                "changed output for {algorithm:?}"
            );
        }
    }

    #[test]
    fn exposes_verified_0_4_3_diffusion_weights() {
        let specifications: &[(DiffusionAlgorithm, &[DiffusionTap], u32)] = &[
            (
                DiffusionAlgorithm::FalseFloydSteinberg,
                &[
                    DiffusionTap::new(1, 0, 3),
                    DiffusionTap::new(-1, 1, 3),
                    DiffusionTap::new(0, 1, 2),
                ],
                8,
            ),
            (
                DiffusionAlgorithm::Fan,
                &[
                    DiffusionTap::new(1, 0, 7),
                    DiffusionTap::new(-2, 1, 1),
                    DiffusionTap::new(-1, 1, 3),
                    DiffusionTap::new(0, 1, 5),
                ],
                16,
            ),
            (
                DiffusionAlgorithm::ShiauFan,
                &[
                    DiffusionTap::new(1, 0, 4),
                    DiffusionTap::new(-2, 1, 1),
                    DiffusionTap::new(-1, 1, 1),
                    DiffusionTap::new(0, 1, 2),
                ],
                8,
            ),
            (
                DiffusionAlgorithm::ShiauFan2,
                &[
                    DiffusionTap::new(1, 0, 8),
                    DiffusionTap::new(-3, 1, 1),
                    DiffusionTap::new(-2, 1, 1),
                    DiffusionTap::new(-1, 1, 2),
                    DiffusionTap::new(0, 1, 4),
                ],
                16,
            ),
            (
                DiffusionAlgorithm::StevensonArce,
                &[
                    DiffusionTap::new(2, 0, 32),
                    DiffusionTap::new(-3, 1, 12),
                    DiffusionTap::new(-1, 1, 26),
                    DiffusionTap::new(1, 1, 30),
                    DiffusionTap::new(3, 1, 16),
                    DiffusionTap::new(-2, 2, 12),
                    DiffusionTap::new(0, 2, 26),
                    DiffusionTap::new(2, 2, 12),
                    DiffusionTap::new(-3, 3, 5),
                    DiffusionTap::new(-1, 3, 12),
                    DiffusionTap::new(1, 3, 12),
                    DiffusionTap::new(3, 3, 5),
                ],
                200,
            ),
            (
                DiffusionAlgorithm::TwoDimensionalKnuth,
                &[DiffusionTap::new(1, 0, 1), DiffusionTap::new(0, 1, 1)],
                2,
            ),
        ];

        for (algorithm, taps, divisor) in specifications {
            let kernel = algorithm.kernel();
            assert_eq!(kernel.taps(), *taps, "wrong weights for {algorithm:?}");
            assert_eq!(
                kernel.divisor(),
                *divisor,
                "wrong divisor for {algorithm:?}"
            );
        }
    }

    #[test]
    fn validates_and_exposes_custom_diffusion_kernels() {
        let taps = [
            DiffusionTap::new(1, 0, 7),
            DiffusionTap::new(-1, 1, 3),
            DiffusionTap::new(0, 1, 5),
            DiffusionTap::new(1, 1, 1),
        ];
        let kernel = DiffusionKernel::new(taps, 16).unwrap();
        assert_eq!(kernel.taps(), taps);
        assert_eq!(kernel.divisor(), 16);
        assert_eq!(kernel.preset(), None);
        assert_eq!(kernel.taps()[0].offset_x(), 1);
        assert_eq!(kernel.taps()[0].offset_y(), 0);
        assert_eq!(kernel.taps()[0].weight(), 7);

        let preset = DiffusionAlgorithm::FloydSteinberg.kernel();
        assert_eq!(preset.taps(), taps);
        assert_eq!(preset.divisor(), 16);
        assert_eq!(preset.preset(), Some(DiffusionAlgorithm::FloydSteinberg));

        let invalid = [
            DiffusionKernel::new(Vec::<DiffusionTap>::new(), 1).unwrap_err(),
            DiffusionKernel::new([DiffusionTap::new(1, 0, 1)], 0).unwrap_err(),
            DiffusionKernel::new([DiffusionTap::new(1, 0, 0)], 1).unwrap_err(),
            DiffusionKernel::new([DiffusionTap::new(0, 0, 1)], 1).unwrap_err(),
            DiffusionKernel::new([DiffusionTap::new(-1, 0, 1)], 1).unwrap_err(),
            DiffusionKernel::new([DiffusionTap::new(0, -1, 1)], 1).unwrap_err(),
        ];
        assert!(
            invalid
                .iter()
                .all(|error| error.kind() == ErrorKind::InvalidParameter)
        );
    }

    #[test]
    fn custom_kernel_matches_its_builtin_equivalent() {
        let source = source(
            6,
            2,
            &[
                [20, 40, 60, 1],
                [70, 90, 110, 2],
                [120, 140, 160, 3],
                [170, 190, 210, 4],
                [220, 240, 250, 5],
                [100, 130, 160, 6],
                [210, 180, 150, 7],
                [160, 130, 100, 8],
                [110, 80, 50, 9],
                [60, 30, 10, 10],
                [130, 170, 210, 11],
                [230, 190, 150, 12],
            ],
        );
        let custom = DiffusionKernel::new(
            [
                DiffusionTap::new(1, 0, 7),
                DiffusionTap::new(-1, 1, 3),
                DiffusionTap::new(0, 1, 5),
                DiffusionTap::new(1, 1, 1),
            ],
            16,
        )
        .unwrap();
        for scan in [DiffusionScan::Raster, DiffusionScan::Serpentine] {
            let builtin = Renderer::new()
                .render(
                    &source,
                    &diffusion(DiffusionAlgorithm::FloydSteinberg).with_scan(scan),
                    &Selection::All,
                )
                .unwrap();
            let rendered = Renderer::new()
                .render(
                    &source,
                    &ErrorDiffusion::new(Palette::black_and_white(), custom.clone())
                        .with_scan(scan),
                    &Selection::All,
                )
                .unwrap();

            assert_eq!(rendered.rgba8_bytes(), builtin.rgba8_bytes());
        }
    }

    #[test]
    fn configures_diffusion_strength_and_error_clamping() {
        let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
        let threshold = Renderer::new()
            .render(
                &source,
                &Threshold::new(Palette::black_and_white()),
                &Selection::All,
            )
            .unwrap();
        let no_diffusion = diffusion(DiffusionAlgorithm::FloydSteinberg)
            .with_strength(0.0)
            .unwrap();
        let no_error = diffusion(DiffusionAlgorithm::FloydSteinberg).with_error_clamp(0);

        for effect in [&no_diffusion, &no_error] {
            let rendered = Renderer::new()
                .render(&source, effect, &Selection::All)
                .unwrap();
            assert_eq!(rendered.rgba8_bytes(), threshold.rgba8_bytes());
        }

        let configured = diffusion(DiffusionAlgorithm::Stucki)
            .with_strength(1.5)
            .unwrap()
            .with_error_clamp(32);
        assert_eq!(configured.strength(), 1.5);
        assert_eq!(configured.error_clamp(), Some(32));
        assert_eq!(configured.clone().without_error_clamp().error_clamp(), None);
        assert_eq!(diffusion(DiffusionAlgorithm::Stucki).strength(), 1.0);
        assert_eq!(diffusion(DiffusionAlgorithm::Stucki).error_clamp(), None);

        for strength in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
            assert_eq!(
                diffusion(DiffusionAlgorithm::Stucki)
                    .with_strength(strength)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidParameter
            );
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
    fn provides_colour_values_and_prefab_palettes() {
        let colour = Colour::new(12, 34, 56);
        let alias: Color = colour;
        assert_eq!(alias.rgb(), [12, 34, 56]);
        assert_eq!(Colour::from([12, 34, 56]), colour);
        assert_eq!(<[u8; 3]>::from(colour), [12, 34, 56]);

        let custom = Palette::new([Colour::BLACK, colour, Colour::WHITE]).unwrap();
        assert_eq!(
            custom.colours(),
            &[[0, 0, 0], [12, 34, 56], [255, 255, 255]]
        );
        assert_eq!(
            custom.iter().collect::<Vec<_>>(),
            [Colour::BLACK, colour, Colour::WHITE]
        );
        assert_eq!(custom.nearest(Colour::new(10, 30, 50)), colour);

        assert_eq!(
            Palette::greyscale().colours(),
            &[
                [0, 0, 0],
                [36, 36, 36],
                [73, 73, 73],
                [109, 109, 109],
                [146, 146, 146],
                [182, 182, 182],
                [219, 219, 219],
                [255, 255, 255],
            ]
        );
        assert_eq!(Palette::game_boy().colours().len(), 4);
        assert_eq!(Palette::cga().colours().len(), 16);
        assert_eq!(Palette::pico_8().colours().len(), 16);
        assert_eq!(Palette::cga().colours()[6], [170, 85, 0]);
        assert_eq!(Palette::pico_8().colours()[8], [255, 0, 77]);
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
    fn validates_custom_threshold_maps() {
        let map = ThresholdMap::new(3, 2, [0, 3, 1, 4, 2, 5]).unwrap();
        assert_eq!(map.width(), 3);
        assert_eq!(map.height(), 2);
        assert_eq!(map.thresholds(), &[0, 3, 1, 4, 2, 5]);

        for result in [
            ThresholdMap::new(0, 2, Box::<[u32]>::default()),
            ThresholdMap::new(2, 0, Box::<[u32]>::default()),
            ThresholdMap::new(2, 2, [0, 1, 2].as_slice()),
            ThresholdMap::new(2, 2, [0, 1, 2, 4].as_slice()),
        ] {
            assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidParameter);
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

        for (map, expected) in [
            (ThresholdMap::bayer_2x2(), expected[0].1.clone()),
            (ThresholdMap::bayer_4x4(), expected[1].1.clone()),
            (ThresholdMap::bayer_8x8(), expected[2].1.clone()),
        ] {
            assert_eq!(map.width(), map.height());
            assert_eq!(map.thresholds(), expected);
        }
    }

    #[test]
    fn artistic_threshold_maps_are_distinct_and_tile_after_transformations() {
        let maps = [
            ThresholdMap::clustered_dots(),
            ThresholdMap::horizontal_lines(),
            ThresholdMap::vertical_lines(),
            ThresholdMap::diagonal_lines(),
            ThresholdMap::crosshatch(),
            ThresholdMap::checkerboard(),
            ThresholdMap::dispersed_dots_3x3(),
            ThresholdMap::dispersed_dots_5x5(),
        ];
        for (index, map) in maps.iter().enumerate() {
            assert!(maps[..index].iter().all(|other| other != map));

            for rotation in [
                ThresholdRotation::None,
                ThresholdRotation::Clockwise90,
                ThresholdRotation::Clockwise180,
                ThresholdRotation::Clockwise270,
            ] {
                for (mirror_x, mirror_y) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    let effect = OrderedDither::new(Palette::black_and_white(), map.clone())
                        .with_offset(-7, 11)
                        .with_rotation(rotation)
                        .with_mirroring(mirror_x, mirror_y);
                    let (width, height) = effect.transformed_dimensions();

                    for y in 0..height * 2 {
                        for x in 0..width * 2 {
                            assert_eq!(effect.threshold(x, y), effect.threshold(x + width, y));
                            assert_eq!(effect.threshold(x, y), effect.threshold(x, y + height));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn renders_exact_pixels_for_each_bayer_size() {
        for map in [
            ThresholdMap::bayer_2x2(),
            ThresholdMap::bayer_4x4(),
            ThresholdMap::bayer_8x8(),
        ] {
            let size = map.width();
            let source = source(size, 1, &vec![[128, 128, 128, 90]; size as usize]);
            let effect = OrderedDither::new(Palette::black_and_white(), map);
            let rendered = Renderer::new()
                .render(&source, &effect, &Selection::All)
                .unwrap();

            for x in 0..size {
                let value = if x % 2 == 0 { 0 } else { 255 };
                assert_eq!(rendered.pixel(x, 0), Some([value, value, value, 90]));
            }
        }
    }

    #[test]
    fn transforms_rectangular_threshold_maps() {
        let map = ThresholdMap::new(3, 2, [0, 1, 2, 3, 4, 5]).unwrap();
        let effect = OrderedDither::new(Palette::black_and_white(), map);

        assert_eq!(
            (0..6).map(|x| effect.threshold(x, 0)).collect::<Vec<_>>(),
            [0, 1, 2, 0, 1, 2]
        );
        let rotated = effect.clone().with_rotation(ThresholdRotation::Clockwise90);
        assert_eq!(
            (0..4).map(|x| rotated.threshold(x, 0)).collect::<Vec<_>>(),
            [3, 0, 3, 0]
        );
        let transformed = effect.with_offset(1, -1).with_mirroring(true, true);
        assert_eq!(
            (0..3)
                .map(|x| transformed.threshold(x, 0))
                .collect::<Vec<_>>(),
            [0, 2, 1]
        );
    }

    #[test]
    fn renders_exact_pixels_for_a_custom_rectangular_map() {
        let source = source(3, 2, &[[128, 128, 128, 90]; 6]);
        let map = ThresholdMap::new(3, 2, [0, 1, 2, 3, 4, 5]).unwrap();
        let effect = OrderedDither::new(Palette::black_and_white(), map);
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(
            rendered.rgba8_bytes(),
            &[
                0, 0, 0, 90, 0, 0, 0, 90, 0, 0, 0, 90, 255, 255, 255, 90, 255, 255, 255, 90, 255,
                255, 255, 90,
            ]
        );
    }

    #[test]
    fn configures_threshold_strength_and_transformations() {
        let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
            .with_strength(0.5)
            .unwrap()
            .with_offset(-2, 3)
            .with_rotation(ThresholdRotation::Clockwise270)
            .with_mirroring(true, false);
        assert_eq!(effect.strength(), 0.5);
        assert_eq!(effect.offset(), (-2, 3));
        assert_eq!(effect.rotation(), ThresholdRotation::Clockwise270);
        assert!(effect.mirror_x());
        assert!(!effect.mirror_y());

        for strength in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
            assert_eq!(
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
                    .with_strength(strength)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidParameter
            );
        }
    }

    #[test]
    fn keeps_polygon_patterns_anchored_to_image_coordinates() {
        let source = source(4, 1, &[[128, 128, 128, 80]; 4]);
        let effect = OrderedDither::new(
            Palette::black_and_white(),
            ThresholdMap::new(4, 1, [0, 3, 1, 2]).unwrap(),
        );
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
            ThresholdMap::bayer_4x4(),
        );
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
            OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
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
        let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
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
        let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
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
            DiffusionAlgorithm::FalseFloydSteinberg,
            DiffusionAlgorithm::Fan,
            DiffusionAlgorithm::ShiauFan,
            DiffusionAlgorithm::ShiauFan2,
            DiffusionAlgorithm::StevensonArce,
            DiffusionAlgorithm::TwoDimensionalKnuth,
        ];

        for algorithm in algorithms {
            let effect = ErrorDiffusion::new(Palette::black_and_white(), algorithm)
                .with_scan(DiffusionScan::Serpentine)
                .with_pixel_size(2)
                .unwrap();
            let rendered = Renderer::new()
                .render(&source, &effect, &Selection::All)
                .unwrap();

            assert_eq!(effect.algorithm(), Some(algorithm));
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
