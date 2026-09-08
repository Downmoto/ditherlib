use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DiffusionAlgorithm, DiffusionScan, DitherError, ErrorDiffusion, ErrorKind, Palette, Pipeline,
    Point, Polygon, Renderer, Selection, read, write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: palette_comparison INPUT OUTPUT.png");
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
    if source.width() % 2 != 0 || source.height() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires even width and height",
        ));
    }

    let width = source.width() as f32;
    let height = source.height() as f32;
    let middle_x = width / 2.0;
    let middle_y = height / 2.0;
    let quadrants = [
        (0.0, 0.0, middle_x, middle_y),
        (middle_x, 0.0, width, middle_y),
        (0.0, middle_y, middle_x, height),
        (middle_x, middle_y, width, height),
    ];
    let palettes = [
        Palette::pico_8(),
        Palette::cga(),
        Palette::greyscale(),
        Palette::game_boy(),
    ];
    let mut pipeline = Pipeline::new();

    for (palette, (left, top, right, bottom)) in palettes.into_iter().zip(quadrants) {
        let area = Polygon::new([
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ])?;
        pipeline.add(
            ErrorDiffusion::new(palette, DiffusionAlgorithm::FloydSteinberg)
                .with_scan(DiffusionScan::Serpentine),
            Selection::Polygon(area),
        );
    }

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
