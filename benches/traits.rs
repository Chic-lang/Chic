use criterion::{Criterion, criterion_group, criterion_main};

#[path = "traits/async.rs"]
mod async_bench;
#[path = "common/mod.rs"]
pub mod bench_common;
#[path = "traits/collections.rs"]
mod collections;
#[path = "traits/iter.rs"]
mod iter;

fn bench_iter(c: &mut Criterion) {
    iter::bench(c);
}

fn bench_collections(c: &mut Criterion) {
    collections::bench(c);
}

fn bench_async(c: &mut Criterion) {
    async_bench::bench(c);
}

criterion_group!(traits_group, bench_iter, bench_collections, bench_async);
criterion_main!(traits_group);
