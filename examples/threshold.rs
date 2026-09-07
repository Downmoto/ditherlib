use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{Palette, Renderer, Selection, Threshold, read, write};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: threshold INPUT OUTPUT.png");
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
    let effect = Threshold::new(Palette::monochrome());
    let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;
    write(output, &rendered)
}
