# Changelog

## 0.4.3

### Added

- Added False Floyd-Steinberg, Fan, Shiau-Fan, Shiau-Fan 2,
  Stevenson-Arce, and Two-dimensional Knuth error-diffusion presets.
- Expanded the diffusion comparison to seven evenly divided images, giving
  every built-in preset half of a full-size input image.
- Documented the visual character of every built-in diffusion preset.

## 0.4.2

### Fixed

- Excluded image assets and test fixtures from the published crate package.

## 0.4.1

### Added

- Added rectangle, centred-square, and regular-polygon convenience constructors.
- Added an attributed sample image and README showcase.

## 0.4.0

### Added

- Added validated custom error-diffusion kernels with public taps, weights,
  offsets, and divisors.
- Added configurable diffusion strength from zero through double strength.
- Added optional per-channel error clamping.
- Added a two-image comparison example for diffusion strength and clamping.

### Changed

- `ErrorDiffusion::new` now accepts either a `DiffusionAlgorithm` preset or a
  custom `DiffusionKernel` and is no longer a `const` function.
- `ErrorDiffusion::algorithm` now returns `Option<DiffusionAlgorithm>` because
  custom kernels do not have a built-in preset name.
- Preset defaults retain their 0.3.1 rendered output and optimised path.

## 0.3.1

### Added

- Added the `Colour` RGB type, its `Color` spelling alias, common colour
  constants, array conversions, palette iteration, and structured nearest
  colour matching.
- Added eight-level greyscale, Game Boy, CGA, and PICO-8 palette presets.
- Allowed custom and monochrome palettes to be created from `Colour` values or
  RGB byte arrays.
- Added a four-quadrant example comparing every prefab palette under the same
  diffusion settings.

## 0.3.0

### Added

- Added configurable error diffusion with Floyd-Steinberg, Atkinson,
  Jarvis-Judice-Ninke, Stucki, Burkes, Sierra, Two-Row Sierra, and Sierra Lite
  presets.
- Added raster and serpentine scan modes for configurable error diffusion.
- Added a two-image comparison example covering every diffusion preset in
  equal-sized polygon quadrants.

### Removed

- Replaced the standalone `FloydSteinberg` and `Atkinson` effects with their
  `DiffusionAlgorithm` presets on `ErrorDiffusion`.

## 0.2.0

### Performance

- Cached polygon scanline calculations, reducing 1920×1080 polygon mask
  rasterisation from 160.2 ms to 68.4 ms, a 2.34× speedup.
- Precomputed Bayer adjustments, improving 1920×1080 ordered dithering by
  3.78× with a 4×4 matrix and 5.55× with an 8×8 matrix.
- Added an exact fast path for black-and-white palettes, improving threshold
  dithering by 1.41×, Floyd–Steinberg by 1.32×, and Atkinson by 1.28× at
  1920×1080.
- Blurred RGB data without processing discarded alpha values, reducing blur
  time by 1.23× and its internal image buffers by 25%.
- Reduced the measured 1920×1080 whole-image pipeline from 74.0 ms to 47.3 ms
  and the polygon pipeline from 528.6 ms to 237.4 ms.

Measurements are 15-sample medians from an Apple M2 using Rust 1.98.1. The
optimisations preserve the public API, rendered output, selection edge rules,
and source alpha values.
