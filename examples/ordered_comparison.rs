use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DitherError, ErrorKind, OrderedDither, Palette, Pipeline, Point, Polygon, RenderedImage,
    Renderer, Selection, SourceImage, ThresholdMap, ThresholdRotation, read, write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(maps_output), Some(strengths_output), Some(rotations_output), None) = (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) else {
        eprintln!(
            "usage: ordered_comparison INPUT MAPS_OUTPUT.png STRENGTHS_OUTPUT.png ROTATIONS_OUTPUT.png"
        );
        return ExitCode::FAILURE;
    };

    match run(input, maps_output, strengths_output, rotations_output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    input: OsString,
    maps_output: OsString,
    strengths_output: OsString,
    rotations_output: OsString,
) -> ditherlib::Result<()> {
    let source = read(input)?;
    if source.width() % 2 != 0 || source.height() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires even width and height",
        ));
    }

    let palette = Palette::black_and_white();
    let maps = [
        ThresholdMap::bayer_2x2(),
        ThresholdMap::bayer_4x4(),
        ThresholdMap::bayer_8x8(),
        ThresholdMap::new(3, 2, [0, 4, 2, 5, 1, 3])?,
    ]
    .map(|map| OrderedDither::new(palette.clone(), map));
    let strengths = [0.25, 0.5, 1.0, 1.5]
        .map(|strength| {
            OrderedDither::new(palette.clone(), ThresholdMap::bayer_4x4()).with_strength(strength)
        })
        .into_iter()
        .collect::<ditherlib::Result<Vec<_>>>()?
        .try_into()
        .expect("four strengths produce four effects");
    let rotations = [
        ThresholdRotation::None,
        ThresholdRotation::Clockwise90,
        ThresholdRotation::Clockwise180,
        ThresholdRotation::Clockwise270,
    ]
    .map(|rotation| {
        OrderedDither::new(
            palette.clone(),
            ThresholdMap::new(3, 2, [0, 4, 2, 5, 1, 3]).expect("the built-in map is valid"),
        )
        .with_rotation(rotation)
    });

    write(maps_output, &render_comparison(&source, maps)?)?;
    write(strengths_output, &render_comparison(&source, strengths)?)?;
    write(rotations_output, &render_comparison(&source, rotations)?)
}

fn render_comparison(
    source: &SourceImage,
    effects: [OrderedDither; 4],
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
