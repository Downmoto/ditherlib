#[path = "benchmarks/composition.rs"]
mod composition;
#[path = "benchmarks/effects.rs"]
mod effects;
#[path = "benchmarks/support.rs"]
mod support;

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(250));
    targets = effects::benchmarks, composition::benchmarks
}
criterion_main!(benches);
