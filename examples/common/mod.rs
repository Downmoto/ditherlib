use std::{error::Error, ffi::OsString, fs, path::PathBuf, process::ExitCode};

pub type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

pub fn run(name: &str, example: impl FnOnce(OsString, PathBuf) -> Result) -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(input), Some(output_directory), None) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: cargo run --example {name} -- INPUT OUTPUT_DIRECTORY");
        return ExitCode::FAILURE;
    };

    let output_directory = PathBuf::from(output_directory);
    if let Err(error) = fs::create_dir_all(&output_directory) {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }

    match example(input, output_directory) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
