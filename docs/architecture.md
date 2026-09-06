# Ditherlib Architecture

## Goals

Ditherlib is an image-processing library focused on non-destructive effects and dithering. It should support effects across an entire image or within selected polygons while keeping the originally loaded pixels unchanged.

The initial design favours a small, predictable API. Layers, GPU processing, tiled rendering, pipeline serialisation, and operation graphs can be considered when concrete use cases require them.

## Processing Model

```text
Image file
    |
    v
Immutable source image
    |
    v
Ordered pipeline
    |-- effect + whole-image selection
    |-- effect + polygon selection
    `-- effect + polygon selection
    |
    v
Renderer with working buffers
    |
    v
Rendered image
    |
    v
Caller or encoded image file
```

The architecture has three central parts:

1. An immutable source image owns the original decoded pixels.
2. A pipeline describes an ordered collection of effects and selections.
3. A renderer evaluates the pipeline into replaceable working buffers.

Changing the pipeline causes a new render from the source. The source pixels are never exposed through mutable access.

## Image Representation

The first version will normalise decoded images to 8-bit RGBA. A single internal representation keeps effect implementations and buffer management straightforward.

Colour effects operate on RGB channels by default. Alpha remains unchanged unless an effect explicitly documents alpha processing. Support for 16-bit and floating-point images can be added after a demonstrated need.

The immutable source may be cheaply shared through reference counting. A rendered image owns a separate pixel buffer.

## Pipeline

A pipeline is an ordered list of steps. Each step contains:

- An effect and its configuration
- A selection describing where the result will be applied

The pipeline stores instructions rather than intermediate images. Removing a step provides undo behaviour, restoring it provides redo behaviour, and rendering always starts from the immutable source.

Pipeline editing and history management remain separate concerns. Applications can maintain their own previous pipeline states without the library retaining a snapshot for every operation.

## Renderer

The renderer uses two same-sized working buffers:

- `current` initially contains a copy of the source.
- `scratch` receives the result of the next effect.

For each pipeline step, the renderer:

1. Resolves the selection into a mask for the current image dimensions.
2. Gives the effect read-only access to `current` and writable access to `scratch`.
3. Ensures pixels outside the selection remain unchanged.
4. Swaps `current` and `scratch`.

Alternating buffers avoids allocating a new full image for every step. Effects always read from a stable input buffer, which prevents scan order from accidentally changing their source pixels.

## Selections and Masks

The first selection variants are:

- The entire image
- A polygon

Polygon vertices use floating-point image coordinates. A polygon is rasterised into a mask matching the image dimensions. Each mask value represents pixel coverage:

- `0` is outside the selection.
- `255` is fully selected.
- Intermediate values provide anti-aliased edges.

The mask is independent of the effect, allowing one selection to be reused for greyscale, blur, and dithering. Rasterised masks may be cached within a render when the same selection and dimensions are reused.

Future selection variants may include bitmap masks, inversion, feathering, and boolean combinations. These additions should preserve the same mask interface.

## Effects

An effect receives:

- A read-only input buffer
- A writable output buffer
- A read-only selection mask
- Its validated configuration

The renderer owns the rule that pixels outside the mask remain unchanged. An effect can still inspect the mask when its algorithm depends on selection boundaries.

Built-in effects are grouped by responsibility:

```text
effects/
    greyscale
    blur
    dither
```

A public effect interface becomes appropriate once multiple built-in effects share the rendering contract. This also permits callers to implement custom effects without changing the renderer.

## Dithering

Dithering is divided into three decisions:

- The target palette
- The colour-distance calculation used to choose a palette entry
- The spatial dithering method

Initial spatial methods can include threshold, ordered Bayer dithering, Floyd-Steinberg error diffusion, and Atkinson error diffusion. Configuration should use ordinary structs and enums until external implementations require additional extension points.

Noise-based methods must accept an explicit seed so renders remain reproducible.

For polygon selections, error diffusion follows these boundary rules:

- Only selected pixels are quantised.
- Quantisation error propagates only to selected neighbours.
- Pixels outside the selection retain their previous values.

These rules make each selected polygon an isolated dithering region.

## File Formats

File decoding and encoding belong to the I/O boundary. Codec support is controlled through Cargo features forwarded to the `image` dependency. JPEG and PNG are enabled by default, while other formats remain individually selectable.

Image-processing algorithms do not depend on a particular file format and should compile with all codec features disabled.

## Proposed Module Layout

```text
src/
    lib.rs
    io.rs
    source.rs
    pipeline.rs
    renderer.rs
    selection.rs
    effects/
        mod.rs
        greyscale.rs
        blur.rs
        dither.rs
```

- `io` handles feature-gated decoding and encoding.
- `source` owns the immutable source representation.
- `pipeline` defines ordered effect steps.
- `renderer` manages buffers and evaluates steps.
- `selection` defines polygons and rasterised masks.
- `effects` contains image transformations.

## Architectural Invariants

Tests should continuously enforce these guarantees:

- Rendering never modifies the source buffer.
- An empty pipeline produces pixels identical to the source.
- Image dimensions remain stable through effects unless a future effect explicitly changes them.
- Pixels outside a selection remain byte-for-byte unchanged.
- A mask always has the same dimensions as its target image.
- Seeded effects produce deterministic results.
- Error diffusion never crosses a selection boundary.
