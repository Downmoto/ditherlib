mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{Point, Polygon, Renderer, Selection, effects::greyscale::Greyscale, read, write};

fn main() -> ExitCode {
    common::run("selections", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let (width, height) = source.dimensions();
    let polygon = Polygon::regular(
        Point::new(width as f32 / 2.0, height as f32 / 2.0),
        6,
        width.min(height) as f32 * 0.4,
        -std::f32::consts::FRAC_PI_2,
    )?;
    let selections = [
        ("selection_all.png", Selection::All),
        ("selection_polygon.png", Selection::Polygon(polygon)),
    ];
    let mut renderer = Renderer::new();

    for (filename, selection) in selections {
        let rendered = renderer.render(&source, &Greyscale, &selection)?;
        write(output_directory.join(filename), &rendered)?;
    }

    Ok(())
}
