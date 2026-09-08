use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DiffusionAlgorithm, DitherError, ErrorDiffusion, ErrorKind, Palette, Pipeline, Point, Polygon,
    RenderedImage, Renderer, Selection, SourceImage, read, write,
};

const FIRST_IMAGE: [DiffusionAlgorithm; 4] = [
    DiffusionAlgorithm::FloydSteinberg,
    DiffusionAlgorithm::Atkinson,
    DiffusionAlgorithm::JarvisJudiceNinke,
    DiffusionAlgorithm::Stucki,
];
const SECOND_IMAGE: [DiffusionAlgorithm; 4] = [
    DiffusionAlgorithm::Burkes,
    DiffusionAlgorithm::Sierra,
    DiffusionAlgorithm::TwoRowSierra,
    DiffusionAlgorithm::SierraLite,
];

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(first_output), Some(second_output), None) = (
        arguments.next(),
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) else {
        eprintln!("usage: diffusion_comparison INPUT FIRST_OUTPUT.png SECOND_OUTPUT.png");
        return ExitCode::FAILURE;
    };

    match run(input, first_output, second_output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: OsString, first_output: OsString, second_output: OsString) -> ditherlib::Result<()> {
    let source = read(input)?;
    if source.width() % 2 != 0 || source.height() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires even width and height",
        ));
    }

    write(first_output, &render_comparison(&source, FIRST_IMAGE)?)?;
    write(second_output, &render_comparison(&source, SECOND_IMAGE)?)
}

fn render_comparison(
    source: &SourceImage,
    algorithms: [DiffusionAlgorithm; 4],
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

    for (algorithm, (left, top, right, bottom)) in algorithms.into_iter().zip(quadrants) {
        let area = Polygon::new([
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ])?;
        pipeline.add(
            ErrorDiffusion::new(Palette::black_and_white(), algorithm),
            Selection::Polygon(area),
        );
    }

    Renderer::new().render_pipeline(source, &pipeline)
}
