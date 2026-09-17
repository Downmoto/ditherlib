mod cells;
/// RGB colours and colour spaces.
pub mod colour;
/// Fixed-kernel error diffusion.
pub mod diffusion;
/// Monochrome and colour halftone screens.
pub mod halftone;
/// White-noise and blue-noise dithering.
pub mod noise;
/// Ostromoukhov variable-coefficient error diffusion.
pub mod ostromoukhov;
/// Colour palettes and matching configuration.
pub mod palette;
/// Riemersma Hilbert-curve error diffusion.
pub mod riemersma;
/// Logical-pixel sampling modes shared by dithering effects.
pub mod sampling {
    pub use super::cells::SamplingMode;
}
/// Threshold and ordered dithering.
pub mod threshold;

#[cfg(test)]
mod tests;
