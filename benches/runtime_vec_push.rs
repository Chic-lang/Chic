use std::mem::{align_of, size_of};

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{
    VecError, chic_rt_vec_clear, chic_rt_vec_drop, chic_rt_vec_new, chic_rt_vec_pop,
    chic_rt_vec_push,
};

const INLINE_BYTES: usize = 64;

fn bench_inline_push_pop(c: &mut Criterion) {
    c.bench_function("runtime_vec_inline_push_pop", |b| {
        b.iter(|| {
            let mut vec = unsafe { chic_rt_vec_new(size_of::<u64>(), align_of::<u64>(), None) };
            let inline_cap = INLINE_BYTES / size_of::<u64>();
            for value in 0..inline_cap {
                let val = value as u64;
                let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr(&val)) };
                assert_eq!(status, VecError::Success as i32);
            }
            for _ in 0..inline_cap {
                let mut out = 0u64;
                let status = unsafe { chic_rt_vec_pop(&mut vec, mut_ptr(&mut out)) };
                assert_eq!(status, VecError::Success as i32);
                black_box(out);
            }
            unsafe { chic_rt_vec_drop(&mut vec) };
        });
    });
}

fn bench_heap_push_pop(c: &mut Criterion) {
    c.bench_function("runtime_vec_heap_push_pop", |b| {
        b.iter(|| {
            let mut vec = unsafe { chic_rt_vec_new(size_of::<u64>(), align_of::<u64>(), None) };
            let heap_count = INLINE_BYTES / size_of::<u64>() + 64;
            for value in 0..heap_count {
                let val = value as u64;
                let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr(&val)) };
                assert_eq!(status, VecError::Success as i32);
            }
            for _ in 0..heap_count {
                let mut out = 0u64;
                let status = unsafe { chic_rt_vec_pop(&mut vec, mut_ptr(&mut out)) };
                assert_eq!(status, VecError::Success as i32);
                black_box(out);
            }
            unsafe { chic_rt_vec_drop(&mut vec) };
        });
    });
}

fn bench_reserve_clear(c: &mut Criterion) {
    c.bench_function("runtime_vec_inline_clear", |b| {
        b.iter(|| {
            let mut vec = unsafe { chic_rt_vec_new(size_of::<u32>(), align_of::<u32>(), None) };
            let inline_cap = INLINE_BYTES / size_of::<u32>();
            for _ in 0..inline_cap {
                let val: u32 = 42;
                let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr(&val)) };
                assert_eq!(status, VecError::Success as i32);
            }
            unsafe { chic_rt_vec_clear(&mut vec) };
            unsafe { chic_rt_vec_drop(&mut vec) };
        });
    });
}

fn runtime_vec_benchmarks(c: &mut Criterion) {
    bench_inline_push_pop(c);
    bench_heap_push_pop(c);
    bench_reserve_clear(c);
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

criterion_group!(runtime_vec, runtime_vec_benchmarks);
criterion_main!(runtime_vec);
