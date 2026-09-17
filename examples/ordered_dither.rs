mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Renderer, Selection,
    effects::dither::{
        palette::Palette,
        threshold::{OrderedDither, ThresholdMap},
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("ordered_dither", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
        .with_pixel_size(4, 4)?;
    let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;

    write(output_directory.join("ordered_dither.png"), &rendered)?;
    Ok(())
}
