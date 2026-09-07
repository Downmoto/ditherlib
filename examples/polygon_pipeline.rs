use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    Blur, Greyscale, OrderedDither, Palette, Pipeline, Point, Polygon, Renderer, Selection, read,
    write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: polygon_pipeline INPUT OUTPUT.png");
        return ExitCode::FAILURE;
    };

    match run(input, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: OsString, output: OsString) -> ditherlib::Result<()> {
    let source = read(input)?;
    let width = source.width() as f32;
    let height = source.height() as f32;

    let blur_area = Polygon::new([
        Point::new(width * 0.05, height * 0.10),
        Point::new(width * 0.65, height * 0.05),
        Point::new(width * 0.75, height * 0.70),
        Point::new(width * 0.15, height * 0.80),
    ])?;
    let greyscale_area = Polygon::new([
        Point::new(width * 0.30, height * 0.20),
        Point::new(width * 0.95, height * 0.15),
        Point::new(width * 0.85, height * 0.90),
        Point::new(width * 0.25, height * 0.75),
    ])?;
    let dither_area = Polygon::new([
        Point::new(width * 0.50, height * 0.05),
        Point::new(width * 0.90, height * 0.55),
        Point::new(width * 0.45, height * 0.95),
        Point::new(width * 0.15, height * 0.50),
    ])?;

    let mut pipeline = Pipeline::new();
    pipeline.add(Greyscale, Selection::Polygon(greyscale_area));
    pipeline.add(
        OrderedDither::new(Palette::monochrome([200, 0, 80]), 4)?.with_pixel_size(4)?,
        Selection::Polygon(dither_area),
    );
    pipeline.add(Blur::new(8.0)?, Selection::Polygon(blur_area));

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
