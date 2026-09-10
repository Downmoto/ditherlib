use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    DitherError, ErrorKind, NoiseAlgorithm, NoiseDither, Palette, Pipeline, Point, Polygon,
    Renderer, Selection, read, write,
};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: noise_comparison INPUT OUTPUT.png");
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
    if source.width() % 2 != 0 {
        return Err(DitherError::new(
            ErrorKind::InvalidParameter,
            "the comparison image requires an even width",
        ));
    }

    let width = source.width() as f32;
    let height = source.height() as f32;
    let middle = width / 2.0;
    let mut pipeline = Pipeline::new();
    for (algorithm, left, right) in [
        (NoiseAlgorithm::White, 0.0, middle),
        (NoiseAlgorithm::Blue, middle, width),
    ] {
        let area = Polygon::rectangle(Point::new(left, 0.0), right - left, height)?;
        let effect = NoiseDither::new(Palette::black_and_white(), algorithm)
            .with_seed(67)
            .with_strength(0.85)?;
        pipeline.add(effect, Selection::Polygon(area));
    }

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
