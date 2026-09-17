mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Effect, Renderer, Selection,
    effects::dither::{
        halftone::{ColourHalftone, ColourHalftoneMode, Halftone, HalftoneShape},
        palette::Palette,
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("halftone", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let monochrome = Halftone::new(Palette::black_and_white(), HalftoneShape::Circle);
    let colour = ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle);
    let effects = [
        ("halftone_monochrome.png", &monochrome as &dyn Effect),
        ("halftone_colour.png", &colour as &dyn Effect),
    ];
    let mut renderer = Renderer::new();

    for (filename, effect) in effects {
        let rendered = renderer.render(&source, effect, &Selection::All)?;
        write(output_directory.join(filename), &rendered)?;
    }

    Ok(())
}
