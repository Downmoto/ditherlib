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
- Custom diffusion kernels, strength, clamping, and scan direction
- Custom RGB palettes, black-and-white palettes, and monochrome palettes
- Configurable logical pixel sizes for every dithering method
- Whole-image and polygon selections with anti-aliased edges
- Ordered multi-effect pipelines with reusable rendering buffers
- Crate-owned image, result, and error types

## Installation

JPEG and PNG support are enabled by default:

```toml
[dependencies]
ditherlib = "0.4"
```

Codec features can be selected individually:

```toml
[dependencies]
ditherlib = { version = "0.4", default-features = false, features = ["png", "webp"] }
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
white. Eight-level greyscale, Game Boy, CGA, and PICO-8 palettes are also
available through `Palette::greyscale()`, `Palette::game_boy()`,
`Palette::cga()`, and `Palette::pico_8()`.

`Colour` represents an RGB value with named channels, common colour constants,
and conversions to and from `[u8; 3]`. `Color` is an alias for callers using
American spelling. `Palette::new` accepts either representation:

```rust
use ditherlib::{Colour, Palette};

fn main() -> ditherlib::Result<()> {
    let palette = Palette::new([
        Colour::BLACK,
        Colour::new(220, 20, 60),
        Colour::WHITE,
    ])?;
    assert_eq!(palette.colours().len(), 3);
    Ok(())
}
```

## Error diffusion

`ErrorDiffusion` accepts a built-in algorithm or a validated custom kernel. It
supports raster or serpentine scanning, diffusion strength from `0.0` through
`2.0`, and optional per-channel error clamping.

```rust,no_run
use ditherlib::{
    DiffusionAlgorithm, DiffusionScan, ErrorDiffusion, Palette, Renderer,
    Selection, read, write,
};

fn main() -> ditherlib::Result<()> {
    let source = read("input.png")?;
    let effect = ErrorDiffusion::new(Palette::black_and_white(), DiffusionAlgorithm::Stucki)
        .with_scan(DiffusionScan::Serpentine)
        .with_pixel_size(2)?;
    let rendered = Renderer::new().render(&source, &effect, &Selection::All)?;
    write("output.png", &rendered)
}
```

Custom kernels contain forward-pointing weighted taps and a divisor:

```rust
use ditherlib::{DiffusionKernel, DiffusionTap, ErrorDiffusion, Palette};

fn main() -> ditherlib::Result<()> {
    let kernel = DiffusionKernel::new(
        [
            DiffusionTap::new(1, 0, 7),
            DiffusionTap::new(-1, 1, 3),
            DiffusionTap::new(0, 1, 5),
            DiffusionTap::new(1, 1, 1),
        ],
        16,
    )?;
    let effect = ErrorDiffusion::new(Palette::black_and_white(), kernel)
        .with_strength(0.75)?
        .with_error_clamp(48);
    assert_eq!(effect.strength(), 0.75);
    Ok(())
}
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
cargo run --release --example diffusion_controls -- input.jpg strengths.png clamps.png
cargo run --release --example palette_comparison -- input.jpg palettes.png
```

The diffusion comparison requires an image with even dimensions. `first.png`
uses Floyd-Steinberg, Atkinson, Jarvis-Judice-Ninke, and Stucki from top-left
to bottom-right. `second.png` uses Burkes, Sierra, Two-Row Sierra, and Sierra
Lite in the same order.

The diffusion controls example creates two quadrant comparisons. `strengths.png`
uses strengths 0.0, 0.5, 1.0, and 1.5 from top-left to bottom-right.
`clamps.png` uses error limits of 0, 24, 64, and unlimited in the same order.

The palette comparison uses greyscale, Game Boy, CGA, and PICO-8 from top-left
to bottom-right. It applies Floyd-Steinberg diffusion with serpentine scanning
to every quadrant so the palette is the only variable.

Additional examples cover each built-in effect in the [`examples`](./examples/)
directory.
