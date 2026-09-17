mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Effect, Renderer, Selection,
    effects::dither::{
        diffusion::{DiffusionAlgorithm, ErrorDiffusion},
        halftone::{ColourHalftone, ColourHalftoneMode, Halftone, HalftoneShape},
        noise::{NoiseAlgorithm, NoiseDither},
        palette::Palette,
        threshold::{OrderedDither, Threshold, ThresholdMap},
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("comparison", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let threshold = Threshold::new(Palette::pico_8());
    let ordered = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4());
    let noise = NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue);
    let error_diffusion = ErrorDiffusion::new(
        Palette::black_and_white(),
        DiffusionAlgorithm::FloydSteinberg,
    );
    let halftone = Halftone::new(Palette::black_and_white(), HalftoneShape::Circle);
    let colour_halftone = ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle);
    let effects = [
        ("comparison_threshold.png", &threshold as &dyn Effect),
        ("comparison_ordered.png", &ordered as &dyn Effect),
        ("comparison_noise.png", &noise as &dyn Effect),
        (
            "comparison_error_diffusion.png",
            &error_diffusion as &dyn Effect,
        ),
        ("comparison_halftone.png", &halftone as &dyn Effect),
        (
            "comparison_colour_halftone.png",
            &colour_halftone as &dyn Effect,
        ),
    ];
    let mut renderer = Renderer::new();

    for (filename, effect) in effects {
        let rendered = renderer.render(&source, effect, &Selection::All)?;
        write(output_directory.join(filename), &rendered)?;
    }

    Ok(())
}
