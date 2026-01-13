use criterion::Criterion;
use std::hint::black_box;

use crate::bench_common;

pub fn bench(c: &mut Criterion) {
    let data = bench_common::dataset_u64();
    c.bench_function("traits::iter::manual_sum", |b| {
        b.iter(|| black_box(manual_sum(data)));
    });

    c.bench_function("traits::iter::generic_trait", |b| {
        b.iter(|| {
            let iter = SliceIter::new(data);
            black_box(sum_static(iter))
        });
    });

    c.bench_function("traits::iter::dyn_trait", |b| {
        b.iter(|| {
            let mut iter = SliceIter::new(data);
            let dyn_iter: &mut dyn DemoIter = &mut iter;
            black_box(sum_dynamic(dyn_iter))
        });
    });
}

trait DemoIter {
    fn next(&mut self) -> Option<u64>;
}

struct SliceIter<'a> {
    data: &'a [u64],
    index: usize,
}

impl<'a> SliceIter<'a> {
    fn new(data: &'a [u64]) -> Self {
        Self { data, index: 0 }
    }
}

impl<'a> DemoIter for SliceIter<'a> {
    fn next(&mut self) -> Option<u64> {
        if self.index >= self.data.len() {
            return None;
        }
        let value = self.data[self.index];
        self.index += 1;
        Some(value)
    }
}

fn manual_sum(data: &[u64]) -> u64 {
    data.iter().copied().sum()
}

fn sum_static<I: DemoIter>(mut iter: I) -> u64 {
    let mut acc = 0;
    while let Some(value) = iter.next() {
        acc += value;
    }
    acc
}

fn sum_dynamic(iter: &mut dyn DemoIter) -> u64 {
    let mut acc = 0;
    while let Some(value) = iter.next() {
        acc += value;
    }
    acc
}
