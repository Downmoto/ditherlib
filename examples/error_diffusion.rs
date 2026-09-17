mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Effect, Renderer, Selection,
    effects::dither::{
        diffusion::{DiffusionAlgorithm, ErrorDiffusion},
        ostromoukhov::OstromoukhovDither,
        palette::Palette,
        riemersma::RiemersmaDither,
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("error_diffusion", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let palette = Palette::black_and_white();
    let floyd_steinberg = ErrorDiffusion::new(palette.clone(), DiffusionAlgorithm::FloydSteinberg);
    let ostromoukhov = OstromoukhovDither::new(palette.clone());
    let riemersma = RiemersmaDither::new(palette);
    let effects = [
        ("floyd_steinberg.png", &floyd_steinberg as &dyn Effect),
        ("ostromoukhov.png", &ostromoukhov as &dyn Effect),
        ("riemersma.png", &riemersma as &dyn Effect),
    ];
    let mut renderer = Renderer::new();

    for (filename, effect) in effects {
        let rendered = renderer.render(&source, effect, &Selection::All)?;
        write(output_directory.join(filename), &rendered)?;
    }

    Ok(())
}
