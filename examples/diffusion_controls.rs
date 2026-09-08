use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DiffusionAlgorithm, DiffusionScan, DitherError, ErrorDiffusion, ErrorKind, Palette, Pipeline,
    Point, Polygon, RenderedImage, Renderer, Selection, SourceImage, read, write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(strength_output), Some(clamp_output), None) = (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) else {
        eprintln!("usage: diffusion_controls INPUT STRENGTH_OUTPUT.png CLAMP_OUTPUT.png");
        return ExitCode::FAILURE;
    };

    match run(input, strength_output, clamp_output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    input: OsString,
    strength_output: OsString,
    clamp_output: OsString,
) -> ditherlib::Result<()> {
    let source = read(input)?;
    if source.width() % 2 != 0 || source.height() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires even width and height",
        ));
    }

    let strengths = [
        base_effect().with_strength(0.0)?,
        base_effect().with_strength(0.5)?,
        base_effect(),
        base_effect().with_strength(1.5)?,
    ];
    let clamps = [
        base_effect().with_error_clamp(0),
        base_effect().with_error_clamp(24),
        base_effect().with_error_clamp(64),
        base_effect(),
    ];

    write(strength_output, &render_comparison(&source, strengths)?)?;
    write(clamp_output, &render_comparison(&source, clamps)?)
}

fn base_effect() -> ErrorDiffusion {
    ErrorDiffusion::new(
        Palette::black_and_white(),
        DiffusionAlgorithm::FloydSteinberg,
    )
    .with_scan(DiffusionScan::Serpentine)
}

fn render_comparison(
    source: &SourceImage,
    effects: [ErrorDiffusion; 4],
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
        let area = Polygon::new([
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ])?;
        pipeline.add(effect, Selection::Polygon(area));
    }

    Renderer::new().render_pipeline(source, &pipeline)
}
