use criterion::Criterion;
use std::hint::black_box;

use crate::bench_common;

pub fn bench(c: &mut Criterion) {
    let data = bench_common::dataset_u32();

    c.bench_function("traits::collections::manual_vec", |b| {
        b.iter(|| black_box(fill_vec(data)));
    });

    c.bench_function("traits::collections::generic_bag", |b| {
        b.iter(|| {
            let mut bag = VecBag::default();
            black_box(fill_bag_generic(&mut bag, data));
        });
    });

    c.bench_function("traits::collections::dyn_bag", |b| {
        b.iter(|| {
            let mut bag = VecBag::default();
            let dyn_bag: &mut dyn Bag = &mut bag;
            black_box(fill_bag_dyn(dyn_bag, data));
        });
    });
}

trait Bag {
    fn add(&mut self, value: u32);
    fn finish(&mut self) -> u64;
}

#[derive(Default)]
struct VecBag {
    data: Vec<u32>,
}

impl Bag for VecBag {
    fn add(&mut self, value: u32) {
        self.data.push(value);
    }

    fn finish(&mut self) -> u64 {
        self.data.iter().map(|&value| value as u64).sum()
    }
}

fn fill_vec(data: &[u32]) -> u64 {
    let mut vec = Vec::with_capacity(data.len());
    vec.extend_from_slice(data);
    vec.iter().map(|&value| value as u64).sum()
}

fn fill_bag_generic<T: Bag>(bag: &mut T, data: &[u32]) -> u64 {
    for &value in data {
        bag.add(value);
    }
    bag.finish()
}

fn fill_bag_dyn(bag: &mut dyn Bag, data: &[u32]) -> u64 {
    for &value in data {
        bag.add(value);
    }
    bag.finish()
}
