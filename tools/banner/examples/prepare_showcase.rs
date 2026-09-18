use std::{env, error::Error, fs::File, io::BufWriter, path::Path, process};

use image::{
    ExtendedColorType, ImageEncoder,
    codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder},
    imageops::{FilterType, crop_imm, resize},
};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 800;
const FOCUS_Y: f64 = 0.30;

fn main() {
    if let Err(error) = run() {
        eprintln!("prepare-showcase: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments.next().ok_or("missing input path")?;
    let output = arguments.next().ok_or("missing output path")?;
    if arguments.next().is_some() {
        return Err("expected only input and output paths".into());
    }

    let image = image::open(input)?.to_rgba8();
    let crop_height =
        (f64::from(image.width()) * f64::from(HEIGHT) / f64::from(WIDTH)).round() as u32;
    if crop_height > image.height() {
        return Err("input image is too wide for the showcase crop".into());
    }
    let centre_y = FOCUS_Y * f64::from(image.height());
    let crop_y = (centre_y - f64::from(crop_height) / 2.0)
        .round()
        .clamp(0.0, f64::from(image.height() - crop_height)) as u32;
    let cropped = crop_imm(&image, 0, crop_y, image.width(), crop_height).to_image();
    let resized = resize(&cropped, WIDTH, HEIGHT, FilterType::Lanczos3);

    if let Some(parent) = Path::new(&output)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let output = BufWriter::new(File::create(output)?);
    PngEncoder::new_with_quality(output, CompressionType::Best, PngFilterType::Adaptive)
        .write_image(resized.as_raw(), WIDTH, HEIGHT, ExtendedColorType::Rgba8)?;
    Ok(())
}
