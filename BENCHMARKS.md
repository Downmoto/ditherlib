# Benchmarks

Ditherlib uses [Criterion](https://bheisler.github.io/criterion.rs/book/) to
measure effect rendering, composition, palette operations, and renderer reuse.
The suite is deterministic and uses only in-memory image data.

## Running the suite

Compile the benchmark target without running it:

```console
cargo bench --no-run
```

Run the complete suite:

```console
cargo bench
```

Run one group by passing a filter after `--`:

```console
cargo bench -- ordered_pixel_size
```

Criterion stores local reports and measurements in `target/criterion`. To save
and compare a machine-local baseline:

```console
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

The complete HTML report is written to `target/criterion/report/index.html`.
Open that file in a browser after the run finishes.

Do not compare absolute timings from different machines. Compiler version,
target architecture, power state, and background work can all affect results.

## Harness configuration

The suite uses Criterion 0.8 with HTML reports enabled through the Plotters
backend and its optional parallel executor disabled. Each benchmark uses:

| Setting | Value |
| --- | ---: |
| Sample size | 10 |
| Warm-up time | 100 ms |
| Measurement time | 250 ms |
| Cargo profile | `bench` (optimised) |

The intentionally short measurement window keeps all 286 cases practical for
a complete local run. Criterion may extend a case when the requested sample
count cannot fit in that window.

## Fixtures and measurement boundary

The fixture generator creates opaque RGBA8 images in memory. Red and green are
horizontal and vertical gradients. Blue is derived from the pixel coordinates
with wrapping integer arithmetic. This gives every run identical input with
both smooth and high-frequency content.

Baseline effect rendering is measured at `64x64`, `256x256`, and `1024x1024`.
Option matrices use `128x128` so the suite can sample every configuration
without making a full run excessively long.

Sources, effects, selections, and pipelines are constructed outside timed
iterations. A timed effect iteration calls `Renderer::render`; a timed pipeline
iteration calls `Renderer::render_pipeline`. Both include selection
rasterisation, effect evaluation, working-buffer copies, and creation of the
owned output image. Palette construction benchmarks intentionally time palette
creation. The `renderer_reuse/fresh` case intentionally includes renderer
creation and its first buffer allocations.

Each option changes independently from a representative baseline. This makes
regressions attributable to one setting and avoids an unhelpful Cartesian
product of unrelated parameters.

## Benchmark matrix

| Group | Coverage |
| --- | --- |
| `effects_by_resolution` | Greyscale, blur, threshold, ordered, noise, halftone, colour halftone, Floyd-Steinberg, Ostromoukhov, and Riemersma at all three resolutions |
| `blur_sigma` | `0`, `2`, and `16` |
| `threshold_*` | Pixel size, all sampling modes, and grid offsets |
| `ordered_map` | Every built-in threshold map and a custom rectangular map |
| `ordered_*` | Pixel size, sampling, strength, rotation, every mirroring combination, map offset, and grid offset |
| `noise_*` | Both algorithms, pixel size, sampling, strength, seed, and grid offset |
| `halftone_*` | Every shape, cell size, angle, phase, and scale |
| `colour_halftone_*` | Both modes, every shape, cell size, scale, both CMYK presets, and angle and offset samples for every RGB and CMYK channel |
| `error_diffusion_kernel` | Every built-in diffusion preset and a custom kernel |
| `error_diffusion_*` | Both scan modes, both error modes, pixel size, sampling, strength, error clamp, and grid offset |
| `ostromoukhov_*` | Pixel size, sampling, and grid offset |
| `riemersma_*` | Pixel size, sampling, history length, decay, and grid offset |
| `palette_construction` | Custom palettes, all source colours, and limited source palettes of 1, 8, and 64 colours |
| `palette_matching` | RGB, linear RGB, and Oklab crossed with colour and luminance matching |
| `palette_presets` | Black and white, greyscale, Game Boy, CGA, PICO-8, monochrome, custom, and source-derived palettes |
| `selections` | `Selection::All`, `Selection::Polygon`, and `Selection::Mask` |
| `pipelines` | Empty, 1-step, 3-step, and 6-step pipelines |
| `renderer_reuse` | Reused renderer buffers compared with a fresh renderer per iteration |

Logical pixel and halftone cell matrices include `1x1`, `2x2`, `4x4`, `8x8`,
and `4x8`. These cases exercise both square and rectangular geometry. Numeric
options otherwise use boundary, default, and representative high values, with
negative values included where offsets or phases permit them.

The suite explicitly exercises every current variant of:

- `SamplingMode`
- `ThresholdRotation`
- `NoiseAlgorithm`
- `HalftoneShape`
- `ColourHalftoneMode`
- `HalftoneChannel`
- `CmykScreenPreset`
- `DiffusionAlgorithm`
- `DiffusionScan`
- `DiffusionErrorMode`
- `ColourSpace`
- `PaletteMatchMode`
- `PaletteSize`
- `Selection`

## Representative baselines

The resolution matrix uses these stable configurations:

| Effect | Configuration |
| --- | --- |
| Greyscale | Default unit effect |
| Blur | Sigma `2` |
| Threshold | Black-and-white palette, `1x1` pixels, average sampling |
| Ordered | Black-and-white palette, Bayer 4x4 map |
| Noise | Black-and-white palette, blue noise, seed `0` |
| Halftone | Black-and-white palette, circle, `8x8` cells |
| Colour halftone | CMYK, circle, `8x8` cells |
| Error diffusion | Black-and-white palette, Floyd-Steinberg, raster scan |
| Ostromoukhov | Black-and-white palette, `1x1` pixels |
| Riemersma | Black-and-white palette, history `16`, decay `16` |

## Reviewing results

After a complete run, check that every group above appears. Investigate large
changes against a saved baseline, especially when neighbouring parameter cases
remain stable. Very small effects at `1x1` may be dominated by output allocation
and buffer copies, while large cells can reduce the amount of per-cell work.
Review the measurement boundary before moving setup into a timed closure or
adding new fixture construction.
