use std::hint::black_box;

use criterion::{BenchmarkGroup, Criterion, Throughput, measurement::WallTime};
use ditherlib::{Effect, Renderer, Selection, SourceImage};

pub const OPTION_DIMENSIONS: (u32, u32) = (128, 128);
pub const RESOLUTIONS: [(&str, (u32, u32)); 3] = [
    ("64x64", (64, 64)),
    ("256x256", (256, 256)),
    ("1024x1024", (1024, 1024)),
];
pub const PIXEL_SIZES: [(&str, (u32, u32)); 5] = [
    ("1x1", (1, 1)),
    ("2x2", (2, 2)),
    ("4x4", (4, 4)),
    ("8x8", (8, 8)),
    ("4x8", (4, 8)),
];

pub fn fixture((width, height): (u32, u32)) -> SourceImage {
    let length = usize::try_from(u64::from(width) * u64::from(height) * 4).unwrap();
    let mut pixels = Vec::with_capacity(length);
    let x_denominator = width.saturating_sub(1).max(1);
    let y_denominator = height.saturating_sub(1).max(1);

    for y in 0..height {
        for x in 0..width {
            let red = (x * 255 / x_denominator) as u8;
            let green = (y * 255 / y_denominator) as u8;
            let blue = x
                .wrapping_mul(37)
                .wrapping_add(y.wrapping_mul(91))
                .wrapping_add(x.wrapping_mul(y).rotate_left(3)) as u8;
            pixels.extend_from_slice(&[red, green, blue, 255]);
        }
    }

    SourceImage::from_rgba8(width, height, pixels).unwrap()
}

pub fn option_fixture() -> SourceImage {
    fixture(OPTION_DIMENSIONS)
}

pub fn bench_effect(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    source: &SourceImage,
    effect: &dyn Effect,
    selection: &Selection,
) {
    group.throughput(Throughput::Elements(
        u64::from(source.width()) * u64::from(source.height()),
    ));
    let mut renderer = Renderer::new();

    group.bench_function(name, |bencher| {
        bencher.iter(|| {
            black_box(
                renderer
                    .render(black_box(source), black_box(effect), black_box(selection))
                    .unwrap(),
            )
        });
    });
}

pub fn bench_effect_matrix(
    criterion: &mut Criterion,
    group_name: &str,
    effects: Vec<(String, Box<dyn Effect>)>,
) {
    let source = option_fixture();
    let selection = Selection::All;
    let mut group = criterion.benchmark_group(group_name);

    for (name, effect) in &effects {
        bench_effect(&mut group, name, &source, effect.as_ref(), &selection);
    }

    group.finish();
}
