# Ditherlib

Ditherlib is a Rust library for non-destructive image effects and dithering.
Effects can target an entire image or anti-aliased polygon selections, and an
ordered pipeline can combine several selected effects into one render.

Source pixels remain immutable after loading. Each render starts from the
source and produces a separately owned image, so callers can freely edit and
re-render pipelines.

## Features

- Greyscale and Gaussian blur
- Threshold and ordered Bayer dithering
- Floyd-Steinberg, Atkinson, Jarvis-Judice-Ninke, Stucki, Burkes, Sierra,
  Two-Row Sierra, and Sierra Lite error diffusion
- Raster and serpentine error-diffusion scanning
- Custom RGB palettes, black-and-white palettes, and monochrome palettes
- Configurable logical pixel sizes for every dithering method
- Whole-image and polygon selections with anti-aliased edges
- Ordered multi-effect pipelines with reusable rendering buffers
- Crate-owned image, result, and error types

## Installation

JPEG and PNG support are enabled by default:

```toml
[dependencies]
ditherlib = "0.3"
```

Codec features can be selected individually:

```toml
[dependencies]
ditherlib = { version = "0.3", default-features = false, features = ["png", "webp"] }
```

Available codec features are `avif`, `bmp`, `dds`, `exr`, `ff`, `gif`, `hdr`,
`ico`, `jpeg`, `png`, `pnm`, `qoi`, `tga`, `tiff`, and `webp`. The processing
algorithms compile without any codec features.

## Usage

This example converts the full image to greyscale, then applies a red
monochrome ordered dither inside a polygon:

```rust,no_run
use ditherlib::{
    Greyscale, OrderedDither, Palette, Pipeline, Point, Polygon, Renderer,
    Selection, read, write,
};

fn main() -> ditherlib::Result<()> {
    let source = read("input.jpg")?;
    let (width, height) = source.dimensions();
    let area = Polygon::new([
        Point::new(width as f32 * 0.20, height as f32 * 0.20),
        Point::new(width as f32 * 0.80, height as f32 * 0.20),
        Point::new(width as f32 * 0.80, height as f32 * 0.80),
        Point::new(width as f32 * 0.20, height as f32 * 0.80),
    ])?;

    let mut pipeline = Pipeline::new();
    pipeline.add(Greyscale, Selection::All);
    pipeline.add(
        OrderedDither::new(Palette::monochrome([220, 20, 60]), 4)?
            .with_pixel_size(4)?,
        Selection::Polygon(area),
    );

    let rendered = Renderer::new().render_pipeline(&source, &pipeline)?;
    write("output.png", &rendered)
}
```

Pipeline order matters. Each step receives the result of the previous step,
including where polygon selections overlap. Rendering the same pipeline again
always begins from the unchanged `SourceImage`.

## Palettes

`Palette::black_and_white()` provides the familiar two-colour palette.
`Palette::monochrome(colour)` combines black, the supplied RGB colour, and
white. `Palette::new` accepts any non-empty collection of RGB colours.

## Error diffusion

`ErrorDiffusion` provides all diffusion presets and supports raster or
serpentine scanning.

```rust,no_run
use ditherlib::{
    DiffusionAlgorithm, DiffusionScan, ErrorDiffusion, Palette, Renderer,
    Selection, read,
};

# fn run() -> ditherlib::Result<()> {
let source = read("input.png")?;
let effect = ErrorDiffusion::new(Palette::black_and_white(), DiffusionAlgorithm::Stucki)
    .with_scan(DiffusionScan::Serpentine)
    .with_pixel_size(2)?;
let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;
# let _ = rendered;
# Ok(())
# }
```

## Errors

Fallible operations return `ditherlib::Result<T>`. Use `DitherError::kind()`
to handle stable `ErrorKind` categories covering file access, unsupported
formats, decoding, encoding, invalid geometry, dimension mismatches, invalid
parameters, and effect failures. Dependency errors may remain available
through the standard `Error::source()` method for diagnostics.

JPEG output discards alpha because the format does not support transparency.

## Examples

Every example accepts an input path and output path:

```sh
cargo run --release --example pipeline -- input.jpg output.png
cargo run --release --example polygon_pipeline -- input.jpg output.png
cargo run --release --example diffusion_comparison -- input.jpg first.png second.png
```

The diffusion comparison requires an image with even dimensions. `first.png`
uses Floyd-Steinberg, Atkinson, Jarvis-Judice-Ninke, and Stucki from top-left
to bottom-right. `second.png` uses Burkes, Sierra, Two-Row Sierra, and Sierra
Lite in the same order.

Additional examples cover each built-in effect in the [`examples`](./examples/)
directory.
