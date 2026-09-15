use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    ColourSpace, DitherError, ErrorKind, Palette, Pipeline, Point, Polygon, Renderer, Selection,
    Threshold, read, write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: palette_matching_comparison INPUT OUTPUT.png");
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

    let middle_x = source.width() as f32 / 2.0;
    let middle_y = source.height() as f32 / 2.0;
    let areas = [
        (middle_x, 0.0, source.width() as f32, middle_y),
        (0.0, middle_y, middle_x, source.height() as f32),
        (
            middle_x,
            middle_y,
            source.width() as f32,
            source.height() as f32,
        ),
    ];
    let mut pipeline = Pipeline::new();

    // Reading order: source, RGB, linear RGB, Oklab.
    for (colour_space, (left, top, right, bottom)) in
        [ColourSpace::Rgb, ColourSpace::LinearRgb, ColourSpace::Oklab]
            .into_iter()
            .zip(areas)
    {
        let area = Polygon::new([
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ])?;
        pipeline.add(
            Threshold::new(Palette::pico_8().with_colour_space(colour_space)),
            Selection::Polygon(area),
        );
    }

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
