use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::prelude::*;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(sampling_output), Some(shapes_output), None) = (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) else {
        eprintln!("usage: pixel_sampling_comparison INPUT SAMPLING_OUTPUT.png SHAPES_OUTPUT.png");
        return ExitCode::FAILURE;
    };

    match run(input, sampling_output, shapes_output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    input: OsString,
    sampling_output: OsString,
    shapes_output: OsString,
) -> ditherlib::Result<()> {
    let source = read(input)?;
    let base = Threshold::new(Palette::pico_8()).with_pixel_size(8, 8)?;
    let samplings = [
        SamplingMode::Average,
        SamplingMode::Centre,
        SamplingMode::Darkest,
        SamplingMode::Lightest,
        SamplingMode::DominantColour,
    ]
    .map(|sampling| base.clone().with_sampling(sampling));

    // Left to right: average, centre, darkest, lightest, dominant colour.
    write(sampling_output, &render_columns(&source, samplings)?)?;

    let shapes = [
        base.clone(),
        base.clone().with_pixel_size(12, 4)?,
        base.clone().with_pixel_size(4, 12)?,
        base.with_grid_offset(40, 40),
    ];
    // Reading order: square, wide, tall, offset square.
    write(shapes_output, &render_quadrants(&source, shapes)?)
}

fn render_columns(
    source: &SourceImage,
    effects: [Threshold; 5],
) -> ditherlib::Result<RenderedImage> {
    let width = source.width() as f32;
    let height = source.height() as f32;
    let mut pipeline = Pipeline::new();

    for (index, effect) in effects.into_iter().enumerate() {
        let left = width * index as f32 / 5.0;
        let right = width * (index + 1) as f32 / 5.0;
        let area = Polygon::rectangle(Point::new(left, 0.0), right - left, height)?;
        pipeline.add(effect, Selection::Polygon(area));
    }

    Renderer::new().render_pipeline(source, &pipeline)
}

fn render_quadrants(
    source: &SourceImage,
    effects: [Threshold; 4],
) -> ditherlib::Result<RenderedImage> {
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
    let mut pipeline = Pipeline::new();

    for (effect, (left, top, right, bottom)) in effects.into_iter().zip(quadrants) {
        let area = Polygon::rectangle(Point::new(left, top), right - left, bottom - top)?;
        pipeline.add(effect, Selection::Polygon(area));
    }

    Renderer::new().render_pipeline(source, &pipeline)
}
