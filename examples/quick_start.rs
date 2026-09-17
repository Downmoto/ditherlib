mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Renderer, Selection,
    effects::dither::{palette::Palette, threshold::Threshold},
    read, write,
};

fn main() -> ExitCode {
    common::run("quick_start", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let effect = Threshold::new(Palette::black_and_white());
    let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;

    write(output_directory.join("quick_start.png"), &rendered)?;
    Ok(())
}
