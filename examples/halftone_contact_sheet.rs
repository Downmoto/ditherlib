use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{Halftone, HalftoneShape, Palette, Renderer, Selection, read};
use image::{Rgba, RgbaImage, imageops::FilterType};

const COLUMNS: u32 = 3;
const MAX_TILE_WIDTH: u32 = 320;
const MAX_TILE_HEIGHT: u32 = 240;
const GUTTER: u32 = 8;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: halftone_contact_sheet INPUT OUTPUT.png");
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
    let (tile_width, tile_height) = thumbnail_dimensions(source.dimensions());
    let shapes = [
        HalftoneShape::Circle,
        HalftoneShape::Square,
        HalftoneShape::Diamond,
        HalftoneShape::Ellipse,
        HalftoneShape::Line,
        HalftoneShape::Cross,
    ];
    let rows = shapes.len().div_ceil(COLUMNS as usize) as u32;
    let mut sheet = RgbaImage::from_pixel(
        COLUMNS * tile_width + (COLUMNS - 1) * GUTTER,
        rows * tile_height + (rows - 1) * GUTTER,
        Rgba([245, 245, 245, 255]),
    );
    let cell_size = (source.width() / 48).max(4);

    // Reading order: circle, square, diamond, ellipse, line, cross.
    for (index, shape) in shapes.into_iter().enumerate() {
        let effect = Halftone::new(Palette::black_and_white(), shape)
            .with_cell_size(cell_size, cell_size)?
            .with_angle(std::f32::consts::FRAC_PI_6)?;
        let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;
        let image = RgbaImage::from_raw(
            source.width(),
            source.height(),
            rendered.rgba8_bytes().to_vec(),
        )
        .expect("rendered dimensions match its pixel buffer");
        let image = image::imageops::resize(&image, tile_width, tile_height, FilterType::Triangle);
        let column = index as u32 % COLUMNS;
        let row = index as u32 / COLUMNS;
        image::imageops::replace(
            &mut sheet,
            &image,
            i64::from(column * (tile_width + GUTTER)),
            i64::from(row * (tile_height + GUTTER)),
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
