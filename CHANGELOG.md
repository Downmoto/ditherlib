# Changelog

## 0.3.0 - 2026-09-08

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

## 0.2.0 - 2026-09-08

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
