use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DiffusionAlgorithm, DitherError, ErrorDiffusion, ErrorKind, Palette, Pipeline, Point, Polygon,
    RenderedImage, Renderer, Selection, SourceImage, read, write,
};

const COMPARISONS: [[DiffusionAlgorithm; 2]; 7] = [
    [
        DiffusionAlgorithm::FloydSteinberg,
        DiffusionAlgorithm::Atkinson,
    ],
    [
        DiffusionAlgorithm::JarvisJudiceNinke,
        DiffusionAlgorithm::Stucki,
    ],
    [DiffusionAlgorithm::Burkes, DiffusionAlgorithm::Sierra],
    [
        DiffusionAlgorithm::TwoRowSierra,
        DiffusionAlgorithm::SierraLite,
    ],
    [
        DiffusionAlgorithm::FalseFloydSteinberg,
        DiffusionAlgorithm::Fan,
    ],
    [DiffusionAlgorithm::ShiauFan, DiffusionAlgorithm::ShiauFan2],
    [
        DiffusionAlgorithm::StevensonArce,
        DiffusionAlgorithm::TwoDimensionalKnuth,
    ],
];

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != COMPARISONS.len() + 1 {
        eprintln!("usage: diffusion_comparison INPUT OUTPUT_1.png ... OUTPUT_7.png");
        return ExitCode::FAILURE;
    }

    match run(&arguments[0], &arguments[1..]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: &OsString, outputs: &[OsString]) -> ditherlib::Result<()> {
    let source = read(input)?;
    if source.width() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires an even width",
        ));
    }

    for (output, algorithms) in outputs.iter().zip(COMPARISONS) {
        write(output, &render_comparison(&source, algorithms)?)?;
    }
    Ok(())
}

fn render_comparison(
    source: &SourceImage,
    algorithms: [DiffusionAlgorithm; 2],
) -> ditherlib::Result<RenderedImage> {
    let width = source.width() as f32;
    let height = source.height() as f32;
    let middle_x = width / 2.0;
    let halves = [(0.0, middle_x), (middle_x, width)];
    let mut pipeline = Pipeline::new();

    for (algorithm, (left, right)) in algorithms.into_iter().zip(halves) {
        let area = Polygon::new([
            Point::new(left, 0.0),
            Point::new(right, 0.0),
            Point::new(right, height),
            Point::new(left, height),
        ])?;
        pipeline.add(
            ErrorDiffusion::new(Palette::black_and_white(), algorithm),
            Selection::Polygon(area),
        );
    }

    Renderer::new().render_pipeline(source, &pipeline)
}
