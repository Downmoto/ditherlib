mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Pipeline, Renderer, Selection,
    effects::{
        blur::Blur,
        dither::{
            palette::Palette,
            threshold::{OrderedDither, ThresholdMap},
        },
        greyscale::Greyscale,
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("pipeline", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let mut pipeline = Pipeline::new();
    pipeline.add(Greyscale, Selection::All);
    pipeline.add(Blur::new(1.5)?, Selection::All);
    pipeline.add(
        OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
            .with_pixel_size(4, 4)?,
        Selection::All,
    );

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output_directory.join("pipeline.png"), &rendered)?;
    Ok(())
}
