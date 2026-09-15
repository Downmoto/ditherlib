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

/// The space used to compare colours and calculate quantisation errors.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ColourSpace {
    /// Gamma-encoded sRGB channels. This preserves pre-0.8 behaviour.
    #[default]
    Rgb,
    /// Linear-light sRGB channels.
    LinearRgb,
    /// The perceptually uniform Oklab colour space.
    Oklab,
}

pub(super) fn colour_components(colour: [u8; 3], colour_space: ColourSpace) -> [f32; 3] {
    let rgb = colour.map(|channel| f32::from(channel) / 255.0);
    match colour_space {
        ColourSpace::Rgb => rgb,
        ColourSpace::LinearRgb => rgb.map(srgb_to_linear),
        ColourSpace::Oklab => linear_rgb_to_oklab(rgb.map(srgb_to_linear)),
    }
}

/// Decodes one gamma-encoded sRGB component into linear light.
///
/// The 0.04045 breakpoint, 12.92 linear slope, 0.055 offset, 1.055 scale, and
/// 2.4 exponent come from the IEC 61966-2-1 sRGB transfer function, reproduced
/// in CSS Color 4 section 10.2:
/// <https://www.w3.org/TR/css-color-4/#color-conversion-code>
fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts linear-light sRGB to Oklab using the reference transform.
///
/// The first matrix maps linear sRGB to approximate LMS cone responses. The
/// cube roots apply Oklab's non-linearity, and the second matrix maps those
/// responses to lightness and the two opponent-colour axes. Coefficients are
/// from Björn Ottosson's reference implementation:
/// <https://bottosson.github.io/posts/oklab/#converting-from-linear-srgb-to-oklab>
fn linear_rgb_to_oklab([red, green, blue]: [f32; 3]) -> [f32; 3] {
    let l = (0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue).cbrt();
    let m = (0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue).cbrt();
    let s = (0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// Returns luma for gamma-encoded RGB, relative luminance for linear RGB, or
/// Oklab's native lightness component.
///
/// The RGB weights are the BT.709/sRGB coefficients defined by WCAG relative
/// luminance: <https://www.w3.org/TR/WCAG22/#dfn-relative-luminance>
pub(super) fn colour_luminance(colour: [f32; 3], colour_space: ColourSpace) -> f32 {
    match colour_space {
        ColourSpace::Rgb | ColourSpace::LinearRgb => {
            0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2]
        }
        ColourSpace::Oklab => colour[0],
    }
}

pub(super) fn clamp_components(mut colour: [f32; 3], colour_space: ColourSpace) -> [f32; 3] {
    match colour_space {
        ColourSpace::Rgb | ColourSpace::LinearRgb => colour.map(|channel| channel.clamp(0.0, 1.0)),
        ColourSpace::Oklab => {
            colour[0] = colour[0].clamp(0.0, 1.0);
            // CSS Color 4 notes that practical Oklab a/b values stay within ±0.5.
            colour[1] = colour[1].clamp(-0.5, 0.5);
            colour[2] = colour[2].clamp(-0.5, 0.5);
            colour
        }
    }
}

/// Calculates squared Euclidean distance between two RGB colours.
pub(super) fn colour_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| {
            let difference = i32::from(left) - i32::from(right);
            (difference * difference) as u32
        })
        .sum()
}
