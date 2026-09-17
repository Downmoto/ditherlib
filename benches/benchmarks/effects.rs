use criterion::Criterion;
use ditherlib::{
    Effect, Selection,
    effects::{
        blur::Blur,
        dither::{
            diffusion::{
                DiffusionAlgorithm, DiffusionErrorMode, DiffusionKernel, DiffusionScan,
                DiffusionTap, ErrorDiffusion,
            },
            halftone::{
                CmykScreenPreset, ColourHalftone, ColourHalftoneMode, Halftone, HalftoneChannel,
                HalftoneShape,
            },
            noise::{NoiseAlgorithm, NoiseDither},
            ostromoukhov::OstromoukhovDither,
            palette::{Palette, PaletteMatchMode},
            riemersma::RiemersmaDither,
            sampling::SamplingMode,
            threshold::{OrderedDither, Threshold, ThresholdMap, ThresholdRotation},
        },
        greyscale::Greyscale,
    },
};

use super::support::{PIXEL_SIZES, RESOLUTIONS, bench_effect, bench_effect_matrix, fixture};

const SAMPLING_MODES: [(&str, SamplingMode); 5] = [
    ("average", SamplingMode::Average),
    ("centre", SamplingMode::Centre),
    ("darkest", SamplingMode::Darkest),
    ("lightest", SamplingMode::Lightest),
    ("dominant_colour", SamplingMode::DominantColour),
];

const DIFFUSION_ALGORITHMS: [(&str, DiffusionAlgorithm); 14] = [
    ("floyd_steinberg", DiffusionAlgorithm::FloydSteinberg),
    ("atkinson", DiffusionAlgorithm::Atkinson),
    ("jarvis_judice_ninke", DiffusionAlgorithm::JarvisJudiceNinke),
    ("stucki", DiffusionAlgorithm::Stucki),
    ("burkes", DiffusionAlgorithm::Burkes),
    ("sierra", DiffusionAlgorithm::Sierra),
    ("two_row_sierra", DiffusionAlgorithm::TwoRowSierra),
    ("sierra_lite", DiffusionAlgorithm::SierraLite),
    (
        "false_floyd_steinberg",
        DiffusionAlgorithm::FalseFloydSteinberg,
    ),
    ("fan", DiffusionAlgorithm::Fan),
    ("shiau_fan", DiffusionAlgorithm::ShiauFan),
    ("shiau_fan_2", DiffusionAlgorithm::ShiauFan2),
    ("stevenson_arce", DiffusionAlgorithm::StevensonArce),
    (
        "two_dimensional_knuth",
        DiffusionAlgorithm::TwoDimensionalKnuth,
    ),
];

fn boxed(name: impl Into<String>, effect: impl Effect + 'static) -> (String, Box<dyn Effect>) {
    (name.into(), Box::new(effect))
}

pub fn benchmarks(criterion: &mut Criterion) {
    resolutions(criterion);
    blur(criterion);
    threshold(criterion);
    ordered(criterion);
    noise(criterion);
    halftone(criterion);
    colour_halftone(criterion);
    error_diffusion(criterion);
    ostromoukhov(criterion);
    riemersma(criterion);
}

fn resolutions(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("effects_by_resolution");
    let selection = Selection::All;

    for (resolution, dimensions) in RESOLUTIONS {
        let source = fixture(dimensions);
        let effects = baseline_effects();
        for (effect_name, effect) in &effects {
            bench_effect(
                &mut group,
                &format!("{effect_name}/{resolution}"),
                &source,
                effect.as_ref(),
                &selection,
            );
        }
    }

    group.finish();
}

fn baseline_effects() -> Vec<(String, Box<dyn Effect>)> {
    vec![
        boxed("greyscale", Greyscale),
        boxed("blur", Blur::new(2.0).unwrap()),
        boxed("threshold", Threshold::new(Palette::black_and_white())),
        boxed(
            "ordered",
            OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4()),
        ),
        boxed(
            "noise",
            NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue),
        ),
        boxed(
            "halftone",
            Halftone::new(Palette::black_and_white(), HalftoneShape::Circle),
        ),
        boxed(
            "colour_halftone",
            ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle),
        ),
        boxed(
            "error_diffusion",
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            ),
        ),
        boxed(
            "ostromoukhov",
            OstromoukhovDither::new(Palette::black_and_white()),
        ),
        boxed(
            "riemersma",
            RiemersmaDither::new(Palette::black_and_white()),
        ),
    ]
}

fn blur(criterion: &mut Criterion) {
    let effects = [("boundary", 0.0), ("default", 2.0), ("high", 16.0)]
        .into_iter()
        .map(|(name, sigma)| boxed(name, Blur::new(sigma).unwrap()))
        .collect();
    bench_effect_matrix(criterion, "blur_sigma", effects);
}

fn threshold(criterion: &mut Criterion) {
    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                Threshold::new(Palette::pico_8())
                    .with_pixel_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "threshold_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                Threshold::new(Palette::pico_8())
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "threshold_sampling", effects);

    let effects = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            Threshold::new(Palette::pico_8())
                .with_pixel_size(4, 4)
                .unwrap()
                .with_grid_offset(x, y),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "threshold_grid_offset", effects);
}

fn ordered(criterion: &mut Criterion) {
    let maps = vec![
        ("bayer_2x2", ThresholdMap::bayer_2x2()),
        ("bayer_4x4", ThresholdMap::bayer_4x4()),
        ("bayer_8x8", ThresholdMap::bayer_8x8()),
        ("blue_noise_16x16", ThresholdMap::blue_noise_16x16()),
        ("clustered_dots", ThresholdMap::clustered_dots()),
        ("horizontal_lines", ThresholdMap::horizontal_lines()),
        ("vertical_lines", ThresholdMap::vertical_lines()),
        ("diagonal_lines", ThresholdMap::diagonal_lines()),
        ("crosshatch", ThresholdMap::crosshatch()),
        ("checkerboard", ThresholdMap::checkerboard()),
        ("dispersed_dots_3x3", ThresholdMap::dispersed_dots_3x3()),
        ("dispersed_dots_5x5", ThresholdMap::dispersed_dots_5x5()),
        (
            "custom_3x2",
            ThresholdMap::new(3, 2, [0, 4, 2, 5, 1, 3]).unwrap(),
        ),
    ];
    let effects = maps
        .into_iter()
        .map(|(name, map)| boxed(name, OrderedDither::new(Palette::black_and_white(), map)))
        .collect();
    bench_effect_matrix(criterion, "ordered_map", effects);

    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                    .with_pixel_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ordered_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ordered_sampling", effects);

    let effects = [("boundary", 0.0), ("default", 1.0), ("high", 2.0)]
        .into_iter()
        .map(|(name, strength)| {
            boxed(
                name,
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                    .with_strength(strength)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ordered_strength", effects);

    let effects = [
        ("none", ThresholdRotation::None),
        ("clockwise_90", ThresholdRotation::Clockwise90),
        ("clockwise_180", ThresholdRotation::Clockwise180),
        ("clockwise_270", ThresholdRotation::Clockwise270),
    ]
    .into_iter()
    .map(|(name, rotation)| {
        boxed(
            name,
            OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                .with_rotation(rotation),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "ordered_rotation", effects);

    let effects = [
        ("none", false, false),
        ("horizontal", true, false),
        ("vertical", false, true),
        ("both", true, true),
    ]
    .into_iter()
    .map(|(name, horizontal, vertical)| {
        boxed(
            name,
            OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                .with_mirroring(horizontal, vertical),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "ordered_mirroring", effects);

    let offsets = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ];
    let effects = offsets
        .into_iter()
        .map(|(name, (x, y))| {
            boxed(
                name,
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                    .with_offset(x, y),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ordered_map_offset", effects);

    let effects = offsets
        .into_iter()
        .map(|(name, (x, y))| {
            boxed(
                name,
                OrderedDither::new(Palette::black_and_white(), ThresholdMap::bayer_4x4())
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_grid_offset(x, y),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ordered_grid_offset", effects);
}

fn noise(criterion: &mut Criterion) {
    let effects = [
        ("white", NoiseAlgorithm::White),
        ("blue", NoiseAlgorithm::Blue),
    ]
    .into_iter()
    .map(|(name, algorithm)| {
        boxed(
            name,
            NoiseDither::new(Palette::black_and_white(), algorithm),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "noise_algorithm", effects);

    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue)
                    .with_pixel_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "noise_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue)
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "noise_sampling", effects);

    let effects = [("boundary", 0.0), ("default", 1.0), ("high", 2.0)]
        .into_iter()
        .map(|(name, strength)| {
            boxed(
                name,
                NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue)
                    .with_strength(strength)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "noise_strength", effects);

    let effects = [("default", 0), ("one", 1), ("high", u64::MAX)]
        .into_iter()
        .map(|(name, seed)| {
            boxed(
                name,
                NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue).with_seed(seed),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "noise_seed", effects);

    let effects = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            NoiseDither::new(Palette::black_and_white(), NoiseAlgorithm::Blue)
                .with_pixel_size(4, 4)
                .unwrap()
                .with_grid_offset(x, y),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "noise_grid_offset", effects);
}

fn halftone(criterion: &mut Criterion) {
    let effects = [
        ("circle", HalftoneShape::Circle),
        ("square", HalftoneShape::Square),
        ("diamond", HalftoneShape::Diamond),
        ("ellipse", HalftoneShape::Ellipse),
        ("line", HalftoneShape::Line),
        ("cross", HalftoneShape::Cross),
    ]
    .into_iter()
    .map(|(name, shape)| boxed(name, Halftone::new(Palette::black_and_white(), shape)))
    .collect();
    bench_effect_matrix(criterion, "halftone_shape", effects);

    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
                    .with_cell_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "halftone_cell_size", effects);

    let effects = [
        ("negative", -std::f32::consts::FRAC_PI_4),
        ("default", 0.0),
        ("high", std::f32::consts::FRAC_PI_2),
    ]
    .into_iter()
    .map(|(name, angle)| {
        boxed(
            name,
            Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
                .with_angle(angle)
                .unwrap(),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "halftone_angle", effects);

    let effects = [
        ("negative", (-8.0, -8.0)),
        ("default", (0.0, 0.0)),
        ("high", (8.0, 8.0)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
                .with_phase(x, y)
                .unwrap(),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "halftone_phase", effects);

    let effects = [("boundary", 0.25), ("default", 1.0), ("high", 2.0)]
        .into_iter()
        .map(|(name, scale)| {
            boxed(
                name,
                Halftone::new(Palette::black_and_white(), HalftoneShape::Circle)
                    .with_scale(scale)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "halftone_scale", effects);
}

fn colour_halftone(criterion: &mut Criterion) {
    let effects = [
        ("rgb", ColourHalftoneMode::Rgb),
        ("cmyk", ColourHalftoneMode::Cmyk),
    ]
    .into_iter()
    .map(|(name, mode)| boxed(name, ColourHalftone::new(mode, HalftoneShape::Circle)))
    .collect();
    bench_effect_matrix(criterion, "colour_halftone_mode", effects);

    let effects = [
        ("circle", HalftoneShape::Circle),
        ("square", HalftoneShape::Square),
        ("diamond", HalftoneShape::Diamond),
        ("ellipse", HalftoneShape::Ellipse),
        ("line", HalftoneShape::Line),
        ("cross", HalftoneShape::Cross),
    ]
    .into_iter()
    .map(|(name, shape)| boxed(name, ColourHalftone::new(ColourHalftoneMode::Cmyk, shape)))
    .collect();
    bench_effect_matrix(criterion, "colour_halftone_shape", effects);

    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle)
                    .with_cell_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "colour_halftone_cell_size", effects);

    let effects = [("boundary", 0.25), ("default", 1.0), ("high", 2.0)]
        .into_iter()
        .map(|(name, scale)| {
            boxed(
                name,
                ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle)
                    .with_scale(scale)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "colour_halftone_scale", effects);

    let effects = [
        ("traditional", CmykScreenPreset::Traditional),
        ("moire_resistant", CmykScreenPreset::MoireResistant),
    ]
    .into_iter()
    .map(|(name, preset)| {
        boxed(
            name,
            ColourHalftone::new(ColourHalftoneMode::Cmyk, HalftoneShape::Circle)
                .with_cmyk_preset(preset)
                .unwrap(),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "colour_halftone_cmyk_preset", effects);

    let channels = [
        ("red", ColourHalftoneMode::Rgb, HalftoneChannel::Red),
        ("green", ColourHalftoneMode::Rgb, HalftoneChannel::Green),
        ("blue", ColourHalftoneMode::Rgb, HalftoneChannel::Blue),
        ("cyan", ColourHalftoneMode::Cmyk, HalftoneChannel::Cyan),
        (
            "magenta",
            ColourHalftoneMode::Cmyk,
            HalftoneChannel::Magenta,
        ),
        ("yellow", ColourHalftoneMode::Cmyk, HalftoneChannel::Yellow),
        ("black", ColourHalftoneMode::Cmyk, HalftoneChannel::Black),
    ];
    let angles = [
        ("boundary", 0.0),
        ("default", std::f32::consts::FRAC_PI_4),
        ("high", std::f32::consts::FRAC_PI_2),
    ];
    let mut effects = Vec::new();
    for (channel_name, mode, channel) in channels {
        for (angle_name, angle) in angles {
            effects.push(boxed(
                format!("{channel_name}/{angle_name}"),
                ColourHalftone::new(mode, HalftoneShape::Circle)
                    .with_channel_angle(channel, angle)
                    .unwrap(),
            ));
        }
    }
    bench_effect_matrix(criterion, "colour_halftone_channel_angle", effects);

    let offsets = [
        ("negative", (-8.0, -8.0)),
        ("default", (0.0, 0.0)),
        ("high", (8.0, 8.0)),
    ];
    let mut effects = Vec::new();
    for (channel_name, mode, channel) in channels {
        for (offset_name, (x, y)) in offsets {
            effects.push(boxed(
                format!("{channel_name}/{offset_name}"),
                ColourHalftone::new(mode, HalftoneShape::Circle)
                    .with_channel_offset(channel, x, y)
                    .unwrap(),
            ));
        }
    }
    bench_effect_matrix(criterion, "colour_halftone_channel_offset", effects);
}

fn error_diffusion(criterion: &mut Criterion) {
    let mut effects = DIFFUSION_ALGORITHMS
        .into_iter()
        .map(|(name, algorithm)| {
            boxed(
                name,
                ErrorDiffusion::new(Palette::black_and_white(), algorithm),
            )
        })
        .collect::<Vec<_>>();
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
    effects.push(boxed(
        "custom_kernel",
        ErrorDiffusion::new(Palette::black_and_white(), custom),
    ));
    bench_effect_matrix(criterion, "error_diffusion_kernel", effects);

    let effects = [
        ("raster", DiffusionScan::Raster),
        ("serpentine", DiffusionScan::Serpentine),
    ]
    .into_iter()
    .map(|(name, scan)| {
        boxed(
            name,
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_scan(scan),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "error_diffusion_scan", effects);

    let effects = [
        (
            "independent_channels",
            DiffusionErrorMode::IndependentChannels,
        ),
        ("luminance", DiffusionErrorMode::Luminance),
    ]
    .into_iter()
    .map(|(name, mode)| {
        boxed(
            name,
            ErrorDiffusion::new(
                Palette::black_and_white().with_matching_mode(PaletteMatchMode::Luminance),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_error_mode(mode),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "error_diffusion_error_mode", effects);

    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                ErrorDiffusion::new(
                    Palette::black_and_white(),
                    DiffusionAlgorithm::FloydSteinberg,
                )
                .with_pixel_size(width, height)
                .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "error_diffusion_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                ErrorDiffusion::new(
                    Palette::black_and_white(),
                    DiffusionAlgorithm::FloydSteinberg,
                )
                .with_pixel_size(4, 4)
                .unwrap()
                .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "error_diffusion_sampling", effects);

    let effects = [("boundary", 0.0), ("default", 1.0), ("high", 2.0)]
        .into_iter()
        .map(|(name, strength)| {
            boxed(
                name,
                ErrorDiffusion::new(
                    Palette::black_and_white(),
                    DiffusionAlgorithm::FloydSteinberg,
                )
                .with_strength(strength)
                .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "error_diffusion_strength", effects);

    let effects = vec![
        boxed(
            "none",
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .without_error_clamp(),
        ),
        boxed(
            "boundary",
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_error_clamp(0),
        ),
        boxed(
            "representative",
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_error_clamp(32),
        ),
        boxed(
            "high",
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_error_clamp(u8::MAX),
        ),
    ];
    bench_effect_matrix(criterion, "error_diffusion_error_clamp", effects);

    let effects = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            ErrorDiffusion::new(
                Palette::black_and_white(),
                DiffusionAlgorithm::FloydSteinberg,
            )
            .with_pixel_size(4, 4)
            .unwrap()
            .with_grid_offset(x, y),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "error_diffusion_grid_offset", effects);
}

fn ostromoukhov(criterion: &mut Criterion) {
    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                OstromoukhovDither::new(Palette::black_and_white())
                    .with_pixel_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ostromoukhov_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                OstromoukhovDither::new(Palette::black_and_white())
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "ostromoukhov_sampling", effects);

    let effects = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            OstromoukhovDither::new(Palette::black_and_white())
                .with_pixel_size(4, 4)
                .unwrap()
                .with_grid_offset(x, y),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "ostromoukhov_grid_offset", effects);
}

fn riemersma(criterion: &mut Criterion) {
    let effects = PIXEL_SIZES
        .into_iter()
        .map(|(name, (width, height))| {
            boxed(
                name,
                RiemersmaDither::new(Palette::black_and_white())
                    .with_pixel_size(width, height)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "riemersma_pixel_size", effects);

    let effects = SAMPLING_MODES
        .into_iter()
        .map(|(name, sampling)| {
            boxed(
                name,
                RiemersmaDither::new(Palette::black_and_white())
                    .with_pixel_size(4, 4)
                    .unwrap()
                    .with_sampling(sampling),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "riemersma_sampling", effects);

    let effects = [("boundary", 1), ("default", 16), ("high", 64)]
        .into_iter()
        .map(|(name, history)| {
            boxed(
                name,
                RiemersmaDither::new(Palette::black_and_white())
                    .with_history_length(history)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "riemersma_history_length", effects);

    let effects = [("boundary", 1.0), ("default", 16.0), ("high", 64.0)]
        .into_iter()
        .map(|(name, decay)| {
            boxed(
                name,
                RiemersmaDither::new(Palette::black_and_white())
                    .with_decay(decay)
                    .unwrap(),
            )
        })
        .collect();
    bench_effect_matrix(criterion, "riemersma_decay", effects);

    let effects = [
        ("negative", (-8, -8)),
        ("default", (0, 0)),
        ("positive", (7, 5)),
    ]
    .into_iter()
    .map(|(name, (x, y))| {
        boxed(
            name,
            RiemersmaDither::new(Palette::black_and_white())
                .with_pixel_size(4, 4)
                .unwrap()
                .with_grid_offset(x, y),
        )
    })
    .collect();
    bench_effect_matrix(criterion, "riemersma_grid_offset", effects);
}
