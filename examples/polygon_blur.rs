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
        Point::new(width * 0.20, height * 0.25),
        Point::new(width * 0.75, height * 0.15),
        Point::new(width * 0.85, height * 0.70),
        Point::new(width * 0.30, height * 0.85),
    ])?;
    let rendered =
        Renderer::new().render(&source, &Blur::new(50.0)?, &Selection::Polygon(polygon))?;

    write(output, &rendered)
}
