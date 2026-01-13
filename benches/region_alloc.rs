use std::mem::{align_of, size_of};

use chic::runtime::region::{chic_rt_region_alloc, chic_rt_region_enter, chic_rt_region_exit};
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{
    VecError, chic_rt_vec_drop, chic_rt_vec_pop, chic_rt_vec_push,
    chic_rt_vec_with_capacity_in_region,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

const BLOCK_SIZE: usize = 4096;

fn region_block_alloc(c: &mut Criterion) {
    c.bench_function("region_block_alloc_4k_x128", |b| {
        b.iter(|| {
            let region = unsafe { chic_rt_region_enter(1729) };
            for _ in 0..128 {
                let handle = unsafe { chic_rt_region_alloc(region, BLOCK_SIZE, 8) };
                black_box(handle.ptr);
            }
            unsafe { chic_rt_region_exit(region) };
        });
    });
}

fn region_vec_push_drop(c: &mut Criterion) {
    c.bench_function("region_vec_push_drop_u64_1k", |b| {
        b.iter(|| {
            let region = unsafe { chic_rt_region_enter(4242) };
            let mut vec = unsafe {
                chic_rt_vec_with_capacity_in_region(
                    size_of::<u64>(),
                    align_of::<u64>(),
                    1024,
                    None,
                    region,
                )
            };
            for value in 0..1024 {
                let val = value as u64;
                let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr(&val)) };
                assert_eq!(status, VecError::Success as i32);
            }
            for _ in 0..1024 {
                let mut out = 0u64;
                let status = unsafe { chic_rt_vec_pop(&mut vec, mut_ptr(&mut out)) };
                assert_eq!(status, VecError::Success as i32);
                black_box(out);
            }
            unsafe { chic_rt_vec_drop(&mut vec) };
            unsafe { chic_rt_region_exit(region) };
        });
    });
}

fn const_ptr<T>(value: &T) -> ValueConstPtr {
    ValueConstPtr {
        ptr: (value as *const T).cast(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}

fn mut_ptr<T>(value: &mut T) -> ValueMutPtr {
    ValueMutPtr {
        ptr: (value as *mut T).cast(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}

fn region_benches(c: &mut Criterion) {
    region_block_alloc(c);
    region_vec_push_drop(c);
}

criterion_group!(region_alloc, region_benches);
criterion_main!(region_alloc);
