mod common;

use std::{ffi::OsString, path::PathBuf, process::ExitCode};

use ditherlib::{
    Renderer, Selection,
    effects::dither::{
        colour::ColourSpace,
        palette::{Palette, PaletteSize},
        threshold::Threshold,
    },
    read, write,
};

fn main() -> ExitCode {
    common::run("palettes", run)
}

fn run(input: OsString, output_directory: PathBuf) -> common::Result {
    let source = read(input)?;
    let palettes = [
        ("palette_builtin.png", Palette::pico_8()),
        (
            "palette_custom.png",
            Palette::new([
                [20, 20, 32],
                [238, 66, 102],
                [72, 191, 145],
                [255, 245, 230],
            ])?,
        ),
        (
            "palette_derived.png",
            Palette::from_source(&source, PaletteSize::Limited(8))?,
        ),
        (
            "palette_perceptual.png",
            Palette::pico_8().with_colour_space(ColourSpace::Oklab),
        ),
    ];
    let mut renderer = Renderer::new();

    for (filename, palette) in palettes {
        let effect = Threshold::new(palette);
        let rendered = renderer.render(&source, &effect, &Selection::All)?;
        write(output_directory.join(filename), &rendered)?;
    }

    Ok(())
}
