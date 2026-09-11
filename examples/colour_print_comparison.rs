use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{
    CmykScreenPreset, ColourHalftone, ColourHalftoneMode, HalftoneShape, Renderer, Selection, read,
};
use image::{Rgba, RgbaImage, imageops::FilterType};

const MAX_TILE_WIDTH: u32 = 360;
const MAX_TILE_HEIGHT: u32 = 360;
const GUTTER: u32 = 8;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: colour_print_comparison INPUT OUTPUT.png");
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

fn run(input: OsString, output: OsString) -> Result<(), Box<dyn std::error::Error>> {
    let source = read(input)?;
    let cell_size = (source.width().min(source.height()) / 64).max(4);
    let rgb = ColourHalftone::new(ColourHalftoneMode::Rgb, HalftoneShape::Circle)
        .with_cell_size(cell_size, cell_size)?;
    let cmyk = ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle)
        .with_cell_size(cell_size, cell_size)?
        .with_cmyk_preset(CmykScreenPreset::Traditional)?;
    let rgb = Renderer::new().render(&source, &rgb, &Selection::All)?;
    let cmyk = Renderer::new().render(&source, &cmyk, &Selection::All)?;
    let images = [source.rgba8_bytes(), rgb.rgba8_bytes(), cmyk.rgba8_bytes()];
    let (tile_width, tile_height) = thumbnail_dimensions(source.dimensions());
    let mut sheet = RgbaImage::from_pixel(
        tile_width * 3 + GUTTER * 2,
        tile_height,
        Rgba([245, 245, 245, 255]),
    );

    // Reading order: source, additive RGB screens, subtractive CMYK screens.
    for (index, pixels) in images.into_iter().enumerate() {
        let image = RgbaImage::from_raw(source.width(), source.height(), pixels.to_vec())
            .expect("image dimensions match its pixel buffer");
        let image = image::imageops::resize(&image, tile_width, tile_height, FilterType::Triangle);
        image::imageops::replace(
            &mut sheet,
            &image,
            i64::from(index as u32 * (tile_width + GUTTER)),
            0,
        );
    }

    sheet.save(output)?;
    Ok(())
}

fn thumbnail_dimensions((width, height): (u32, u32)) -> (u32, u32) {
    if u64::from(width) * u64::from(MAX_TILE_HEIGHT) > u64::from(height) * u64::from(MAX_TILE_WIDTH)
    {
        (
            MAX_TILE_WIDTH,
            (u64::from(height) * u64::from(MAX_TILE_WIDTH) / u64::from(width)).max(1) as u32,
        )
    } else {
        (
            (u64::from(width) * u64::from(MAX_TILE_HEIGHT) / u64::from(height)).max(1) as u32,
            MAX_TILE_HEIGHT,
        )
    }
}
