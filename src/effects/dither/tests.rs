use super::{
    CmykScreenPreset, Color, Colour, ColourHalftone, ColourHalftoneMode, ColourSpace,
    DiffusionAlgorithm, DiffusionErrorMode, DiffusionKernel, DiffusionScan, DiffusionTap,
    ErrorDiffusion, Halftone, HalftoneChannel, HalftoneShape, NoiseAlgorithm, NoiseDither,
    OrderedDither, OstromoukhovDither, Palette, PaletteMatchMode, PaletteSize, RiemersmaDither,
    SamplingMode, Threshold, ThresholdMap, ThresholdRotation,
    cells::sample_cell,
    colour::colour_components,
    halftone::{cmyk_to_rgb, rgb_to_cmyk},
    noise::blue_noise,
    ostromoukhov::published_coefficients,
    riemersma::hilbert_cells,
};
use crate::{Effect, ErrorKind, Mask, Point, Polygon, Renderer, Selection, SourceImage};

/// Creates an immutable image from test pixels.
fn source(width: u32, height: u32, pixels: &[[u8; 4]]) -> SourceImage {
    SourceImage {
        width,
        height,
        pixels: pixels
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

fn diffusion(algorithm: DiffusionAlgorithm) -> ErrorDiffusion {
    ErrorDiffusion::new(Palette::black_and_white(), algorithm)
}

#[test]
fn renders_exact_pixels_for_every_diffusion_preset() {
    let pixels = (0..35)
        .map(|index| {
            let value = ((index * 47 + index * index * 3) % 256) as u8;
            [value, value.wrapping_add(53), value.wrapping_mul(3), 200]
        })
        .collect::<Vec<_>>();
    let source = source(7, 5, &pixels);
    // These maps were recorded after checking each preset's taps and divisor.
    // `#` is black, `.` is white, and every source alpha is 200.
    let cases = [
        (
            DiffusionAlgorithm::FloydSteinberg,
            ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
        ),
        (
            DiffusionAlgorithm::Atkinson,
            ["###..#.", "...##.#", "##..#.#", "#.##..#", ".##.##."],
        ),
        (
            DiffusionAlgorithm::JarvisJudiceNinke,
            ["###..#.", ".#.##.#", ".#..#.#", "#.##...", ".##.##."],
        ),
        (
            DiffusionAlgorithm::Stucki,
            ["##...#.", ".#.##.#", "##..#.#", "#.##.#.", ".##.##."],
        ),
        (
            DiffusionAlgorithm::Burkes,
            ["##..#..", ".#.##.#", "#..##.#", "#.##...", ".##.##."],
        ),
        (
            DiffusionAlgorithm::Sierra,
            ["###..#.", "...##.#", "##..#.#", "#.##..#", ".##.##."],
        ),
        (
            DiffusionAlgorithm::TwoRowSierra,
            ["##..#..", ".#.##.#", "##..#.#", "#.##..#", ".##..#."],
        ),
        (
            DiffusionAlgorithm::SierraLite,
            ["##..#.#", ".#.#.#.", ".#.##.#", "#.#.#..", ".##..#."],
        ),
        (
            DiffusionAlgorithm::FalseFloydSteinberg,
            ["##..#.#", ".#.##.#", ".#..#.#", "####...", "..#.##."],
        ),
        (
            DiffusionAlgorithm::Fan,
            ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
        ),
        (
            DiffusionAlgorithm::ShiauFan,
            ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##.#.#"],
        ),
        (
            DiffusionAlgorithm::ShiauFan2,
            ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
        ),
        (
            DiffusionAlgorithm::StevensonArce,
            ["###..#.", ".#.##.#", ".#..#.#", "#.##...", ".##.##."],
        ),
        (
            DiffusionAlgorithm::TwoDimensionalKnuth,
            ["##..#.#", ".#.#.#.", "#.#.#.#", "#.##.#.", ".##..#."],
        ),
    ];

    for (algorithm, rows) in cases {
        let rendered = Renderer::new()
            .render(&source, &diffusion(algorithm), &Selection::All)
            .unwrap();
        let expected = rows
            .concat()
            .bytes()
            .flat_map(|pixel| match pixel {
                b'#' => [0, 0, 0, 200],
                b'.' => [255, 255, 255, 200],
                _ => unreachable!("pixel maps contain only `#` and `.`"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rendered.rgba8_bytes(),
            expected,
            "changed output for {algorithm:?}"
        );
    }
}

#[test]
fn exposes_verified_0_4_3_diffusion_weights() {
    let specifications: &[(DiffusionAlgorithm, &[DiffusionTap], u32)] = &[
        (
            DiffusionAlgorithm::FalseFloydSteinberg,
            &[
                DiffusionTap::new(1, 0, 3),
                DiffusionTap::new(-1, 1, 3),
                DiffusionTap::new(0, 1, 2),
            ],
            8,
        ),
        (
            DiffusionAlgorithm::Fan,
            &[
                DiffusionTap::new(1, 0, 7),
                DiffusionTap::new(-2, 1, 1),
                DiffusionTap::new(-1, 1, 3),
                DiffusionTap::new(0, 1, 5),
            ],
            16,
        ),
        (
            DiffusionAlgorithm::ShiauFan,
            &[
                DiffusionTap::new(1, 0, 4),
                DiffusionTap::new(-2, 1, 1),
                DiffusionTap::new(-1, 1, 1),
                DiffusionTap::new(0, 1, 2),
            ],
            8,
        ),
        (
            DiffusionAlgorithm::ShiauFan2,
            &[
                DiffusionTap::new(1, 0, 8),
                DiffusionTap::new(-3, 1, 1),
                DiffusionTap::new(-2, 1, 1),
                DiffusionTap::new(-1, 1, 2),
                DiffusionTap::new(0, 1, 4),
            ],
            16,
        ),
        (
            DiffusionAlgorithm::StevensonArce,
            &[
                DiffusionTap::new(2, 0, 32),
                DiffusionTap::new(-3, 1, 12),
                DiffusionTap::new(-1, 1, 26),
                DiffusionTap::new(1, 1, 30),
                DiffusionTap::new(3, 1, 16),
                DiffusionTap::new(-2, 2, 12),
                DiffusionTap::new(0, 2, 26),
                DiffusionTap::new(2, 2, 12),
                DiffusionTap::new(-3, 3, 5),
                DiffusionTap::new(-1, 3, 12),
                DiffusionTap::new(1, 3, 12),
                DiffusionTap::new(3, 3, 5),
            ],
            200,
        ),
        (
            DiffusionAlgorithm::TwoDimensionalKnuth,
            &[DiffusionTap::new(1, 0, 1), DiffusionTap::new(0, 1, 1)],
            2,
        ),
    ];

    for (algorithm, taps, divisor) in specifications {
        let kernel = algorithm.kernel();
        assert_eq!(kernel.taps(), *taps, "wrong weights for {algorithm:?}");
        assert_eq!(
            kernel.divisor(),
            *divisor,
            "wrong divisor for {algorithm:?}"
        );
    }
}

#[test]
fn validates_and_exposes_custom_diffusion_kernels() {
    let taps = [
        DiffusionTap::new(1, 0, 7),
        DiffusionTap::new(-1, 1, 3),
        DiffusionTap::new(0, 1, 5),
        DiffusionTap::new(1, 1, 1),
    ];
    let kernel = DiffusionKernel::new(taps, 16).unwrap();
    assert_eq!(kernel.taps(), taps);
    assert_eq!(kernel.divisor(), 16);
    assert_eq!(kernel.preset(), None);
    assert_eq!(kernel.taps()[0].offset_x(), 1);
    assert_eq!(kernel.taps()[0].offset_y(), 0);
    assert_eq!(kernel.taps()[0].weight(), 7);

    let preset = DiffusionAlgorithm::FloydSteinberg.kernel();
    assert_eq!(preset.taps(), taps);
    assert_eq!(preset.divisor(), 16);
    assert_eq!(preset.preset(), Some(DiffusionAlgorithm::FloydSteinberg));

    let invalid = [
        DiffusionKernel::new(Vec::<DiffusionTap>::new(), 1).unwrap_err(),
        DiffusionKernel::new([DiffusionTap::new(1, 0, 1)], 0).unwrap_err(),
        DiffusionKernel::new([DiffusionTap::new(1, 0, 0)], 1).unwrap_err(),
        DiffusionKernel::new([DiffusionTap::new(0, 0, 1)], 1).unwrap_err(),
        DiffusionKernel::new([DiffusionTap::new(-1, 0, 1)], 1).unwrap_err(),
        DiffusionKernel::new([DiffusionTap::new(0, -1, 1)], 1).unwrap_err(),
    ];
    assert!(
        invalid
            .iter()
            .all(|error| error.kind() == ErrorKind::InvalidParameter)
    );
}

#[test]
fn custom_kernel_matches_its_builtin_equivalent() {
    let source = source(
        6,
        2,
        &[
            [20, 40, 60, 1],
            [70, 90, 110, 2],
            [120, 140, 160, 3],
            [170, 190, 210, 4],
            [220, 240, 250, 5],
            [100, 130, 160, 6],
            [210, 180, 150, 7],
            [160, 130, 100, 8],
            [110, 80, 50, 9],
            [60, 30, 10, 10],
            [130, 170, 210, 11],
            [230, 190, 150, 12],
        ],
    );
    let custom = DiffusionKernel::new(
        [
            DiffusionTap::new(1, 0, 7),
            DiffusionTap::new(-1, 1, 3),
            DiffusionTap::new(0, 1, 5),
            DiffusionTap::new(1, 1, 1),
        ],
        16,
    )
    .unwrap();
    for scan in [DiffusionScan::Raster, DiffusionScan::Serpentine] {
        let builtin = Renderer::new()
            .render(
                &source,
                &diffusion(DiffusionAlgorithm::FloydSteinberg).with_scan(scan),
                &Selection::All,
            )
            .unwrap();
        let rendered = Renderer::new()
            .render(
                &source,
                &ErrorDiffusion::new(Palette::black_and_white(), custom.clone()).with_scan(scan),
                &Selection::All,
            )
            .unwrap();

        assert_eq!(rendered.rgba8_bytes(), builtin.rgba8_bytes());
    }
}

#[test]
fn configures_diffusion_strength_and_error_clamping() {
    let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
    let threshold = Renderer::new()
        .render(
            &source,
            &Threshold::new(Palette::black_and_white()),
            &Selection::All,
        )
        .unwrap();
    let no_diffusion = diffusion(DiffusionAlgorithm::FloydSteinberg)
        .with_strength(0.0)
        .unwrap();
    let no_error = diffusion(DiffusionAlgorithm::FloydSteinberg).with_error_clamp(0);

    for effect in [&no_diffusion, &no_error] {
        let rendered = Renderer::new()
            .render(&source, effect, &Selection::All)
            .unwrap();
        assert_eq!(rendered.rgba8_bytes(), threshold.rgba8_bytes());
    }

    let configured = diffusion(DiffusionAlgorithm::Stucki)
        .with_strength(1.5)
        .unwrap()
        .with_error_clamp(32);
    assert_eq!(configured.strength(), 1.5);
    assert_eq!(configured.error_clamp(), Some(32));
    assert_eq!(configured.clone().without_error_clamp().error_clamp(), None);
    assert_eq!(diffusion(DiffusionAlgorithm::Stucki).strength(), 1.0);
    assert_eq!(diffusion(DiffusionAlgorithm::Stucki).error_clamp(), None);

    for strength in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
        assert_eq!(
            diffusion(DiffusionAlgorithm::Stucki)
                .with_strength(strength)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidParameter
        );
    }
}

#[test]
fn validates_palette_and_matches_custom_colours() {
    let error = Palette::new(Vec::<[u8; 3]>::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidParameter);

    let palette = Palette::new([[255, 0, 0], [0, 0, 255]]).unwrap();
    assert_eq!(palette.colours(), &[[255, 0, 0], [0, 0, 255]]);
    assert_eq!(palette.nearest_colour([200, 10, 40]), [255, 0, 0]);
    assert_eq!(palette.nearest_colour([40, 10, 200]), [0, 0, 255]);
    assert_eq!(palette.nearest_colour([127, 0, 127]), [255, 0, 0]);

    let black_and_white = Palette::black_and_white();
    assert_eq!(black_and_white.nearest_colour([255, 127, 0]), [0; 3]);
    assert_eq!(black_and_white.nearest_colour([0, 128, 255]), [255; 3]);

    assert_eq!(
        Palette::monochrome([1, 2, 3]).colours(),
        &[[0, 0, 0], [1, 2, 3], [255, 255, 255]]
    );
    assert_eq!(
        Palette::monochrome([0, 0, 0]).colours(),
        &[[0, 0, 0], [255, 255, 255]]
    );
    assert_eq!(
        Palette::monochrome([255, 255, 255]).colours(),
        &[[0, 0, 0], [255, 255, 255]]
    );
    assert_eq!(
        Palette::black_and_white().colours(),
        &[[0, 0, 0], [255, 255, 255]]
    );
    assert_eq!(Palette::black_and_white().colour_space(), ColourSpace::Rgb);
    assert_eq!(
        Palette::black_and_white().matching_mode(),
        PaletteMatchMode::Colour
    );
}

#[test]
fn derives_every_visible_source_colour_in_first_seen_order() {
    let source = source(
        7,
        1,
        &[
            [10, 20, 30, 255],
            [40, 50, 60, 128],
            [10, 20, 30, 64],
            [70, 80, 90, 0],
            [100, 110, 120, 1],
            [40, 50, 60, 255],
            [100, 110, 120, 255],
        ],
    );
    let palette = Palette::from_source(&source, PaletteSize::All).unwrap();

    assert_eq!(
        palette.colours(),
        &[[10, 20, 30], [40, 50, 60], [100, 110, 120]]
    );
    assert_eq!(palette.nearest_colour([40, 50, 60]), [40, 50, 60]);
    assert!(palette.components.is_none());
    assert!(palette.exact_colours.is_some());
    assert_eq!(PaletteSize::default(), PaletteSize::All);
}

#[test]
fn caches_palette_components_only_for_transformed_matching() {
    let rgb = Palette::new([[10, 20, 30], [40, 50, 60]]).unwrap();
    assert!(rgb.components.is_none());

    let perceptual = rgb.with_colour_space(ColourSpace::Oklab);
    assert!(perceptual.components.is_some());
    let luminance = Palette::black_and_white().with_matching_mode(PaletteMatchMode::Luminance);
    assert!(luminance.components.is_some());
}

#[test]
fn derives_a_limited_deterministic_palette_from_source_colours() {
    let mut pixels = Vec::new();
    pixels.extend([[240, 20, 20, 255]; 12]);
    pixels.extend([[180, 10, 10, 255]; 3]);
    pixels.extend([[20, 20, 240, 255]; 10]);
    pixels.extend([[10, 10, 170, 255]; 2]);
    pixels.push([20, 220, 20, 255]);
    let source = source(pixels.len() as u32, 1, &pixels);

    let first = Palette::from_source(&source, PaletteSize::Limited(2)).unwrap();
    let second = Palette::from_source(&source, PaletteSize::Limited(2)).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.colours(), &[[240, 20, 20], [20, 20, 240]]);
    assert!(
        first
            .colours()
            .iter()
            .all(|colour| { pixels.iter().any(|pixel| pixel[..3] == colour[..]) })
    );

    let all = Palette::from_source(&source, PaletteSize::All).unwrap();
    let oversized = Palette::from_source(&source, PaletteSize::Limited(99)).unwrap();
    assert_eq!(oversized, all);
}

#[test]
fn rejects_empty_derived_palettes_and_zero_limits() {
    let visible = source(1, 1, &[[1, 2, 3, 255]]);
    assert_eq!(
        Palette::from_source(&visible, PaletteSize::Limited(0))
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
    let transparent = source(2, 1, &[[1, 2, 3, 0], [4, 5, 6, 0]]);
    assert_eq!(
        Palette::from_source(&transparent, PaletteSize::All)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
}

#[test]
fn converts_colour_space_edges_and_reference_values() {
    assert_eq!(
        colour_components([0, 0, 0], ColourSpace::LinearRgb),
        [0.0; 3]
    );
    assert_eq!(
        colour_components([255, 255, 255], ColourSpace::LinearRgb),
        [1.0; 3]
    );
    let middle = colour_components([128, 128, 128], ColourSpace::LinearRgb);
    assert!(
        middle
            .into_iter()
            .all(|channel| (channel - 0.215_860_53).abs() < 1e-6)
    );

    let black = colour_components([0, 0, 0], ColourSpace::Oklab);
    assert!(black.into_iter().all(|component| component.abs() < 1e-6));
    let white = colour_components([255, 255, 255], ColourSpace::Oklab);
    assert!((white[0] - 1.0).abs() < 1e-6);
    assert!(white[1].abs() < 1e-6);
    assert!(white[2].abs() < 1e-6);
    let red = colour_components([255, 0, 0], ColourSpace::Oklab);
    for (actual, expected) in red.into_iter().zip([0.627_955_4, 0.224_863_1, 0.125_846_3]) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

#[test]
fn configures_perceptual_and_luminance_palette_matching() {
    let palette = Palette::new([[255, 0, 0], [0, 255, 0]])
        .unwrap()
        .with_colour_space(ColourSpace::LinearRgb)
        .with_matching_mode(PaletteMatchMode::Luminance);
    assert_eq!(palette.colour_space(), ColourSpace::LinearRgb);
    assert_eq!(palette.matching_mode(), PaletteMatchMode::Luminance);
    assert_eq!(palette.nearest_colour([0, 100, 0]), [255, 0, 0]);
    assert_eq!(
        palette
            .clone()
            .with_matching_mode(PaletteMatchMode::Colour)
            .nearest_colour([0, 100, 0]),
        [0, 255, 0]
    );

    let colours = (0..=255)
        .step_by(17)
        .flat_map(|red| {
            (0..=255).step_by(17).flat_map(move |green| {
                (0..=255)
                    .step_by(17)
                    .map(move |blue| [red as u8, green as u8, blue as u8])
            })
        })
        .collect::<Vec<_>>();
    let palettes = [ColourSpace::Rgb, ColourSpace::LinearRgb, ColourSpace::Oklab]
        .map(|space| Palette::pico_8().with_colour_space(space));
    assert!(colours.into_iter().any(|colour| {
        let matches = palettes
            .each_ref()
            .map(|palette| palette.nearest_colour(colour));
        matches[0] != matches[1] && matches[1] != matches[2]
    }));
}

#[test]
fn diffuses_errors_in_each_colour_space_and_mode() {
    let pixels = (0..63)
        .map(|index| {
            [
                (index * 47) as u8,
                (index * 89) as u8,
                (index * 137) as u8,
                (index * 23) as u8,
            ]
        })
        .collect::<Vec<_>>();
    let source = source(9, 7, &pixels);

    for colour_space in [ColourSpace::Rgb, ColourSpace::LinearRgb, ColourSpace::Oklab] {
        for matching_mode in [PaletteMatchMode::Colour, PaletteMatchMode::Luminance] {
            for error_mode in [
                DiffusionErrorMode::IndependentChannels,
                DiffusionErrorMode::Luminance,
            ] {
                let palette = Palette::pico_8()
                    .with_colour_space(colour_space)
                    .with_matching_mode(matching_mode);
                let effect =
                    ErrorDiffusion::new(palette.clone(), DiffusionAlgorithm::FloydSteinberg)
                        .with_error_mode(error_mode);
                let first = Renderer::new()
                    .render(&source, &effect, &Selection::All)
                    .unwrap();
                let second = Renderer::new()
                    .render(&source, &effect, &Selection::All)
                    .unwrap();
                assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
                assert_eq!(effect.error_mode(), error_mode);
                for (before, after) in source
                    .rgba8_bytes()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(first.rgba8_bytes().as_chunks::<4>().0)
                {
                    assert!(palette.colours().contains(&after[..3].try_into().unwrap()));
                    assert_eq!(after[3], before[3]);
                }
            }
        }
    }
}

#[test]
fn zero_perceptual_diffusion_matches_threshold_quantisation() {
    for colour_space in [ColourSpace::LinearRgb, ColourSpace::Oklab] {
        let palette = Palette::pico_8().with_colour_space(colour_space);
        let source = source(
            4,
            1,
            &[
                [20, 80, 140, 1],
                [70, 130, 190, 2],
                [120, 180, 240, 3],
                [200, 100, 40, 4],
            ],
        );
        let threshold = Renderer::new()
            .render(&source, &Threshold::new(palette.clone()), &Selection::All)
            .unwrap();
        let diffusion = ErrorDiffusion::new(palette, DiffusionAlgorithm::FloydSteinberg)
            .with_strength(0.0)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &diffusion, &Selection::All)
            .unwrap();
        assert_eq!(rendered.rgba8_bytes(), threshold.rgba8_bytes());
    }
}

#[test]
fn provides_colour_values_and_prefab_palettes() {
    let colour = Colour::new(12, 34, 56);
    let alias: Color = colour;
    assert_eq!(alias.rgb(), [12, 34, 56]);
    assert_eq!(Colour::from([12, 34, 56]), colour);
    assert_eq!(<[u8; 3]>::from(colour), [12, 34, 56]);

    let custom = Palette::new([Colour::BLACK, colour, Colour::WHITE]).unwrap();
    assert_eq!(
        custom.colours(),
        &[[0, 0, 0], [12, 34, 56], [255, 255, 255]]
    );
    assert_eq!(
        custom.iter().collect::<Vec<_>>(),
        [Colour::BLACK, colour, Colour::WHITE]
    );
    assert_eq!(custom.nearest(Colour::new(10, 30, 50)), colour);

    assert_eq!(
        Palette::greyscale().colours(),
        &[
            [0, 0, 0],
            [36, 36, 36],
            [73, 73, 73],
            [109, 109, 109],
            [146, 146, 146],
            [182, 182, 182],
            [219, 219, 219],
            [255, 255, 255],
        ]
    );
    assert_eq!(Palette::game_boy().colours().len(), 4);
    assert_eq!(Palette::cga().colours().len(), 16);
    assert_eq!(Palette::pico_8().colours().len(), 16);
    assert_eq!(Palette::cga().colours()[6], [170, 85, 0]);
    assert_eq!(Palette::pico_8().colours()[8], [255, 0, 77]);
}

#[test]
fn thresholds_exact_black_and_white_pixels_and_preserves_alpha() {
    let source = source(
        3,
        1,
        &[[10, 20, 30, 1], [128, 128, 128, 2], [245, 235, 225, 3]],
    );
    let rendered = Renderer::new()
        .render(
            &source,
            &Threshold::new(Palette::black_and_white()),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), Some([0, 0, 0, 1]));
    assert_eq!(rendered.pixel(1, 0), Some([255, 255, 255, 2]));
    assert_eq!(rendered.pixel(2, 0), Some([255, 255, 255, 3]));
}

#[test]
fn thresholds_monochrome_with_black_colour_and_white() {
    let source = source(
        3,
        1,
        &[[10, 10, 10, 1], [240, 10, 10, 2], [250, 250, 250, 3]],
    );
    let rendered = Renderer::new()
        .render(
            &source,
            &Threshold::new(Palette::monochrome([255, 0, 0])),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), Some([0, 0, 0, 1]));
    assert_eq!(rendered.pixel(1, 0), Some([255, 0, 0, 2]));
    assert_eq!(rendered.pixel(2, 0), Some([255, 255, 255, 3]));
}

#[test]
fn thresholds_custom_palette() {
    let source = source(2, 1, &[[240, 20, 10, 4], [20, 10, 240, 5]]);
    let effect = Threshold::new(Palette::new([[255, 0, 0], [0, 0, 255]]).unwrap());
    let rendered = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), Some([255, 0, 0, 4]));
    assert_eq!(rendered.pixel(1, 0), Some([0, 0, 255, 5]));
}

#[test]
fn thresholds_only_the_selected_polygon() {
    let source = source(3, 1, &[[100, 100, 100, 10]; 3]);
    let polygon = Polygon::new([
        Point::new(1.0, 0.0),
        Point::new(2.0, 0.0),
        Point::new(2.0, 1.0),
        Point::new(1.0, 1.0),
    ])
    .unwrap();
    let rendered = Renderer::new()
        .render(
            &source,
            &Threshold::new(Palette::black_and_white()),
            &Selection::Polygon(polygon),
        )
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), Some([100, 100, 100, 10]));
    assert_eq!(rendered.pixel(1, 0), Some([0, 0, 0, 10]));
    assert_eq!(rendered.pixel(2, 0), Some([100, 100, 100, 10]));
}

#[test]
fn validates_custom_threshold_maps() {
    let map = ThresholdMap::new(3, 2, [0, 3, 1, 4, 2, 5]).unwrap();
    assert_eq!(map.width(), 3);
    assert_eq!(map.height(), 2);
    assert_eq!(map.thresholds(), &[0, 3, 1, 4, 2, 5]);

    for result in [
        ThresholdMap::new(0, 2, Box::<[u32]>::default()),
        ThresholdMap::new(2, 0, Box::<[u32]>::default()),
        ThresholdMap::new(2, 2, [0, 1, 2].as_slice()),
        ThresholdMap::new(2, 2, [0, 1, 2, 4].as_slice()),
    ] {
        assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidParameter);
    }
}

#[test]
fn uses_standard_bayer_matrices() {
    let expected = [
        (2, vec![0, 2, 3, 1]),
        (
            4,
            vec![0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5],
        ),
        (
            8,
            vec![
                0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46,
                6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27,
                49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
            ],
        ),
    ];

    for (map, expected) in [
        (ThresholdMap::bayer_2x2(), expected[0].1.clone()),
        (ThresholdMap::bayer_4x4(), expected[1].1.clone()),
        (ThresholdMap::bayer_8x8(), expected[2].1.clone()),
    ] {
        assert_eq!(map.width(), map.height());
        assert_eq!(map.thresholds(), expected);
    }
}

#[test]
fn artistic_threshold_maps_are_distinct_and_tile_after_transformations() {
    let maps = [
        ThresholdMap::clustered_dots(),
        ThresholdMap::horizontal_lines(),
        ThresholdMap::vertical_lines(),
        ThresholdMap::diagonal_lines(),
        ThresholdMap::crosshatch(),
        ThresholdMap::checkerboard(),
        ThresholdMap::dispersed_dots_3x3(),
        ThresholdMap::dispersed_dots_5x5(),
    ];
    for (index, map) in maps.iter().enumerate() {
        assert!(maps[..index].iter().all(|other| other != map));

        for rotation in [
            ThresholdRotation::None,
            ThresholdRotation::Clockwise90,
            ThresholdRotation::Clockwise180,
            ThresholdRotation::Clockwise270,
        ] {
            for (mirror_x, mirror_y) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let effect = OrderedDither::new(Palette::black_and_white(), map.clone())
                    .with_offset(-7, 11)
                    .with_rotation(rotation)
                    .with_mirroring(mirror_x, mirror_y);
                let (width, height) = effect.transformed_dimensions();

                for y in 0..height * 2 {
                    for x in 0..width * 2 {
                        assert_eq!(effect.threshold(x, y), effect.threshold(x + width, y));
                        assert_eq!(effect.threshold(x, y), effect.threshold(x, y + height));
                    }
                }
            }
        }
    }
}

#[test]
fn renders_exact_pixels_for_each_bayer_size() {
    for map in [
        ThresholdMap::bayer_2x2(),
        ThresholdMap::bayer_4x4(),
        ThresholdMap::bayer_8x8(),
    ] {
        let size = map.width();
        let source = source(size, 1, &vec![[128, 128, 128, 90]; size as usize]);
        let effect = OrderedDither::new(Palette::black_and_white(), map);
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        for x in 0..size {
            let value = if x % 2 == 0 { 0 } else { 255 };
            assert_eq!(rendered.pixel(x, 0), Some([value, value, value, 90]));
        }
    }
}

#[test]
fn transforms_rectangular_threshold_maps() {
    let map = ThresholdMap::new(3, 2, [0, 1, 2, 3, 4, 5]).unwrap();
    let effect = OrderedDither::new(Palette::black_and_white(), map);

    assert_eq!(
        (0..6).map(|x| effect.threshold(x, 0)).collect::<Vec<_>>(),
        [0, 1, 2, 0, 1, 2]
    );
    let rotated = effect.clone().with_rotation(ThresholdRotation::Clockwise90);
    assert_eq!(
        (0..4).map(|x| rotated.threshold(x, 0)).collect::<Vec<_>>(),
        [3, 0, 3, 0]
    );
    let transformed = effect.with_offset(1, -1).with_mirroring(true, true);
    assert_eq!(
        (0..3)
            .map(|x| transformed.threshold(x, 0))
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
}

#[test]
fn renders_exact_pixels_for_a_custom_rectangular_map() {
    let source = source(3, 2, &[[128, 128, 128, 90]; 6]);
    let map = ThresholdMap::new(3, 2, [0, 1, 2, 3, 4, 5]).unwrap();
    let effect = OrderedDither::new(Palette::black_and_white(), map);
    let rendered = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();

    assert_eq!(
        rendered.rgba8_bytes(),
        &[
            0, 0, 0, 90, 0, 0, 0, 90, 0, 0, 0, 90, 255, 255, 255, 90, 255, 255, 255, 90, 255, 255,
            255, 90,
        ]
    );
}

#[test]
fn configures_threshold_strength_and_transformations() {
    let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
        .with_strength(0.5)
        .unwrap()
        .with_offset(-2, 3)
        .with_rotation(ThresholdRotation::Clockwise270)
        .with_mirroring(true, false);
    assert_eq!(effect.strength(), 0.5);
    assert_eq!(effect.offset(), (-2, 3));
    assert_eq!(effect.rotation(), ThresholdRotation::Clockwise270);
    assert!(effect.mirror_x());
    assert!(!effect.mirror_y());

    for strength in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
        assert_eq!(
            OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
                .with_strength(strength)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidParameter
        );
    }
}

#[test]
fn keeps_polygon_patterns_anchored_to_image_coordinates() {
    let source = source(4, 1, &[[128, 128, 128, 80]; 4]);
    let effect = OrderedDither::new(
        Palette::black_and_white(),
        ThresholdMap::new(4, 1, [0, 3, 1, 2]).unwrap(),
    );
    let all = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();
    let polygon = Polygon::new([
        Point::new(1.0, 0.0),
        Point::new(3.0, 0.0),
        Point::new(3.0, 1.0),
        Point::new(1.0, 1.0),
    ])
    .unwrap();
    let selected = Renderer::new()
        .render(&source, &effect, &Selection::Polygon(polygon))
        .unwrap();

    assert_eq!(selected.pixel(0, 0), source.pixel(0, 0));
    assert_eq!(selected.pixel(1, 0), all.pixel(1, 0));
    assert_eq!(selected.pixel(2, 0), all.pixel(2, 0));
    assert_eq!(selected.pixel(3, 0), source.pixel(3, 0));
}

#[test]
fn ordered_dithering_is_repeatable() {
    let source = source(
        3,
        1,
        &[[40, 80, 120, 1], [100, 140, 180, 2], [160, 200, 240, 3]],
    );
    let effect = OrderedDither::new(
        Palette::new([[0, 20, 40], [100, 120, 140], [220, 240, 255]]).unwrap(),
        ThresholdMap::bayer_4x4(),
    );
    let first = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();
    let second = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();

    assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
}

#[test]
fn blue_noise_map_contains_every_rank() {
    let map = ThresholdMap::blue_noise_16x16();
    let mut ranks = map.thresholds().to_vec();
    ranks.sort_unstable();

    assert_eq!(map.width(), 16);
    assert_eq!(map.height(), 16);
    assert_eq!(ranks, (0..256).collect::<Vec<_>>());
}

#[test]
fn blue_noise_does_not_repeat_the_built_in_map() {
    let first_tile = (0..16)
        .flat_map(|y| (0..16).map(move |x| blue_noise(x, y, 42)))
        .collect::<Vec<_>>();
    let next_tile = (0..16)
        .flat_map(|y| (16..32).map(move |x| blue_noise(x, y, 42)))
        .collect::<Vec<_>>();

    assert_ne!(first_tile, next_tile);
}

#[test]
fn noise_is_repeatable_seeded_and_anchored_to_image_coordinates() {
    let source = source(16, 8, &[[128, 128, 128, 90]; 128]);
    let polygon = Polygon::rectangle(Point::new(4.0, 2.0), 8.0, 4.0).unwrap();

    for algorithm in [NoiseAlgorithm::White, NoiseAlgorithm::Blue] {
        let effect = NoiseDither::new(Palette::black_and_white(), algorithm).with_seed(41);
        let first = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        let second = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        let different_seed = Renderer::new()
            .render(
                &source,
                &NoiseDither::new(Palette::black_and_white(), algorithm).with_seed(42),
                &Selection::All,
            )
            .unwrap();
        let selected = Renderer::new()
            .render(&source, &effect, &Selection::Polygon(polygon.clone()))
            .unwrap();

        assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
        assert_ne!(first.rgba8_bytes(), different_seed.rgba8_bytes());
        for y in 0..8 {
            for x in 0..16 {
                if (4..12).contains(&x) && (2..6).contains(&y) {
                    assert_eq!(selected.pixel(x, y), first.pixel(x, y));
                } else {
                    assert_eq!(selected.pixel(x, y), source.pixel(x, y));
                }
            }
        }
    }
}

#[test]
fn configures_noise_strength_and_pixel_size() {
    let effect = NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::White)
        .with_seed(123)
        .with_strength(0.75)
        .unwrap()
        .with_pixel_size(3, 3)
        .unwrap();

    assert_eq!(effect.algorithm(), NoiseAlgorithm::White);
    assert_eq!(effect.seed(), 123);
    assert_eq!(effect.strength(), 0.75);
    assert_eq!(effect.pixel_size(), (3, 3));
    for strength in [-0.1, 2.1, f32::NAN, f32::INFINITY] {
        assert_eq!(
            NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::White)
                .with_strength(strength)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidParameter
        );
    }
    assert_eq!(
        NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::White)
            .with_pixel_size(0, 1)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
}

#[test]
fn configures_and_validates_halftone_screens() {
    let effect = Halftone::new(Palette::black_and_white(), HalftoneShape::Diamond)
        .with_cell_size(12, 8)
        .unwrap()
        .with_angle(std::f32::consts::FRAC_PI_4)
        .unwrap()
        .with_phase(2.5, -1.0)
        .unwrap()
        .with_scale(0.75)
        .unwrap();

    assert_eq!(effect.shape(), HalftoneShape::Diamond);
    assert_eq!(effect.cell_width(), 12);
    assert_eq!(effect.cell_height(), 8);
    assert_eq!(effect.angle(), std::f32::consts::FRAC_PI_4);
    assert_eq!(effect.phase(), (2.5, -1.0));
    assert_eq!(effect.scale(), 0.75);

    assert_eq!(
        Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
            .with_cell_width(0)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
                .with_scale(value)
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidParameter
        );
    }
}

#[test]
fn halftone_shapes_render_to_the_palette_at_image_edges_and_preserve_alpha() {
    let pixels = (0..35)
        .map(|index| [96, 128, 160, (index * 7) as u8])
        .collect::<Vec<_>>();
    let source = source(7, 5, &pixels);
    let palette = Palette::new([[10, 20, 30], [90, 120, 150], [240, 245, 250]]).unwrap();

    for shape in [
        HalftoneShape::Circle,
        HalftoneShape::Square,
        HalftoneShape::Diamond,
        HalftoneShape::Ellipse,
        HalftoneShape::Line,
        HalftoneShape::Cross,
    ] {
        let effect = Halftone::new(palette.clone(), shape)
            .with_cell_size(9, 6)
            .unwrap()
            .with_angle(0.37)
            .unwrap()
            .with_phase(-2.0, 1.5)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        for (before, after) in source
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(rendered.rgba8_bytes().as_chunks::<4>().0)
        {
            assert!(palette.colours().contains(&after[..3].try_into().unwrap()));
            assert_eq!(after[3], before[3]);
        }
    }
}

#[test]
fn halftone_transformations_stay_anchored_across_selections() {
    let source = source(12, 8, &[[120, 140, 160, 73]; 96]);
    let effect = Halftone::new(Palette::monochrome(Colour::RED), HalftoneShape::Cross)
        .with_cell_size(5, 7)
        .unwrap()
        .with_angle(0.63)
        .unwrap()
        .with_phase(1.25, -3.5)
        .unwrap()
        .with_scale(1.2)
        .unwrap();
    let full = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();
    let polygon = Polygon::rectangle(Point::new(3.0, 2.0), 6.0, 4.0).unwrap();
    let selected = Renderer::new()
        .render(&source, &effect, &Selection::Polygon(polygon))
        .unwrap();

    for y in 0..8 {
        for x in 0..12 {
            if (3..9).contains(&x) && (2..6).contains(&y) {
                assert_eq!(selected.pixel(x, y), full.pixel(x, y));
            } else {
                assert_eq!(selected.pixel(x, y), source.pixel(x, y));
            }
        }
    }
}

#[test]
fn separates_rgb_into_cmyk_channels() {
    assert_eq!(rgb_to_cmyk([255, 0, 0]), [0, 255, 255, 0]);
    assert_eq!(rgb_to_cmyk([0, 255, 0]), [255, 0, 255, 0]);
    assert_eq!(rgb_to_cmyk([0, 0, 255]), [255, 255, 0, 0]);
    assert_eq!(rgb_to_cmyk([0, 0, 0]), [0, 0, 0, 255]);
    assert_eq!(rgb_to_cmyk([128, 128, 128]), [0, 0, 0, 127]);
    assert_eq!(cmyk_to_rgb([0, 0, 0, 0]), [255, 255, 255]);
    assert_eq!(cmyk_to_rgb([255, 0, 0, 0]), [0, 255, 255]);
    assert_eq!(cmyk_to_rgb([0, 0, 0, 255]), [0, 0, 0]);
}

#[test]
fn configures_independent_rgb_and_cmyk_screens() {
    let rgb = ColourHalftone::new(ColourHalftoneMode::Rgb, HalftoneShape::Circle)
        .with_cell_size(12, 9)
        .unwrap()
        .with_scale(0.8)
        .unwrap()
        .with_channel_angle(HalftoneChannel::Red, 0.25)
        .unwrap()
        .with_channel_offset(HalftoneChannel::Blue, 2.0, -3.0)
        .unwrap();
    assert_eq!(rgb.mode(), ColourHalftoneMode::Rgb);
    assert_eq!(rgb.shape(), HalftoneShape::Circle);
    assert_eq!(rgb.cell_width(), 12);
    assert_eq!(rgb.cell_height(), 9);
    assert_eq!(rgb.scale(), 0.8);
    assert_eq!(rgb.channel_angle(HalftoneChannel::Red), Some(0.25));
    assert_eq!(rgb.channel_offset(HalftoneChannel::Blue), Some((2.0, -3.0)));
    assert_eq!(rgb.channel_angle(HalftoneChannel::Cyan), None);

    let cmyk = ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Ellipse)
        .with_channel_offset(HalftoneChannel::Cyan, 1.0, 2.0)
        .unwrap()
        .with_cmyk_preset(CmykScreenPreset::MoireResistant)
        .unwrap();
    assert_eq!(cmyk.channel_angle(HalftoneChannel::Cyan), Some(0.321_140_6));
    assert_eq!(
        cmyk.channel_angle(HalftoneChannel::Magenta),
        Some(1.249_455_8)
    );
    assert_eq!(cmyk.channel_angle(HalftoneChannel::Yellow), Some(0.0));
    assert_eq!(
        cmyk.channel_angle(HalftoneChannel::Black),
        Some(std::f32::consts::FRAC_PI_4)
    );
    assert_eq!(cmyk.channel_offset(HalftoneChannel::Cyan), Some((1.0, 2.0)));
}

#[test]
fn rejects_channels_and_values_outside_the_selected_mode() {
    let rgb = ColourHalftone::new(ColourHalftoneMode::Rgb, HalftoneShape::Circle);
    let errors = [
        rgb.clone()
            .with_channel_angle(HalftoneChannel::Cyan, 0.0)
            .unwrap_err(),
        rgb.clone()
            .with_channel_offset(HalftoneChannel::Black, 0.0, 0.0)
            .unwrap_err(),
        rgb.clone()
            .with_cmyk_preset(CmykScreenPreset::Traditional)
            .unwrap_err(),
        rgb.clone()
            .with_channel_angle(HalftoneChannel::Red, f32::NAN)
            .unwrap_err(),
        rgb.with_channel_offset(HalftoneChannel::Red, f32::INFINITY, 0.0)
            .unwrap_err(),
    ];
    assert!(
        errors
            .iter()
            .all(|error| error.kind() == ErrorKind::InvalidParameter)
    );
}

#[test]
fn colour_halftoning_is_deterministic_anchored_and_alpha_safe() {
    let pixels = (0..96)
        .map(|index| [100, 140, 180, (index * 11) as u8])
        .collect::<Vec<_>>();
    let source = source(12, 8, &pixels);

    for mode in [ColourHalftoneMode::Rgb, ColourHalftoneMode::Cmyk] {
        let effect = ColourHalftone::new(mode, HalftoneShape::Circle)
            .with_cell_size(7, 5)
            .unwrap();
        let first = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        let second = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();
        assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
        assert!(
            first
                .rgba8_bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2])
        );
        for (before, after) in source
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(first.rgba8_bytes().as_chunks::<4>().0)
        {
            assert_eq!(after[3], before[3]);
        }

        let polygon = Polygon::rectangle(Point::new(3.0, 2.0), 6.0, 4.0).unwrap();
        let selected = Renderer::new()
            .render(&source, &effect, &Selection::Polygon(polygon))
            .unwrap();
        for y in 0..8 {
            for x in 0..12 {
                if (3..9).contains(&x) && (2..6).contains(&y) {
                    assert_eq!(selected.pixel(x, y), first.pixel(x, y));
                } else {
                    assert_eq!(selected.pixel(x, y), source.pixel(x, y));
                }
            }
        }
    }
}

#[test]
fn colour_halftoning_keeps_rgb_and_cmyk_endpoints_solid() {
    for mode in [ColourHalftoneMode::Rgb, ColourHalftoneMode::Cmyk] {
        let effect = ColourHalftone::new(mode, HalftoneShape::Circle);
        for colour in [[0, 0, 0, 17], [255, 255, 255, 29]] {
            let source = source(11, 9, &vec![colour; 99]);
            let rendered = Renderer::new()
                .render(&source, &effect, &Selection::All)
                .unwrap();
            assert_eq!(rendered.rgba8_bytes(), source.rgba8_bytes());
        }
    }
}

#[test]
fn floyd_steinberg_produces_exact_pixels() {
    let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
    let rendered = Renderer::new()
        .render(
            &source,
            &diffusion(DiffusionAlgorithm::FloydSteinberg),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(
        rendered.rgba8_bytes(),
        &[0, 0, 0, 70, 255, 255, 255, 70, 0, 0, 0, 70, 0, 0, 0, 70,]
    );
}

#[test]
fn floyd_steinberg_does_not_import_error_across_a_polygon_boundary() {
    let source = source(2, 1, &[[100, 100, 100, 20]; 2]);
    let polygon = Polygon::new([
        Point::new(1.0, 0.0),
        Point::new(2.0, 0.0),
        Point::new(2.0, 1.0),
        Point::new(1.0, 1.0),
    ])
    .unwrap();
    let rendered = Renderer::new()
        .render(
            &source,
            &diffusion(DiffusionAlgorithm::FloydSteinberg),
            &Selection::Polygon(polygon),
        )
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), Some([100, 100, 100, 20]));
    assert_eq!(rendered.pixel(1, 0), Some([0, 0, 0, 20]));
}

#[test]
fn floyd_steinberg_handles_narrow_edge_selections() {
    let source = source(
        1,
        3,
        &[[80, 80, 80, 1], [120, 120, 120, 2], [160, 160, 160, 3]],
    );
    let rendered = Renderer::new()
        .render(
            &source,
            &diffusion(DiffusionAlgorithm::FloydSteinberg),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(rendered.dimensions(), (1, 3));
    assert_eq!(
        rendered
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[3])
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn atkinson_produces_exact_pixels() {
    let source = source(4, 1, &[[100, 100, 100, 70]; 4]);
    let rendered = Renderer::new()
        .render(
            &source,
            &diffusion(DiffusionAlgorithm::Atkinson),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(
        rendered.rgba8_bytes(),
        &[0, 0, 0, 70, 0, 0, 0, 70, 0, 0, 0, 70, 255, 255, 255, 70,]
    );
}

#[test]
fn atkinson_does_not_jump_unselected_gaps() {
    let input = [100, 100, 100, 1, 0, 0, 0, 2, 120, 120, 120, 3];
    let mut output = input;
    let mask = Mask::new(3, 1, vec![255, 0, 255]).unwrap();

    diffusion(DiffusionAlgorithm::Atkinson)
        .apply(&input, &mut output, (3, 1), &mask)
        .unwrap();

    assert_eq!(output, [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3]);
}

#[test]
fn atkinson_handles_narrow_image_edges() {
    let source = source(1, 2, &[[100, 100, 100, 4], [140, 140, 140, 5]]);
    let rendered = Renderer::new()
        .render(
            &source,
            &diffusion(DiffusionAlgorithm::Atkinson),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(rendered.dimensions(), (1, 2));
    assert_eq!(rendered.pixel(0, 0).unwrap()[3], 4);
    assert_eq!(rendered.pixel(0, 1).unwrap()[3], 5);
}

#[test]
fn error_diffusion_is_repeatable() {
    let source = source(
        3,
        2,
        &[
            [30, 60, 90, 1],
            [70, 100, 130, 2],
            [110, 140, 170, 3],
            [150, 180, 210, 4],
            [190, 220, 250, 5],
            [230, 200, 170, 6],
        ],
    );

    let floyd = diffusion(DiffusionAlgorithm::FloydSteinberg);
    let first = Renderer::new()
        .render(&source, &floyd, &Selection::All)
        .unwrap();
    let second = Renderer::new()
        .render(&source, &floyd, &Selection::All)
        .unwrap();
    assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());

    let atkinson = diffusion(DiffusionAlgorithm::Atkinson);
    let first = Renderer::new()
        .render(&source, &atkinson, &Selection::All)
        .unwrap();
    let second = Renderer::new()
        .render(&source, &atkinson, &Selection::All)
        .unwrap();
    assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
}

#[test]
fn ostromoukhov_coefficients_match_the_published_table() {
    assert_eq!(published_coefficients(0), [13, 0, 5]);
    assert_eq!(published_coefficients(22), [3, 2, 1]);
    assert_eq!(published_coefficients(64), [11, 10, 0]);
    assert_eq!(published_coefficients(77), [4, 1, 1]);
    assert_eq!(published_coefficients(95), [5, 3, 2]);
    assert_eq!(published_coefficients(127), [4, 1, 1]);

    let checksum = (0u8..128)
        .map(|tone| {
            published_coefficients(tone)
                .into_iter()
                .enumerate()
                .map(|(channel, value)| {
                    u64::from(tone + 1) * (channel + 1) as u64 * u64::from(value)
                })
                .sum::<u64>()
        })
        .sum::<u64>();
    assert_eq!(checksum, 8_700_413);

    for tone in 0..=255 {
        assert_eq!(
            published_coefficients(tone),
            published_coefficients(255 - tone)
        );
        assert!(published_coefficients(tone).iter().sum::<u16>() > 0);
    }
}

#[test]
fn ostromoukhov_is_deterministic_distinct_and_smooth_on_gradients() {
    let pixels = (0..64)
        .flat_map(|_| (0..256).map(|value| [value as u8, value as u8, value as u8, 173]))
        .collect::<Vec<_>>();
    let image = source(256, 64, &pixels);
    let effect = OstromoukhovDither::new(Palette::black_and_white());
    let first = Renderer::new()
        .render(&image, &effect, &Selection::All)
        .unwrap();
    let second = Renderer::new()
        .render(&image, &effect, &Selection::All)
        .unwrap();
    let fixed = Renderer::new()
        .render(
            &image,
            &diffusion(DiffusionAlgorithm::FloydSteinberg).with_scan(DiffusionScan::Serpentine),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
    assert_ne!(first.rgba8_bytes(), fixed.rgba8_bytes());

    let white_counts = (0..16)
        .map(|band| {
            (0..64)
                .flat_map(|y| (band * 16..band * 16 + 16).map(move |x| (x, y)))
                .filter(|&(x, y)| first.pixel(x, y).unwrap()[0] == 255)
                .count()
        })
        .collect::<Vec<_>>();
    assert!(white_counts.windows(2).all(|counts| counts[0] < counts[1]));
    for (band, &actual) in white_counts.iter().enumerate() {
        let expected = (0..64)
            .flat_map(|_| band * 16..band * 16 + 16)
            .map(|value| value as f32 / 255.0)
            .sum::<f32>();
        assert!((actual as f32 - expected).abs() < 16.0);
    }
    assert!(
        first
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| { (pixel[..3] == [0; 3] || pixel[..3] == [255; 3]) && pixel[3] == 173 })
    );
}

#[test]
fn ostromoukhov_supports_logical_pixels_and_polygon_selections() {
    let pixels = (0..35)
        .map(|index| {
            [
                (index * 37) as u8,
                (index * 71) as u8,
                (index * 109) as u8,
                201,
            ]
        })
        .collect::<Vec<_>>();
    let image = source(7, 5, &pixels);
    let polygon = Polygon::rectangle(Point::new(1.0, 1.0), 5.0, 3.0).unwrap();
    let effect = OstromoukhovDither::new(Palette::pico_8())
        .with_pixel_size(2, 2)
        .unwrap()
        .with_grid_offset(1, 0)
        .with_sampling(SamplingMode::DominantColour);
    let rendered = Renderer::new()
        .render(&image, &effect, &Selection::Polygon(polygon))
        .unwrap();

    assert_eq!(effect.pixel_size(), (2, 2));
    assert_eq!((effect.pixel_width(), effect.pixel_height()), (2, 2));
    assert_eq!(effect.grid_offset(), (1, 0));
    assert_eq!(effect.sampling(), SamplingMode::DominantColour);
    assert_eq!(effect.palette(), &Palette::pico_8());
    assert_eq!(
        OstromoukhovDither::new(Palette::black_and_white())
            .with_pixel_width(0)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
    for y in 0..5 {
        for x in 0..7 {
            if (1..6).contains(&x) && (1..4).contains(&y) {
                let pixel = rendered.pixel(x, y).unwrap();
                assert!(
                    effect
                        .palette()
                        .colours()
                        .contains(&pixel[..3].try_into().unwrap())
                );
                assert_eq!(pixel[3], 201);
            } else {
                assert_eq!(rendered.pixel(x, y), image.pixel(x, y));
            }
        }
    }
}

#[test]
fn hilbert_traversal_visits_every_rectangular_cell_once() {
    assert_eq!(
        hilbert_cells(4, 4),
        [
            (0, 0),
            (1, 0),
            (1, 1),
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 3),
            (1, 2),
            (2, 2),
            (2, 3),
            (3, 3),
            (3, 2),
            (3, 1),
            (2, 1),
            (2, 0),
            (3, 0),
        ]
    );

    let cells = hilbert_cells(5, 3);
    let unique = cells
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(cells.len(), 15);
    assert_eq!(unique.len(), 15);
    assert!((0..3).all(|y| (0..5).all(|x| unique.contains(&(x, y)))));
}

#[test]
fn configures_and_validates_riemersma_dithering() {
    let effect = RiemersmaDither::new(Palette::pico_8())
        .with_history_length(8)
        .unwrap()
        .with_decay(4.0)
        .unwrap()
        .with_pixel_size(3, 2)
        .unwrap()
        .with_grid_offset(-1, 2)
        .with_sampling(SamplingMode::Centre);

    assert_eq!(effect.palette(), &Palette::pico_8());
    assert_eq!(effect.history_length(), 8);
    assert_eq!(effect.decay(), 4.0);
    assert_eq!(effect.pixel_size(), (3, 2));
    assert_eq!((effect.pixel_width(), effect.pixel_height()), (3, 2));
    assert_eq!(effect.grid_offset(), (-1, 2));
    assert_eq!(effect.sampling(), SamplingMode::Centre);

    let default = RiemersmaDither::new(Palette::black_and_white());
    assert_eq!(default.history_length(), 16);
    assert_eq!(default.decay(), 16.0);

    for error in [
        default.clone().with_history_length(0).unwrap_err(),
        default.clone().with_decay(0.99).unwrap_err(),
        default.clone().with_decay(f32::NAN).unwrap_err(),
        default.clone().with_decay(f32::INFINITY).unwrap_err(),
        default.with_pixel_size(0, 1).unwrap_err(),
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidParameter);
    }
}

#[test]
fn riemersma_is_deterministic_distinct_and_configurable() {
    let pixels = (0..64)
        .map(|index| {
            let value = ((index * 43 + index * index * 7) % 256) as u8;
            [value, value, value, 100 + index as u8]
        })
        .collect::<Vec<_>>();
    let image = source(8, 8, &pixels);
    let effect = RiemersmaDither::new(Palette::black_and_white());
    let first = Renderer::new()
        .render(&image, &effect, &Selection::All)
        .unwrap();
    let second = Renderer::new()
        .render(&image, &effect, &Selection::All)
        .unwrap();
    let serpentine = Renderer::new()
        .render(
            &image,
            &diffusion(DiffusionAlgorithm::FloydSteinberg).with_scan(DiffusionScan::Serpentine),
            &Selection::All,
        )
        .unwrap();
    let short_history = Renderer::new()
        .render(
            &image,
            &effect
                .clone()
                .with_history_length(1)
                .unwrap()
                .with_decay(1.0)
                .unwrap(),
            &Selection::All,
        )
        .unwrap();

    assert_eq!(first.rgba8_bytes(), second.rgba8_bytes());
    assert_ne!(first.rgba8_bytes(), serpentine.rgba8_bytes());
    assert_ne!(first.rgba8_bytes(), short_history.rgba8_bytes());
    for (before, after) in image
        .rgba8_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(first.rgba8_bytes().as_chunks::<4>().0)
    {
        assert!(after[..3] == [0; 3] || after[..3] == [255; 3]);
        assert_eq!(after[3], before[3]);
    }
}

#[test]
fn riemersma_supports_logical_pixels_and_polygon_selections() {
    let pixels = (0..35)
        .map(|index| {
            [
                (index * 37) as u8,
                (index * 71) as u8,
                (index * 109) as u8,
                200,
            ]
        })
        .collect::<Vec<_>>();
    let image = source(7, 5, &pixels);
    let polygon = Polygon::rectangle(Point::new(1.0, 1.0), 5.0, 3.0).unwrap();
    let effect = RiemersmaDither::new(Palette::pico_8())
        .with_pixel_size(2, 2)
        .unwrap()
        .with_grid_offset(1, 0)
        .with_sampling(SamplingMode::DominantColour);
    let rendered = Renderer::new()
        .render(&image, &effect, &Selection::Polygon(polygon))
        .unwrap();

    for y in 0..5 {
        for x in 0..7 {
            if (1..6).contains(&x) && (1..4).contains(&y) {
                let pixel = rendered.pixel(x, y).unwrap();
                assert!(
                    effect
                        .palette()
                        .colours()
                        .contains(&[pixel[0], pixel[1], pixel[2]])
                );
                assert_eq!(pixel[3], 200);
            } else {
                assert_eq!(rendered.pixel(x, y), image.pixel(x, y));
            }
        }
    }
}

#[test]
fn validates_dither_pixel_sizes() {
    assert_eq!(
        Threshold::new(Palette::black_and_white()).pixel_size(),
        (1, 1)
    );
    assert_eq!(
        Threshold::new(Palette::black_and_white())
            .with_pixel_size(3, 3)
            .unwrap()
            .pixel_size(),
        (3, 3)
    );

    let errors = [
        Threshold::new(Palette::black_and_white())
            .with_pixel_size(0, 1)
            .unwrap_err(),
        OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
            .with_pixel_size(0, 1)
            .unwrap_err(),
        diffusion(DiffusionAlgorithm::FloydSteinberg)
            .with_pixel_size(0, 1)
            .unwrap_err(),
        diffusion(DiffusionAlgorithm::Atkinson)
            .with_pixel_size(0, 1)
            .unwrap_err(),
    ];
    assert!(
        errors
            .iter()
            .all(|error| error.kind() == ErrorKind::InvalidParameter)
    );
}

#[test]
fn configures_rectangular_pixel_grids_and_sampling_for_every_dither() {
    let threshold = Threshold::new(Palette::black_and_white())
        .with_pixel_width(2)
        .unwrap()
        .with_pixel_height(3)
        .unwrap()
        .with_grid_offset(-1, 4)
        .with_sampling(SamplingMode::Centre);
    let ordered = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
        .with_pixel_size(2, 3)
        .unwrap()
        .with_grid_offset(-1, 4)
        .with_sampling(SamplingMode::Darkest);
    let noise = NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::White)
        .with_pixel_size(2, 3)
        .unwrap()
        .with_grid_offset(-1, 4)
        .with_sampling(SamplingMode::Lightest);
    let configured_diffusion = diffusion(DiffusionAlgorithm::FloydSteinberg)
        .with_pixel_size(2, 3)
        .unwrap()
        .with_grid_offset(-1, 4)
        .with_sampling(SamplingMode::DominantColour);

    assert_eq!(threshold.pixel_size(), (2, 3));
    assert_eq!((threshold.pixel_width(), threshold.pixel_height()), (2, 3));
    assert_eq!(threshold.grid_offset(), (-1, 4));
    assert_eq!(threshold.sampling(), SamplingMode::Centre);
    assert_eq!(ordered.pixel_size(), (2, 3));
    assert_eq!(ordered.grid_offset(), (-1, 4));
    assert_eq!(ordered.sampling(), SamplingMode::Darkest);
    assert_eq!(noise.pixel_size(), (2, 3));
    assert_eq!(noise.grid_offset(), (-1, 4));
    assert_eq!(noise.sampling(), SamplingMode::Lightest);
    assert_eq!(configured_diffusion.pixel_size(), (2, 3));
    assert_eq!(configured_diffusion.grid_offset(), (-1, 4));
    assert_eq!(
        configured_diffusion.sampling(),
        SamplingMode::DominantColour
    );
    assert_eq!(SamplingMode::default(), SamplingMode::Average);

    for error in [
        Threshold::new(Palette::black_and_white())
            .with_pixel_height(0)
            .unwrap_err(),
        OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
            .with_pixel_width(0)
            .unwrap_err(),
        NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::White)
            .with_pixel_size(1, 0)
            .unwrap_err(),
        diffusion(DiffusionAlgorithm::FloydSteinberg)
            .with_pixel_size(1, 0)
            .unwrap_err(),
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidParameter);
    }
}

#[test]
fn samples_average_centre_extrema_and_dominant_colour() {
    let image = source(
        3,
        3,
        &[
            [200, 0, 0, 1],
            [0, 0, 0, 2],
            [200, 0, 0, 3],
            [0, 0, 255, 4],
            [0, 255, 0, 5],
            [255, 255, 255, 6],
            [255, 255, 0, 7],
            [0, 255, 255, 8],
            [255, 0, 255, 9],
        ],
    );
    let mask = Selection::All.rasterise(3, 3).unwrap();
    let sample =
        |sampling| sample_cell(image.rgba8_bytes(), &mask, 3, (0, 0, 3, 3), sampling).unwrap();

    assert_eq!(sample(SamplingMode::Average), [129, 113, 113]);
    assert_eq!(sample(SamplingMode::Centre), [0, 255, 0]);
    assert_eq!(sample(SamplingMode::Darkest), [0, 0, 0]);
    assert_eq!(sample(SamplingMode::Lightest), [255, 255, 255]);
    assert_eq!(sample(SamplingMode::DominantColour), [200, 0, 0]);
}

#[test]
fn sampling_uses_only_the_covered_cell_intersection() {
    let image = source(
        4,
        1,
        &[
            [255, 0, 0, 1],
            [0, 0, 255, 2],
            [255, 0, 0, 3],
            [0, 255, 0, 4],
        ],
    );
    let mask = Mask::new(4, 1, vec![64, 255, 191, 0]).unwrap();
    let sample =
        |sampling| sample_cell(image.rgba8_bytes(), &mask, 4, (0, 0, 4, 1), sampling).unwrap();

    assert_eq!(sample(SamplingMode::Average), [128, 0, 128]);
    assert_eq!(sample(SamplingMode::Centre), [255, 0, 0]);
    assert_eq!(sample(SamplingMode::Darkest), [0, 0, 255]);
    assert_eq!(sample(SamplingMode::Lightest), [255, 0, 0]);
    assert_eq!(sample(SamplingMode::DominantColour), [255, 0, 0]);
}

#[test]
fn grid_offsets_create_image_clipped_edge_cells() {
    let pixels = (1..=7)
        .map(|value| {
            let value = value * 10;
            [value, value, value, value]
        })
        .collect::<Vec<_>>();
    let image = source(7, 1, &pixels);
    let palette = Palette::new((1..=7).map(|value| [value * 10; 3]).collect::<Vec<_>>()).unwrap();
    let effect = Threshold::new(palette)
        .with_pixel_size(3, 1)
        .unwrap()
        .with_grid_offset(1, 0)
        .with_sampling(SamplingMode::Centre);
    let rendered = Renderer::new()
        .render(&image, &effect, &Selection::All)
        .unwrap();

    assert_eq!(
        rendered
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[0])
            .collect::<Vec<_>>(),
        [10, 30, 30, 30, 60, 60, 60]
    );
}

#[test]
fn every_dither_shares_sampling_for_partial_edges_and_polygon_intersections() {
    let pixels = (0..35)
        .map(|index| {
            [
                (index * 37 + 11) as u8,
                (index * 71 + 29) as u8,
                (index * 113 + 47) as u8,
                (index * 5 + 50) as u8,
            ]
        })
        .collect::<Vec<_>>();
    let image = source(7, 5, &pixels);
    let polygon = Polygon::new([
        Point::new(0.4, 0.2),
        Point::new(6.8, 1.1),
        Point::new(5.6, 4.9),
        Point::new(1.1, 4.2),
    ])
    .unwrap();

    for sampling in [
        SamplingMode::Average,
        SamplingMode::Centre,
        SamplingMode::Darkest,
        SamplingMode::Lightest,
        SamplingMode::DominantColour,
    ] {
        let configure_threshold = || {
            Threshold::new(Palette::pico_8())
                .with_pixel_size(3, 2)
                .unwrap()
                .with_grid_offset(1, -1)
                .with_sampling(sampling)
        };
        let expected = Renderer::new()
            .render(
                &image,
                &configure_threshold(),
                &Selection::Polygon(polygon.clone()),
            )
            .unwrap();
        let ordered = OrderedDither::new(Palette::pico_8(), ThresholdMap::bayer_2x2())
            .with_strength(0.0)
            .unwrap()
            .with_pixel_size(3, 2)
            .unwrap()
            .with_grid_offset(1, -1)
            .with_sampling(sampling);
        let noise = NoiseDither::new(Palette::pico_8(), NoiseAlgorithm::White)
            .with_strength(0.0)
            .unwrap()
            .with_pixel_size(3, 2)
            .unwrap()
            .with_grid_offset(1, -1)
            .with_sampling(sampling);
        let diffusion = ErrorDiffusion::new(Palette::pico_8(), DiffusionAlgorithm::FloydSteinberg)
            .with_strength(0.0)
            .unwrap()
            .with_pixel_size(3, 2)
            .unwrap()
            .with_grid_offset(1, -1)
            .with_sampling(sampling);

        for rendered in [
            Renderer::new()
                .render(&image, &ordered, &Selection::Polygon(polygon.clone()))
                .unwrap(),
            Renderer::new()
                .render(&image, &noise, &Selection::Polygon(polygon.clone()))
                .unwrap(),
            Renderer::new()
                .render(&image, &diffusion, &Selection::Polygon(polygon.clone()))
                .unwrap(),
        ] {
            assert_eq!(
                rendered.rgba8_bytes(),
                expected.rgba8_bytes(),
                "inconsistent {sampling:?} sampling"
            );
        }
    }
}

#[test]
fn threshold_fills_logical_pixels_with_their_average_colour() {
    let source = source(
        4,
        1,
        &[
            [0, 0, 0, 1],
            [200, 200, 200, 2],
            [100, 100, 100, 3],
            [255, 255, 255, 4],
        ],
    );
    let effect = Threshold::new(Palette::black_and_white())
        .with_pixel_size(2, 2)
        .unwrap();
    let rendered = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();

    assert_eq!(
        rendered.rgba8_bytes(),
        &[0, 0, 0, 1, 0, 0, 0, 2, 255, 255, 255, 3, 255, 255, 255, 4]
    );
}

#[test]
fn ordered_dithering_scales_bayer_cells() {
    let source = source(4, 1, &[[128, 128, 128, 9]; 4]);
    let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
        .with_pixel_size(2, 2)
        .unwrap();
    let rendered = Renderer::new()
        .render(&source, &effect, &Selection::All)
        .unwrap();

    assert_eq!(
        rendered.rgba8_bytes(),
        &[0, 0, 0, 9, 0, 0, 0, 9, 255, 255, 255, 9, 255, 255, 255, 9]
    );
}

#[test]
fn scaled_dithering_keeps_cells_anchored_and_clipped_to_a_polygon() {
    let source = source(4, 1, &[[128, 128, 128, 7]; 4]);
    let polygon = Polygon::new([
        Point::new(3.0, 0.0),
        Point::new(4.0, 0.0),
        Point::new(4.0, 1.0),
        Point::new(3.0, 1.0),
    ])
    .unwrap();
    let effect = OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_2x2())
        .with_pixel_size(2, 2)
        .unwrap();
    let rendered = Renderer::new()
        .render(&source, &effect, &Selection::Polygon(polygon))
        .unwrap();

    assert_eq!(rendered.pixel(0, 0), source.pixel(0, 0));
    assert_eq!(rendered.pixel(1, 0), source.pixel(1, 0));
    assert_eq!(rendered.pixel(2, 0), source.pixel(2, 0));
    assert_eq!(rendered.pixel(3, 0), Some([255, 255, 255, 7]));
}

#[test]
fn error_diffusion_operates_between_logical_pixels() {
    let pixels = (1..=8)
        .map(|alpha| [100, 100, 100, alpha])
        .collect::<Vec<_>>();
    let source = source(8, 1, &pixels);
    let floyd = diffusion(DiffusionAlgorithm::FloydSteinberg)
        .with_pixel_size(2, 2)
        .unwrap();
    let atkinson = diffusion(DiffusionAlgorithm::Atkinson)
        .with_pixel_size(2, 2)
        .unwrap();

    let floyd = Renderer::new()
        .render(&source, &floyd, &Selection::All)
        .unwrap();
    let atkinson = Renderer::new()
        .render(&source, &atkinson, &Selection::All)
        .unwrap();

    let floyd_colours = [0, 0, 255, 255, 0, 0, 0, 0];
    let atkinson_colours = [0, 0, 0, 0, 0, 0, 255, 255];
    for (index, pixel) in floyd.rgba8_bytes().as_chunks::<4>().0.iter().enumerate() {
        let value = floyd_colours[index];
        assert_eq!(*pixel, [value, value, value, (index + 1) as u8]);
    }
    for (index, pixel) in atkinson.rgba8_bytes().as_chunks::<4>().0.iter().enumerate() {
        let value = atkinson_colours[index];
        assert_eq!(*pixel, [value, value, value, (index + 1) as u8]);
    }
}

#[test]
fn supports_every_diffusion_algorithm_and_configuration() {
    let pixels = (0..35)
        .map(|index| {
            [
                (index * 47) as u8,
                (index * 83) as u8,
                (index * 131) as u8,
                (index * 19) as u8,
            ]
        })
        .collect::<Vec<_>>();
    let source = source(7, 5, &pixels);
    let algorithms = [
        DiffusionAlgorithm::FloydSteinberg,
        DiffusionAlgorithm::Atkinson,
        DiffusionAlgorithm::JarvisJudiceNinke,
        DiffusionAlgorithm::Stucki,
        DiffusionAlgorithm::Burkes,
        DiffusionAlgorithm::Sierra,
        DiffusionAlgorithm::TwoRowSierra,
        DiffusionAlgorithm::SierraLite,
        DiffusionAlgorithm::FalseFloydSteinberg,
        DiffusionAlgorithm::Fan,
        DiffusionAlgorithm::ShiauFan,
        DiffusionAlgorithm::ShiauFan2,
        DiffusionAlgorithm::StevensonArce,
        DiffusionAlgorithm::TwoDimensionalKnuth,
    ];

    for algorithm in algorithms {
        let effect = ErrorDiffusion::new(Palette::black_and_white(), algorithm)
            .with_scan(DiffusionScan::Serpentine)
            .with_pixel_size(2, 2)
            .unwrap();
        let rendered = Renderer::new()
            .render(&source, &effect, &Selection::All)
            .unwrap();

        assert_eq!(effect.algorithm(), Some(algorithm));
        assert_eq!(effect.scan(), DiffusionScan::Serpentine);
        assert_eq!(effect.pixel_size(), (2, 2));
        assert_eq!(effect.palette(), &Palette::black_and_white());
        for (before, after) in source
            .rgba8_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(rendered.rgba8_bytes().as_chunks::<4>().0)
        {
            assert!(after[..3] == [0; 3] || after[..3] == [255; 3]);
            assert_eq!(after[3], before[3]);
        }
    }

    assert_eq!(
        ErrorDiffusion::new(Palette::black_and_white(), DiffusionAlgorithm::Stucki)
            .with_pixel_size(0, 1)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidParameter
    );
}

#[test]
fn serpentine_scan_is_anchored_to_image_rows() {
    let selected_row = [
        [20, 20, 20, 1],
        [70, 70, 70, 2],
        [110, 110, 110, 3],
        [140, 140, 140, 4],
        [170, 170, 170, 5],
        [220, 220, 220, 6],
    ];
    let mut pixels = vec![[0, 0, 0, 255]; 6];
    pixels.extend(selected_row);
    let image = source(6, 2, &pixels);
    let mask = Mask::new(6, 2, [vec![0; 6], vec![255; 6]].concat()).unwrap();
    let effect = ErrorDiffusion::new(
        Palette::black_and_white(),
        DiffusionAlgorithm::FloydSteinberg,
    )
    .with_scan(DiffusionScan::Serpentine);
    let mut actual = image.rgba8_bytes().to_vec();
    effect
        .apply(image.rgba8_bytes(), &mut actual, (6, 2), &mask)
        .unwrap();

    let reversed = selected_row.into_iter().rev().collect::<Vec<_>>();
    let reversed_source = source(6, 1, &reversed);
    let reference = Renderer::new()
        .render(
            &reversed_source,
            &diffusion(DiffusionAlgorithm::FloydSteinberg),
            &Selection::All,
        )
        .unwrap();
    let expected = reference
        .rgba8_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .rev()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(&actual[6 * 4..], expected);
}
