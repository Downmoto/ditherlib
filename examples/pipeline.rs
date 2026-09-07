use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{Blur, OrderedDither, Palette, Pipeline, Renderer, Selection, read, write};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: pipeline INPUT OUTPUT.png");
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
    let mut pipeline = Pipeline::new();
    pipeline.add(Blur::new(2.0)?, Selection::All);
    pipeline.add(
        OrderedDither::new(Palette::black_and_white(), 4)?.with_pixel_size(4)?,
        Selection::All,
    );

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write(output, &rendered)
}
