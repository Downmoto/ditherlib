use std::{env, ffi::OsString, process::ExitCode};

use ditherlib::{OrderedDither, Palette, Renderer, Selection, ThresholdMap, read};
use image::{Rgba, RgbaImage, imageops::FilterType};

const COLUMNS: u32 = 4;
const MAX_TILE_WIDTH: u32 = 320;
const MAX_TILE_HEIGHT: u32 = 240;
const LABEL_HEIGHT: u32 = 11;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: pattern_sheet INPUT OUTPUT.png");
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
    let pixel_size = 2 * source
        .width()
        .div_ceil(tile_width)
        .max(source.height().div_ceil(tile_height));
    let patterns = [
        ("CLUSTERED DOTS", ThresholdMap::clustered_dots()),
        ("HORIZONTAL LINES", ThresholdMap::horizontal_lines()),
        ("VERTICAL LINES", ThresholdMap::vertical_lines()),
        ("DIAGONAL LINES", ThresholdMap::diagonal_lines()),
        ("CROSSHATCH", ThresholdMap::crosshatch()),
        ("CHECKERBOARD", ThresholdMap::checkerboard()),
        ("DISPERSED DOTS 3X3", ThresholdMap::dispersed_dots_3x3()),
        ("DISPERSED DOTS 5X5", ThresholdMap::dispersed_dots_5x5()),
    ];
    let rows = patterns.len().div_ceil(COLUMNS as usize) as u32;
    let mut sheet = RgbaImage::from_pixel(
        COLUMNS * tile_width,
        rows * (tile_height + LABEL_HEIGHT),
        Rgba([245, 245, 245, 255]),
    );

    for (index, (label, map)) in patterns.into_iter().enumerate() {
        let effect = OrderedDither::new(Palette::black_and_white(), map)
            .with_strength(0.85)?
            .with_pixel_size(pixel_size)?;
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
        let left = column * tile_width;
        let top = row * (tile_height + LABEL_HEIGHT);
        draw_label(&mut sheet, left, top, tile_width, label);
        image::imageops::replace(
            &mut sheet,
            &image,
            i64::from(left),
            i64::from(top + LABEL_HEIGHT),
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

fn draw_label(image: &mut RgbaImage, left: u32, top: u32, width: u32, label: &str) {
    let label_width = label.chars().count() as u32 * 6 - 1;
    let start_x = left + width.saturating_sub(label_width) / 2;

    for (character_index, character) in label.chars().enumerate() {
        for (y, row) in glyph(character).into_iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    let x = start_x + character_index as u32 * 6 + x;
                    let y = top + 2 + y as u32;
                    if x < left + width {
                        image.put_pixel(x, y, Rgba([20, 20, 20, 255]));
                    }
                }
            }
        }
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'G' => [14, 17, 16, 23, 17, 17, 14],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [31, 4, 4, 4, 4, 4, 31],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        _ => [0; 7],
    }
}
