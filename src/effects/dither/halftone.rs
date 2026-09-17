use super::palette::Palette;
use crate::{DitherError, Effect, ErrorKind, Mask, Result};

/// The dot geometry used by a [`Halftone`] screen.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum HalftoneShape {
    /// Circular dots.
    #[default]
    Circle,
    /// Square dots.
    Square,
    /// Diamond-shaped dots.
    Diamond,
    /// Elliptical dots that follow the cell aspect ratio.
    Ellipse,
    /// Parallel lines.
    Line,
    /// Perpendicular lines forming crosses.
    Cross,
}

/// Applies an image-origin-anchored halftone screen.
///
/// Screen angle is measured clockwise in radians. Phase is measured in image
/// pixels along the rotated screen axes. The palette may be black and white,
/// monochrome, or contain any number of colours.
#[derive(Clone, Debug, PartialEq)]
pub struct Halftone {
    palette: Palette,
    shape: HalftoneShape,
    cell_width: u32,
    cell_height: u32,
    angle: f32,
    phase: (f32, f32),
    scale: f32,
}

impl Halftone {
    /// Creates a halftone screen with 8 by 8 pixel cells.
    pub const fn new(palette: Palette, shape: HalftoneShape) -> Self {
        Self {
            palette,
            shape,
            cell_width: 8,
            cell_height: 8,
            angle: 0.0,
            phase: (0.0, 0.0),
            scale: 1.0,
        }
    }

    /// Returns the target palette.
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Returns the dot shape.
    pub const fn shape(&self) -> HalftoneShape {
        self.shape
    }

    /// Sets the cell width in pixels.
    pub fn with_cell_width(mut self, width: u32) -> Result<Self> {
        self.cell_width = validate_cell_dimension(width)?;
        Ok(self)
    }

    /// Returns the cell width in pixels.
    pub const fn cell_width(&self) -> u32 {
        self.cell_width
    }

    /// Sets the cell height in pixels.
    pub fn with_cell_height(mut self, height: u32) -> Result<Self> {
        self.cell_height = validate_cell_dimension(height)?;
        Ok(self)
    }

    /// Returns the cell height in pixels.
    pub const fn cell_height(&self) -> u32 {
        self.cell_height
    }

    /// Sets both cell dimensions in pixels.
    pub fn with_cell_size(mut self, width: u32, height: u32) -> Result<Self> {
        self.cell_width = validate_cell_dimension(width)?;
        self.cell_height = validate_cell_dimension(height)?;
        Ok(self)
    }

    /// Sets the clockwise screen angle in radians.
    pub fn with_angle(mut self, angle: f32) -> Result<Self> {
        if !angle.is_finite() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone angle must be finite",
            ));
        }
        self.angle = angle;
        Ok(self)
    }

    /// Returns the clockwise screen angle in radians.
    pub const fn angle(&self) -> f32 {
        self.angle
    }

    /// Sets the screen phase in pixels along its rotated axes.
    pub fn with_phase(mut self, x: f32, y: f32) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone phase must be finite",
            ));
        }
        self.phase = (x, y);
        Ok(self)
    }

    /// Returns the screen phase in pixels.
    pub const fn phase(&self) -> (f32, f32) {
        self.phase
    }

    /// Sets the dot scale.
    ///
    /// Values below one shrink dots and values above one grow them.
    pub fn with_scale(mut self, scale: f32) -> Result<Self> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone scale must be finite and greater than zero",
            ));
        }
        self.scale = scale;
        Ok(self)
    }

    /// Returns the dot scale.
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    fn threshold(&self, x: f32, y: f32) -> f32 {
        halftone_threshold(
            self.shape,
            self.cell_width,
            self.cell_height,
            self.angle,
            self.phase,
            self.scale,
            x,
            y,
        )
    }
}

impl Effect for Halftone {
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

        for y in min_y..max_y {
            for x in min_x..max_x {
                let pixel_index = y as usize * image_width + x as usize;
                if mask.coverage_bytes()[pixel_index] == 0 {
                    continue;
                }
                let byte_index = pixel_index * 4;
                let adjustment =
                    ((self.threshold(x as f32 + 0.5, y as f32 + 0.5) - 0.5) * 255.0).round() as i32;
                let colour = [
                    input[byte_index],
                    input[byte_index + 1],
                    input[byte_index + 2],
                ]
                .map(|channel| (i32::from(channel) + adjustment).clamp(0, 255) as u8);
                let colour = self.palette.nearest_colour(colour);
                output[byte_index..byte_index + 3].copy_from_slice(&colour);
            }
        }
        Ok(())
    }
}

fn validate_cell_dimension(dimension: u32) -> Result<u32> {
    if dimension == 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "halftone cell dimensions must be greater than zero",
        ));
    }
    Ok(dimension)
}

/// Channel separation used by [`ColourHalftone`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ColourHalftoneMode {
    /// Screens the additive red, green, and blue channels.
    #[default]
    Rgb,
    /// Converts to and screens subtractive cyan, magenta, yellow, and black.
    Cmyk,
}

/// A channel that can have an independent halftone angle and offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HalftoneChannel {
    /// Additive red.
    Red,
    /// Additive green.
    Green,
    /// Additive blue.
    Blue,
    /// Subtractive cyan.
    Cyan,
    /// Subtractive magenta.
    Magenta,
    /// Subtractive yellow.
    Yellow,
    /// Subtractive black.
    Black,
}

/// A standard set of CMYK screen angles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum CmykScreenPreset {
    /// Conventional 15° cyan, 75° magenta, 0° yellow, and 45° black screens.
    #[default]
    Traditional,
    /// Digital angles that reduce short-period moiré patterns.
    MoireResistant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ChannelScreen {
    angle: f32,
    offset: (f32, f32),
}

impl ChannelScreen {
    const fn new(angle: f32) -> Self {
        Self {
            angle,
            offset: (0.0, 0.0),
        }
    }
}

/// Applies independently transformed RGB or CMYK halftone screens.
///
/// RGB mode thresholds the three additive channels before direct
/// recombination. CMYK mode performs under-colour removal, thresholds four
/// subtractive ink channels, then recombines them into RGB. Every screen is
/// anchored to image coordinates, so output is deterministic across renders
/// and selections.
#[derive(Clone, Debug, PartialEq)]
pub struct ColourHalftone {
    mode: ColourHalftoneMode,
    shape: HalftoneShape,
    cell_width: u32,
    cell_height: u32,
    scale: f32,
    screens: [ChannelScreen; 4],
}

impl ColourHalftone {
    /// Creates a colour halftone with 8 by 8 pixel cells.
    pub const fn new(mode: ColourHalftoneMode, shape: HalftoneShape) -> Self {
        let screens = match mode {
            ColourHalftoneMode::Rgb => [
                ChannelScreen::new(0.261_799_4),
                ChannelScreen::new(1.308_996_9),
                ChannelScreen::new(0.0),
                ChannelScreen::new(0.0),
            ],
            ColourHalftoneMode::Cmyk => cmyk_screens(CmykScreenPreset::Traditional),
        };
        Self {
            mode,
            shape,
            cell_width: 8,
            cell_height: 8,
            scale: 1.0,
            screens,
        }
    }

    /// Returns the channel mode.
    pub const fn mode(&self) -> ColourHalftoneMode {
        self.mode
    }

    /// Returns the shared dot shape.
    pub const fn shape(&self) -> HalftoneShape {
        self.shape
    }

    /// Sets the cell width in pixels.
    pub fn with_cell_width(mut self, width: u32) -> Result<Self> {
        self.cell_width = validate_cell_dimension(width)?;
        Ok(self)
    }

    /// Returns the cell width in pixels.
    pub const fn cell_width(&self) -> u32 {
        self.cell_width
    }

    /// Sets the cell height in pixels.
    pub fn with_cell_height(mut self, height: u32) -> Result<Self> {
        self.cell_height = validate_cell_dimension(height)?;
        Ok(self)
    }

    /// Returns the cell height in pixels.
    pub const fn cell_height(&self) -> u32 {
        self.cell_height
    }

    /// Sets both cell dimensions in pixels.
    pub fn with_cell_size(mut self, width: u32, height: u32) -> Result<Self> {
        self.cell_width = validate_cell_dimension(width)?;
        self.cell_height = validate_cell_dimension(height)?;
        Ok(self)
    }

    /// Sets the shared dot scale.
    pub fn with_scale(mut self, scale: f32) -> Result<Self> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone scale must be finite and greater than zero",
            ));
        }
        self.scale = scale;
        Ok(self)
    }

    /// Returns the shared dot scale.
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// Applies a standard CMYK screen-angle preset.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] in RGB mode.
    pub fn with_cmyk_preset(mut self, preset: CmykScreenPreset) -> Result<Self> {
        if self.mode != ColourHalftoneMode::Cmyk {
            return Err(invalid_halftone_channel());
        }
        for (screen, preset) in self.screens.iter_mut().zip(cmyk_screens(preset)) {
            screen.angle = preset.angle;
        }
        Ok(self)
    }

    /// Sets one channel's clockwise angle in radians.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when the angle is non-finite or
    /// the channel does not belong to the selected mode.
    pub fn with_channel_angle(mut self, channel: HalftoneChannel, angle: f32) -> Result<Self> {
        if !angle.is_finite() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone channel angle must be finite",
            ));
        }
        let index = channel_index(self.mode, channel).ok_or_else(invalid_halftone_channel)?;
        self.screens[index].angle = angle;
        Ok(self)
    }

    /// Returns a channel's clockwise angle in radians, if it belongs to this mode.
    pub fn channel_angle(&self, channel: HalftoneChannel) -> Option<f32> {
        channel_index(self.mode, channel).map(|index| self.screens[index].angle)
    }

    /// Sets one channel's screen offset in pixels along its rotated axes.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when an offset is non-finite or
    /// the channel does not belong to the selected mode.
    pub fn with_channel_offset(mut self, channel: HalftoneChannel, x: f32, y: f32) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "halftone channel offset must be finite",
            ));
        }
        let index = channel_index(self.mode, channel).ok_or_else(invalid_halftone_channel)?;
        self.screens[index].offset = (x, y);
        Ok(self)
    }

    /// Returns a channel's screen offset, if it belongs to this mode.
    pub fn channel_offset(&self, channel: HalftoneChannel) -> Option<(f32, f32)> {
        channel_index(self.mode, channel).map(|index| self.screens[index].offset)
    }

    fn threshold(&self, screen: usize, x: f32, y: f32) -> u8 {
        let screen = self.screens[screen];
        ((halftone_threshold(
            self.shape,
            self.cell_width,
            self.cell_height,
            screen.angle,
            screen.offset,
            self.scale,
            x,
            y,
        ) * 254.0)
            .round() as u8)
            .saturating_add(1)
    }
}

impl Effect for ColourHalftone {
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

        for y in min_y..max_y {
            for x in min_x..max_x {
                let pixel_index = y as usize * image_width + x as usize;
                if mask.coverage_bytes()[pixel_index] == 0 {
                    continue;
                }
                let byte_index = pixel_index * 4;
                let source = [
                    input[byte_index],
                    input[byte_index + 1],
                    input[byte_index + 2],
                ];
                let x = x as f32 + 0.5;
                let y = y as f32 + 0.5;
                let colour = match self.mode {
                    ColourHalftoneMode::Rgb => [0, 1, 2].map(|channel| {
                        if source[channel] >= self.threshold(channel, x, y) {
                            255
                        } else {
                            0
                        }
                    }),
                    ColourHalftoneMode::Cmyk => {
                        let channels = rgb_to_cmyk(source);
                        let screened = std::array::from_fn(|channel| {
                            if channels[channel] >= self.threshold(channel, x, y) {
                                255
                            } else {
                                0
                            }
                        });
                        cmyk_to_rgb(screened)
                    }
                };
                output[byte_index..byte_index + 3].copy_from_slice(&colour);
            }
        }
        Ok(())
    }
}

const fn cmyk_screens(preset: CmykScreenPreset) -> [ChannelScreen; 4] {
    match preset {
        CmykScreenPreset::Traditional => [
            ChannelScreen::new(0.261_799_4),
            ChannelScreen::new(1.308_996_9),
            ChannelScreen::new(0.0),
            ChannelScreen::new(std::f32::consts::FRAC_PI_4),
        ],
        CmykScreenPreset::MoireResistant => [
            ChannelScreen::new(0.321_140_6),
            ChannelScreen::new(1.249_455_8),
            ChannelScreen::new(0.0),
            ChannelScreen::new(std::f32::consts::FRAC_PI_4),
        ],
    }
}

const fn channel_index(mode: ColourHalftoneMode, channel: HalftoneChannel) -> Option<usize> {
    match (mode, channel) {
        (ColourHalftoneMode::Rgb, HalftoneChannel::Red)
        | (ColourHalftoneMode::Cmyk, HalftoneChannel::Cyan) => Some(0),
        (ColourHalftoneMode::Rgb, HalftoneChannel::Green)
        | (ColourHalftoneMode::Cmyk, HalftoneChannel::Magenta) => Some(1),
        (ColourHalftoneMode::Rgb, HalftoneChannel::Blue)
        | (ColourHalftoneMode::Cmyk, HalftoneChannel::Yellow) => Some(2),
        (ColourHalftoneMode::Cmyk, HalftoneChannel::Black) => Some(3),
        _ => None,
    }
}

fn invalid_halftone_channel() -> DitherError {
    DitherError::new(
        ErrorKind::InvalidParameter,
        "halftone channel does not belong to the selected mode",
    )
}

pub(super) fn rgb_to_cmyk([red, green, blue]: [u8; 3]) -> [u8; 4] {
    let black = 255 - red.max(green).max(blue);
    if black == 255 {
        return [0, 0, 0, 255];
    }
    let range = u32::from(255 - black);
    [
        ((u32::from(255 - red - black) * 255 + range / 2) / range) as u8,
        ((u32::from(255 - green - black) * 255 + range / 2) / range) as u8,
        ((u32::from(255 - blue - black) * 255 + range / 2) / range) as u8,
        black,
    ]
}

pub(super) fn cmyk_to_rgb([cyan, magenta, yellow, black]: [u8; 4]) -> [u8; 3] {
    [cyan, magenta, yellow].map(|channel| 255_u8.saturating_sub(channel).saturating_sub(black))
}

#[allow(clippy::too_many_arguments)]
fn halftone_threshold(
    shape: HalftoneShape,
    cell_width: u32,
    cell_height: u32,
    angle: f32,
    phase: (f32, f32),
    scale: f32,
    x: f32,
    y: f32,
) -> f32 {
    let (sin, cos) = angle.sin_cos();
    let screen_x = cos * x + sin * y - phase.0;
    let screen_y = -sin * x + cos * y - phase.1;
    let width = cell_width as f32;
    let height = cell_height as f32;
    let x = screen_x.rem_euclid(width) - width * 0.5;
    let y = screen_y.rem_euclid(height) - height * 0.5;
    let nx = x.abs() / (width * 0.5);
    let ny = y.abs() / (height * 0.5);
    let threshold = match shape {
        HalftoneShape::Circle => {
            let unit = width.min(height) * 0.5;
            (x * x + y * y).sqrt() / (unit * std::f32::consts::SQRT_2)
        }
        HalftoneShape::Square => x.abs().max(y.abs()) / (width.min(height) * 0.5),
        HalftoneShape::Diamond => (nx + ny) * 0.5,
        HalftoneShape::Ellipse => (nx * nx + ny * ny).sqrt() / std::f32::consts::SQRT_2,
        HalftoneShape::Line => ny,
        HalftoneShape::Cross => nx.min(ny),
    };
    (threshold / scale).clamp(0.0, 1.0)
}
