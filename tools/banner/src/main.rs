use std::{
    env,
    error::Error,
    fs::{self, File},
    io::BufWriter,
    path::PathBuf,
    process,
};

use ditherlib::{
    Renderer, Selection, SourceImage,
    effects::dither::{
        diffusion::{DiffusionAlgorithm, DiffusionScan, ErrorDiffusion},
        palette::Palette,
    },
};
use image::{
    ExtendedColorType, ImageEncoder, Rgba, RgbaImage,
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::{FilterType, crop_imm, resize},
};

const WIDTH: u32 = 2400;
const HEIGHT: u32 = 800;
const IMAGE_X: u32 = 1000;
const FADE_END_X: u32 = 1420;
const BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
const USAGE: &str = "\
Generate a versioned Ditherlib README banner.

Usage:
  cargo run --release --manifest-path tools/banner/Cargo.toml -- \\
    --version VERSION --input IMAGE --output BANNER.png \\
    [--focus-x 0.5] [--focus-y 0.5]
";

struct Options {
    version: String,
    input: PathBuf,
    output: PathBuf,
    focus_x: f64,
    focus_y: f64,
}

fn main() {
    if env::args().any(|argument| argument == "--help" || argument == "-h") {
        print!("{USAGE}");
        return;
    }

    if let Err(error) = run() {
        eprintln!("banner: {error}\n\n{USAGE}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_options(env::args().skip(1))?;
    let decoded = image::open(&options.input)?.to_rgba8();
    let image_width = WIDTH - IMAGE_X;
    let prepared = crop_and_resize(
        decoded,
        image_width,
        HEIGHT,
        options.focus_x,
        options.focus_y,
    );
    let source = SourceImage::from_rgba8(image_width, HEIGHT, prepared.into_raw())?;
    let palette = Palette::new([
        [14, 13, 12],
        [66, 29, 19],
        [151, 61, 29],
        [223, 113, 53],
        [242, 218, 184],
    ])?;
    let effect = ErrorDiffusion::new(palette, DiffusionAlgorithm::FloydSteinberg)
        .with_scan(DiffusionScan::Serpentine)
        .with_pixel_size(2, 2)?;
    let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;
    let dithered = RgbaImage::from_raw(image_width, HEIGHT, rendered.rgba8_bytes().to_vec())
        .ok_or("rendered image dimensions do not match its pixel data")?;

    let mut banner = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([12, 11, 10, 255]));
    draw_dithered_image(&mut banner, &dithered);
    draw_text(&mut banner, "ditherlib", 90, 246, 17, [244, 225, 194, 255])?;
    draw_text(
        &mut banner,
        "image effects and dithering for Rust",
        92,
        407,
        4,
        [163, 189, 74, 255],
    )?;
    draw_rect(&mut banner, 92, 465, 72, 6, [163, 189, 74, 190]);
    draw_rect(&mut banner, 176, 465, 24, 6, [223, 113, 53, 180]);
    draw_text(
        &mut banner,
        &format!("v{}", options.version),
        92,
        738,
        3,
        [244, 225, 194, 56],
    )?;

    if let Some(parent) = options
        .output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let output = BufWriter::new(File::create(options.output)?);
    PngEncoder::new_with_quality(output, CompressionType::Best, PngFilterType::Adaptive)
        .write_image(banner.as_raw(), WIDTH, HEIGHT, ExtendedColorType::Rgba8)?;
    Ok(())
}

fn parse_options(arguments: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut version = None;
    let mut input = None;
    let mut output = None;
    let mut focus_x = 0.5;
    let mut focus_y = 0.5;
    let mut arguments = arguments.into_iter();

    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--version" => version = Some(value),
            "--input" => input = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--focus-x" => focus_x = parse_focus(&flag, &value)?,
            "--focus-y" => focus_y = parse_focus(&flag, &value)?,
            _ => return Err(format!("unknown option {flag}")),
        }
    }

    let version = version.ok_or("missing --version")?;
    if !valid_version(&version) {
        return Err("--version must contain three numeric components, such as 1.2.0".into());
    }
    let input = input.ok_or("missing --input")?;
    let output = output.ok_or("missing --output")?;
    if input == output {
        return Err("--input and --output must be different files".into());
    }
    if !output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err("--output must use a .png extension".into());
    }

    Ok(Options {
        version,
        input,
        output,
        focus_x,
        focus_y,
    })
}

fn parse_focus(flag: &str, value: &str) -> Result<f64, String> {
    let focus = value
        .parse::<f64>()
        .map_err(|_| format!("{flag} must be a number from 0.0 to 1.0"))?;
    if !(0.0..=1.0).contains(&focus) {
        return Err(format!("{flag} must be a number from 0.0 to 1.0"));
    }
    Ok(focus)
}

fn valid_version(version: &str) -> bool {
    let components: Vec<_> = version.split('.').collect();
    components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn crop_and_resize(
    image: RgbaImage,
    target_width: u32,
    target_height: u32,
    focus_x: f64,
    focus_y: f64,
) -> RgbaImage {
    let source_ratio = f64::from(image.width()) / f64::from(image.height());
    let target_ratio = f64::from(target_width) / f64::from(target_height);
    let (crop_width, crop_height) = if source_ratio > target_ratio {
        (
            (f64::from(image.height()) * target_ratio).round() as u32,
            image.height(),
        )
    } else {
        (
            image.width(),
            (f64::from(image.width()) / target_ratio).round() as u32,
        )
    };
    let centre_x = focus_x * f64::from(image.width());
    let centre_y = focus_y * f64::from(image.height());
    let x = (centre_x - f64::from(crop_width) / 2.0)
        .round()
        .clamp(0.0, f64::from(image.width() - crop_width)) as u32;
    let y = (centre_y - f64::from(crop_height) / 2.0)
        .round()
        .clamp(0.0, f64::from(image.height() - crop_height)) as u32;
    let cropped = crop_imm(&image, x, y, crop_width, crop_height).to_image();
    resize(&cropped, target_width, target_height, FilterType::Lanczos3)
}

fn draw_dithered_image(banner: &mut RgbaImage, image: &RgbaImage) {
    for y in 0..HEIGHT {
        for x in 0..image.width() {
            let destination_x = IMAGE_X + x;
            let progress = f64::from(destination_x - IMAGE_X) / f64::from(FADE_END_X - IMAGE_X);
            let cell_x = destination_x / 8;
            let cell_y = y / 8;
            let threshold =
                (f64::from(BAYER[((cell_y % 4) * 4 + cell_x % 4) as usize]) + 0.5) / 16.0;

            if destination_x >= FADE_END_X || progress >= threshold {
                banner.put_pixel(destination_x, y, *image.get_pixel(x, y));
            } else if threshold - progress < 0.12 && x % 8 < 4 && y % 8 < 4 {
                blend_pixel(banner.get_pixel_mut(destination_x, y), [151, 61, 29, 70]);
            }
        }
    }
}

fn draw_text(
    image: &mut RgbaImage,
    text: &str,
    x: u32,
    y: u32,
    scale: u32,
    colour: [u8; 4],
) -> Result<(), String> {
    let mut cursor = x;
    for character in text.chars() {
        let rows = glyph(character)
            .ok_or_else(|| format!("unsupported banner character {character:?}"))?;
        for (row, bits) in rows.into_iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    draw_rect(
                        image,
                        cursor + column * scale,
                        y + row as u32 * scale,
                        scale,
                        scale,
                        colour,
                    );
                }
            }
        }
        cursor += 6 * scale;
    }
    Ok(())
}

fn draw_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, colour: [u8; 4]) {
    for pixel_y in y..(y + height).min(image.height()) {
        for pixel_x in x..(x + width).min(image.width()) {
            blend_pixel(image.get_pixel_mut(pixel_x, pixel_y), colour);
        }
    }
}

fn blend_pixel(destination: &mut Rgba<u8>, source: [u8; 4]) {
    let alpha = u32::from(source[3]);
    let inverse = 255 - alpha;
    for channel in 0..3 {
        destination[channel] =
            ((u32::from(source[channel]) * alpha + u32::from(destination[channel]) * inverse + 127)
                / 255) as u8;
    }
    destination[3] = 255;
}

fn glyph(character: char) -> Option<[u8; 7]> {
    Some(match character {
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0, 0b00100],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'a' => [0, 0b01110, 0b00001, 0b01111, 0b10001, 0b10011, 0b01101],
        'b' => [
            0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'c' => [0, 0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'd' => [
            0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111,
        ],
        'e' => [0, 0b01110, 0b10001, 0b11111, 0b10000, 0b10000, 0b01111],
        'f' => [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
        ],
        'g' => [0, 0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'h' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001,
        ],
        'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'm' => [0, 0b11011, 0b10101, 0b10101, 0b10101, 0b10101, 0b10101],
        'n' => [0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001, 0b10001],
        'o' => [0, 0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'r' => [0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000, 0b10000],
        's' => [0, 0b01111, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        't' => [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
        ],
        'u' => [0, 0b10001, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_release_versions_and_banner_text() {
        assert!(valid_version("1.0.0"));
        assert!(valid_version("12.34.56"));
        assert!(!valid_version("1.0"));
        assert!(!valid_version("1.0.0-beta"));

        for character in "ditherlib image effects and dithering for Rust v1.2.0".chars() {
            assert!(
                glyph(character).is_some(),
                "missing glyph for {character:?}"
            );
        }
    }

    #[test]
    fn crops_to_the_requested_dimensions() {
        let source = RgbaImage::new(20, 40);
        let result = crop_and_resize(source, 14, 8, 0.5, 0.3);
        assert_eq!(result.dimensions(), (14, 8));
    }
}
