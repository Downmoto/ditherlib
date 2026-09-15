use super::cells::{quantise_cells, validate_pixel_size};
use super::palette::Palette;
use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const THRESHOLD_STRENGTH_SCALE: u16 = 256;
pub(super) const BLUE_NOISE_SIZE: u32 = 16;
pub(super) const BLUE_NOISE_16X16: [u32; 256] = [
    171, 57, 199, 110, 146, 212, 93, 27, 181, 143, 219, 188, 63, 154, 86, 191, 114, 28, 221, 70,
    175, 42, 123, 234, 113, 76, 25, 163, 5, 226, 137, 18, 94, 161, 136, 8, 254, 77, 196, 3, 206,
    46, 247, 121, 79, 208, 43, 249, 201, 231, 53, 190, 100, 165, 139, 62, 150, 177, 89, 198, 36,
    107, 176, 69, 127, 33, 87, 151, 19, 220, 34, 241, 96, 215, 10, 138, 167, 240, 147, 11, 184,
    111, 245, 211, 119, 54, 108, 173, 26, 117, 65, 232, 24, 98, 49, 218, 157, 67, 1, 170, 75, 237,
    193, 134, 72, 251, 155, 203, 58, 120, 192, 83, 38, 135, 200, 48, 142, 23, 88, 223, 4, 183, 40,
    84, 179, 227, 6, 252, 174, 236, 97, 222, 105, 186, 153, 51, 210, 104, 141, 16, 129, 152, 71,
    103, 31, 61, 17, 159, 41, 253, 15, 118, 164, 229, 91, 246, 214, 44, 115, 205, 140, 217, 122,
    194, 68, 132, 204, 80, 59, 32, 189, 64, 168, 22, 233, 158, 92, 178, 78, 244, 29, 169, 99, 235,
    180, 131, 7, 124, 106, 197, 81, 14, 238, 47, 9, 109, 149, 216, 2, 45, 148, 243, 209, 74, 39,
    255, 56, 187, 125, 207, 162, 224, 85, 66, 195, 116, 20, 82, 160, 225, 145, 128, 166, 102, 73,
    35, 133, 185, 50, 130, 228, 172, 101, 202, 55, 13, 182, 30, 213, 0, 144, 248, 90, 21, 239, 12,
    156, 60, 250, 37, 126, 95, 242, 112, 52, 230,
];

/// The noise distribution used by [`NoiseDither`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum NoiseAlgorithm {
    /// Independent deterministic samples at each image coordinate.
    #[default]
    White,
    /// A seeded variant of the built-in 16x16 blue-noise threshold map.
    Blue,
}

/// Applies deterministic noise threshold dithering.
///
/// Samples are derived from image-origin logical pixel coordinates and the
/// seed. Traversal order and selection bounds therefore do not shift the
/// surrounding noise field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoiseDither {
    palette: Palette,
    algorithm: NoiseAlgorithm,
    pixel_size: u32,
    seed: u64,
    strength: u16,
}

impl NoiseDither {
    /// Creates noise dithering with a zero seed and full noise strength.
    pub const fn new(palette: Palette, algorithm: NoiseAlgorithm) -> Self {
        Self {
            palette,
            algorithm,
            pixel_size: 1,
            seed: 0,
            strength: THRESHOLD_STRENGTH_SCALE,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the noise algorithm.
    pub const fn algorithm(&self) -> NoiseAlgorithm {
        self.algorithm
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

    /// Sets the deterministic noise seed.
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Returns the deterministic noise seed.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Sets the noise threshold adjustment strength.
    ///
    /// Strength is rounded to the nearest 1/256. Zero disables noise, one uses
    /// it unchanged, and two doubles it.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] unless `strength` is finite and
    /// between zero and two inclusive.
    pub fn with_strength(mut self, strength: f32) -> Result<Self> {
        if !strength.is_finite() || !(0.0..=2.0).contains(&strength) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "noise strength must be between zero and two",
            ));
        }
        self.strength = (strength * f32::from(THRESHOLD_STRENGTH_SCALE)).round() as u16;
        Ok(self)
    }

    /// Returns the noise threshold adjustment strength.
    pub fn strength(&self) -> f32 {
        f32::from(self.strength) / f32::from(THRESHOLD_STRENGTH_SCALE)
    }
}

impl Effect for NoiseDither {
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
            |colour, x, y| {
                let threshold = match self.algorithm {
                    NoiseAlgorithm::White => white_noise(x, y, self.seed),
                    NoiseAlgorithm::Blue => blue_noise(x, y, self.seed),
                };
                let adjustment = (i32::from(threshold) * 2 + 1 - 256) * 255 / 512;
                let adjustment =
                    adjustment * i32::from(self.strength) / i32::from(THRESHOLD_STRENGTH_SCALE);
                let colour =
                    colour.map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8);
                self.palette.nearest_colour(colour)
            },
        );
        Ok(())
    }
}

/// A weighted destination in an error-diffusion kernel.
fn white_noise(x: u32, y: u32, seed: u64) -> u8 {
    let value = seed
        ^ u64::from(x).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(y).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    (mix64(value) >> 56) as u8
}

/// Samples seeded per-tile variants of the built-in blue-noise map.
pub(super) fn blue_noise(x: u32, y: u32, seed: u64) -> u8 {
    let tile_x = x / BLUE_NOISE_SIZE;
    let tile_y = y / BLUE_NOISE_SIZE;
    let variation = mix64(
        seed ^ u64::from(tile_x).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ u64::from(tile_y).wrapping_mul(0xbf58_476d_1ce4_e5b9),
    );
    let mut x = (x % BLUE_NOISE_SIZE).wrapping_add(variation as u32) % BLUE_NOISE_SIZE;
    let mut y = (y % BLUE_NOISE_SIZE).wrapping_add((variation >> 32) as u32) % BLUE_NOISE_SIZE;
    if variation & (1 << 8) != 0 {
        std::mem::swap(&mut x, &mut y);
    }
    if variation & (1 << 9) != 0 {
        x = BLUE_NOISE_SIZE - x - 1;
    }
    if variation & (1 << 10) != 0 {
        y = BLUE_NOISE_SIZE - y - 1;
    }
    BLUE_NOISE_16X16[(y * BLUE_NOISE_SIZE + x) as usize] as u8
}

/// Applies the SplitMix64 finaliser without retaining mutable random state.
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
