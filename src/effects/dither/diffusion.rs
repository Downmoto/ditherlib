use super::cells::{align_to_grid, cell_bounds, sample_cell, validate_pixel_size, write_cell};
use super::colour::{ColourSpace, clamp_components, colour_components, colour_luminance};
use super::palette::{Palette, PaletteMatchMode};
use crate::{DitherError, Effect, ErrorKind, Mask, Result};

const FIXED_SCALE: i32 = 256;
const DIFFUSION_STRENGTH_SCALE: u16 = 256;

/// The components through which quantisation error is diffused.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiffusionErrorMode {
    /// Diffuses each component independently in the selected colour space.
    #[default]
    IndependentChannels,
    /// Diffuses luminance error without propagating chroma error.
    Luminance,
}

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
    error_mode: DiffusionErrorMode,
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
            error_mode: DiffusionErrorMode::IndependentChannels,
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

    /// Selects independent-component or luminance-only error diffusion.
    pub const fn with_error_mode(mut self, error_mode: DiffusionErrorMode) -> Self {
        self.error_mode = error_mode;
        self
    }

    /// Returns the quantisation error mode.
    pub const fn error_mode(&self) -> DiffusionErrorMode {
        self.error_mode
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
            error_mode: self.error_mode,
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
        if self.palette.colour_space != ColourSpace::Rgb
            || self.palette.matching_mode != PaletteMatchMode::Colour
            || self.error_mode != DiffusionErrorMode::IndependentChannels
        {
            diffuse_error_transformed::<SERPENTINE>(
                input,
                output,
                dimensions,
                mask,
                self.parameters(self.kernel.taps(), self.kernel.divisor()),
            );
            return;
        }
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
struct DiffusionParameters<'a> {
    palette: &'a Palette,
    taps: &'a [DiffusionTap],
    pixel_size: u32,
    strength: u16,
    error_clamp: Option<u8>,
    error_mode: DiffusionErrorMode,
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
        error_mode: _,
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

/// Diffuses quantisation errors in linear RGB or Oklab, or by luminance only.
fn diffuse_error_transformed<const SERPENTINE: bool>(
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
        error_mode,
        custom_divisor,
    } = parameters;
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
            let colour = sample_cell(
                input,
                mask,
                image_width,
                cell_bounds(x, y, pixel_size, dimensions),
            );
            selected.push(colour.is_some());
            working.push(colour_components(
                colour.unwrap_or([0; 3]),
                palette.colour_space,
            ));
        }
    }

    let divisor = custom_divisor as f32;
    let strength = f32::from(strength) / f32::from(DIFFUSION_STRENGTH_SCALE);
    let error_clamp = error_clamp.map(|maximum| f32::from(maximum) / 255.0);
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

            let adjusted = clamp_components(working[working_index], palette.colour_space);
            let colour = palette.nearest_transformed(adjusted);
            let matched = colour_components(colour, palette.colour_space);
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
            let mut error = match error_mode {
                DiffusionErrorMode::IndependentChannels => {
                    std::array::from_fn(|channel| adjusted[channel] - matched[channel])
                }
                DiffusionErrorMode::Luminance => {
                    let error = colour_luminance(adjusted, palette.colour_space)
                        - colour_luminance(matched, palette.colour_space);
                    match palette.colour_space {
                        ColourSpace::Rgb | ColourSpace::LinearRgb => [error; 3],
                        ColourSpace::Oklab => [error, 0.0, 0.0],
                    }
                }
            };
            if let Some(maximum) = error_clamp {
                error = error.map(|channel| channel.clamp(-maximum, maximum));
            }

            for tap in taps {
                let offset_x = if reverse {
                    -i64::from(tap.offset_x)
                } else {
                    i64::from(tap.offset_x)
                };
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
                let neighbour_index = neighbour_y as usize * working_width + neighbour_x as usize;
                if !selected[neighbour_index]
                    || !diffusion_path_is_selected(
                        &selected,
                        working_width,
                        cell_x,
                        cell_y,
                        offset_x,
                        offset_y,
                    )
                {
                    continue;
                }
                let factor = tap.weight as f32 * strength / divisor;
                for (working, error) in working[neighbour_index].iter_mut().zip(error) {
                    *working += error * factor;
                }
            }
        }
    }
}

/// Returns the image-origin-aligned coordinate containing `coordinate`.
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
