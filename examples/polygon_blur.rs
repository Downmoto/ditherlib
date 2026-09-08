use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{Blur, Point, Polygon, Renderer, Selection, read, write};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: polygon_blur INPUT OUTPUT.png");
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
    let polygon = Polygon::new([
        Point::new(width * 0.50, height * 0.10),
        Point::new(width * 0.59, height * 0.38),
        Point::new(width * 0.88, height * 0.38),
        Point::new(width * 0.65, height * 0.55),
        Point::new(width * 0.74, height * 0.84),
        Point::new(width * 0.50, height * 0.67),
        Point::new(width * 0.26, height * 0.84),
        Point::new(width * 0.35, height * 0.55),
        Point::new(width * 0.12, height * 0.38),
        Point::new(width * 0.41, height * 0.38),
    ])?;
    let rendered = Renderer::new().render(&source, &Blur::new(32.0)?, &Selection::Polygon(polygon))?;

    write(output, &rendered)
}
