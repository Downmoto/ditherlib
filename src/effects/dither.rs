mod cells;
mod colour;
mod diffusion;
mod halftone;
mod noise;
mod ostromoukhov;
mod palette;
mod riemersma;
mod threshold;

pub use cells::SamplingMode;
pub use colour::{Color, Colour, ColourSpace};
pub use diffusion::{
    DiffusionAlgorithm, DiffusionErrorMode, DiffusionKernel, DiffusionScan, DiffusionTap,
    ErrorDiffusion,
};
pub use halftone::{
    CmykScreenPreset, ColourHalftone, ColourHalftoneMode, Halftone, HalftoneChannel, HalftoneShape,
};
pub use noise::{NoiseAlgorithm, NoiseDither};
pub use ostromoukhov::OstromoukhovDither;
pub use palette::{Palette, PaletteMatchMode, PaletteSize};
pub use riemersma::RiemersmaDither;
pub use threshold::{OrderedDither, Threshold, ThresholdMap, ThresholdRotation};

#[cfg(test)]
mod tests;
