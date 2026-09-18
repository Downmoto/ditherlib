# Banner generator

This tool creates Ditherlib's versioned README banners. It uses Ditherlib for
the copper error-diffusion treatment and draws the layout and bundled pixel
type deterministically.

From the repository root, run:

```sh
cargo run --release --locked --manifest-path tools/banner/Cargo.toml -- \
  --version 1.0.0 \
  --input assets/samurai.jpg \
  --output assets/ditherlib_banner_v1.0.0.png \
  --focus-y 0.30
```

`--focus-x` and `--focus-y` select the focal point of the source image using
values from `0.0` to `1.0`, measured from its top-left corner. Both default to
`0.5`.

For each release:

1. Add a new, properly licensed source image and record its attribution in
   `assets/README.md`.
2. Generate a banner whose filename contains the release version.
3. Update the banner URL in the root README.
4. Retain earlier versioned banners so published README files keep working.

The output is a 2400x800 PNG. The version passed to `--version` is printed
faintly in the bottom-left corner. To reproduce an older banner exactly, check
out that release's Git tag and run its documented command with `--locked`.
