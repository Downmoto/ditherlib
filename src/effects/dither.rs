mod cells;
mod colour;
mod diffusion;
mod halftone;
mod noise;
mod palette;
mod threshold;

pub use colour::{Color, Colour, ColourSpace};
pub use diffusion::{
    DiffusionAlgorithm, DiffusionErrorMode, DiffusionKernel, DiffusionScan, DiffusionTap,
    ErrorDiffusion,
};
pub use halftone::{
    ChannelMode, CmykScreenPreset, ColourChannel, ColourHalftone, ColourHalftoneMode, DotShape,
    Halftone, HalftoneChannel, HalftoneShape,
};
pub use noise::{NoiseAlgorithm, NoiseDither};
pub use palette::{Palette, PaletteMatchMode, PaletteSize};
pub use threshold::{OrderedDither, Threshold, ThresholdMap, ThresholdRotation};

#[cfg(test)]
mod tests;
