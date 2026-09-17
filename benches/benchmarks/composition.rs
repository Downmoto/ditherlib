use std::hint::black_box;

use criterion::{Criterion, Throughput};
use ditherlib::{
    Mask, Pipeline, Point, Polygon, Renderer, Selection,
    effects::{
        blur::Blur,
        dither::{
            colour::ColourSpace,
            diffusion::{DiffusionAlgorithm, ErrorDiffusion},
            noise::{NoiseAlgorithm, NoiseDither},
            palette::{Palette, PaletteMatchMode, PaletteSize},
            threshold::{OrderedDither, Threshold, ThresholdMap},
        },
        greyscale::Greyscale,
    },
};

use super::support::{OPTION_DIMENSIONS, bench_effect, option_fixture};

pub fn benchmarks(criterion: &mut Criterion) {
    palettes(criterion);
    selections(criterion);
    pipelines(criterion);
    renderer_reuse(criterion);
}

fn palettes(criterion: &mut Criterion) {
    palette_construction(criterion);
    palette_matching(criterion);
    palette_presets(criterion);
}

fn palette_construction(criterion: &mut Criterion) {
    let source = option_fixture();
    let colours = Palette::pico_8().colours().to_vec();
    let mut group = criterion.benchmark_group("palette_construction");

    group.bench_function("custom", |bencher| {
        bencher.iter(|| Palette::new(black_box(&colours)).unwrap());
    });
    for (name, size) in [
        ("all", PaletteSize::All),
        ("limited_1", PaletteSize::Limited(1)),
        ("limited_8", PaletteSize::Limited(8)),
        ("limited_64", PaletteSize::Limited(64)),
    ] {
        group.bench_function(name, |bencher| {
            bencher.iter(|| Palette::from_source(black_box(&source), black_box(size)).unwrap());
        });
    }

    group.finish();
}

fn palette_matching(criterion: &mut Criterion) {
    let source = option_fixture();
    let colours = source
        .rgba8_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<Vec<_>>();
    let colour_spaces = [
        ("rgb", ColourSpace::Rgb),
        ("linear_rgb", ColourSpace::LinearRgb),
        ("oklab", ColourSpace::Oklab),
    ];
    let matching_modes = [
        ("colour", PaletteMatchMode::Colour),
        ("luminance", PaletteMatchMode::Luminance),
    ];
    let mut group = criterion.benchmark_group("palette_matching");
    group.throughput(Throughput::Elements(colours.len() as u64));

    for (space_name, colour_space) in colour_spaces {
        for (mode_name, matching_mode) in matching_modes {
            let palette = Palette::pico_8()
                .with_colour_space(colour_space)
                .with_matching_mode(matching_mode);
            group.bench_function(format!("{space_name}/{mode_name}"), |bencher| {
                bencher.iter(|| {
                    for &colour in black_box(&colours) {
                        black_box(palette.nearest_colour(black_box(colour)));
                    }
                });
            });
        }
    }

    group.finish();
}

fn palette_presets(criterion: &mut Criterion) {
    let source = option_fixture();
    let selection = Selection::All;
    let mut group = criterion.benchmark_group("palette_presets");
    let palettes = [
        ("black_and_white", Palette::black_and_white()),
        ("greyscale", Palette::greyscale()),
        ("game_boy", Palette::game_boy()),
        ("cga", Palette::cga()),
        ("pico_8", Palette::pico_8()),
        ("monochrome", Palette::monochrome([255, 64, 32])),
        (
            "custom",
            Palette::new([[8, 24, 32], [52, 104, 86], [224, 224, 192]]).unwrap(),
        ),
        (
            "derived",
            Palette::from_source(&source, PaletteSize::Limited(16)).unwrap(),
        ),
    ];

    for (name, palette) in palettes {
        let effect = Threshold::new(palette);
        bench_effect(&mut group, name, &source, &effect, &selection);
    }

    group.finish();
}

fn selections(criterion: &mut Criterion) {
    let source = option_fixture();
    let effect = Threshold::new(Palette::pico_8());
    let (width, height) = OPTION_DIMENSIONS;
    let mut group = criterion.benchmark_group("selections");
    let selections = [
        ("all", Selection::All),
        (
            "polygon",
            Selection::Polygon(
                Polygon::rectangle(
                    Point::new(8.25, 8.25),
                    width as f32 - 16.5,
                    height as f32 - 16.5,
                )
                .unwrap(),
            ),
        ),
        (
            "mask",
            Selection::Mask(
                Mask::new(
                    width,
                    height,
                    (0..width * height)
                        .map(|index| if index % 3 == 0 { 128 } else { 255 })
                        .collect(),
                )
                .unwrap(),
            ),
        ),
    ];

    for (name, selection) in &selections {
        bench_effect(&mut group, name, &source, &effect, selection);
    }

    group.finish();
}

fn pipelines(criterion: &mut Criterion) {
    let source = option_fixture();
    let mut group = criterion.benchmark_group("pipelines");
    group.throughput(Throughput::Elements(
        u64::from(source.width()) * u64::from(source.height()),
    ));

    let pipelines = [
        ("0_steps", Pipeline::new()),
        ("1_step", one_step_pipeline()),
        ("3_steps", three_step_pipeline()),
        ("6_steps", six_step_pipeline()),
    ];
    for (name, pipeline) in &pipelines {
        let mut renderer = Renderer::new();
        group.bench_function(*name, |bencher| {
            bencher.iter(|| {
                black_box(
                    renderer
                        .render_pipeline(black_box(&source), black_box(pipeline))
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn one_step_pipeline() -> Pipeline {
    let mut pipeline = Pipeline::new();
    pipeline.add(Greyscale, Selection::All);
    pipeline
}

fn three_step_pipeline() -> Pipeline {
    let mut pipeline = one_step_pipeline();
    pipeline.add(
        OrderedDither::new(Palette::pico_8(), ThresholdMap::bayer_4x4()),
        Selection::All,
    );
    pipeline.add(Blur::new(2.0).unwrap(), Selection::All);
    pipeline
}

fn six_step_pipeline() -> Pipeline {
    let mut pipeline = one_step_pipeline();
    pipeline.add(
        Threshold::new(Palette::pico_8())
            .with_pixel_size(2, 2)
            .unwrap(),
        Selection::All,
    );
    pipeline.add(
        OrderedDither::new(Palette::pico_8(), ThresholdMap::blue_noise_16x16()),
        Selection::All,
    );
    pipeline.add(
        NoiseDither::new(Palette::pico_8(), NoiseAlgorithm::Blue),
        Selection::All,
    );
    pipeline.add(
        ErrorDiffusion::new(Palette::pico_8(), DiffusionAlgorithm::FloydSteinberg),
        Selection::All,
    );
    pipeline.add(Blur::new(2.0).unwrap(), Selection::All);
    pipeline
}

fn renderer_reuse(criterion: &mut Criterion) {
    let source = option_fixture();
    let effect = OrderedDither::new(Palette::pico_8(), ThresholdMap::bayer_4x4());
    let selection = Selection::All;
    let mut group = criterion.benchmark_group("renderer_reuse");
    group.throughput(Throughput::Elements(
        u64::from(source.width()) * u64::from(source.height()),
    ));

    let mut renderer = Renderer::new();
    group.bench_function("reused", |bencher| {
        bencher.iter(|| {
            black_box(
                renderer
                    .render(
                        black_box(&source),
                        black_box(&effect),
                        black_box(&selection),
                    )
                    .unwrap(),
            )
        });
    });
    group.bench_function("fresh", |bencher| {
        bencher.iter(|| {
            black_box(
                Renderer::new()
                    .render(
                        black_box(&source),
                        black_box(&effect),
                        black_box(&selection),
                    )
                    .unwrap(),
            )
        });
    });

    group.finish();
}
