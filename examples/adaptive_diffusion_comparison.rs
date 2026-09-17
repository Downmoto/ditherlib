use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::prelude::*;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: adaptive_diffusion_comparison INPUT OUTPUT.png");
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
    let middle = width / 2.0;
    let left = Polygon::rectangle(Point::new(0.0, 0.0), middle, height)?;
    let right = Polygon::rectangle(Point::new(middle, 0.0), width - middle, height)?;
    let mut pipeline = Pipeline::new();

    pipeline.add(
        OstromoukhovDither::new(Palette::black_and_white()).with_pixel_size(4, 4)?,
        Selection::Polygon(left),
    );
    pipeline.add(
        ErrorDiffusion::new(
            Palette::black_and_white(),
            DiffusionAlgorithm::FloydSteinberg,
        )
        .with_scan(DiffusionScan::Serpentine)
        .with_pixel_size(4, 4)?,
        Selection::Polygon(right),
    );

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
