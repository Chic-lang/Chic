use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::mem::{align_of, size_of};

use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{
    ChicVec, VecError, chic_rt_vec_drop, chic_rt_vec_elem_size, chic_rt_vec_get_ptr,
    chic_rt_vec_len, chic_rt_vec_new, chic_rt_vec_ptr_at, chic_rt_vec_push,
};

struct ManagedVec<T> {
    repr: ChicVec,
    _marker: std::marker::PhantomData<T>,
}

impl<T> ManagedVec<T> {
    fn new() -> Self {
        let repr = unsafe { chic_rt_vec_new(size_of::<T>(), align_of::<T>(), None) };
        Self {
            repr,
            _marker: std::marker::PhantomData,
        }
    }

    fn push(&mut self, value: T) {
        let status = unsafe { chic_rt_vec_push(&mut self.repr, value_const_ptr(&value)) };
        assert_eq!(status, VecError::Success as i32, "push should succeed");
    }

    fn as_ptr(&self) -> *const ChicVec {
        &self.repr
    }
}

impl<T> Drop for ManagedVec<T> {
    fn drop(&mut self) {
        unsafe { chic_rt_vec_drop(&mut self.repr) };
    }
}

#[inline(always)]
unsafe fn inline_vec_ptr_at(base_ptr: ValueMutPtr, elem_size: usize, index: usize) -> *mut u8 {
    let offset = index
        .checked_mul(elem_size)
        .expect("offset should not overflow");
    unsafe { base_ptr.ptr.add(offset) }
}

fn bench_runtime_ptr_at(c: &mut Criterion) {
    let mut vec = ManagedVec::<u32>::new();
    for value in 0..1024u32 {
        vec.push(value);
    }
    c.bench_function("vec_ptr_runtime", |b| {
        b.iter(|| unsafe {
            let mut acc = 0u64;
            for idx in 0..1024usize {
                let ptr = chic_rt_vec_ptr_at(vec.as_ptr(), idx).ptr as *const u32;
                acc += (*ptr) as u64;
            }
            black_box(acc)
        })
    });
}

fn bench_inline_ptr_at(c: &mut Criterion) {
    let mut vec = ManagedVec::<u32>::new();
    for value in 0..1024u32 {
        vec.push(value);
    }
    c.bench_function("vec_ptr_inline", |b| {
        b.iter(|| unsafe {
            let raw = vec.as_ptr();
            let len = chic_rt_vec_len(raw);
            let base_ptr = chic_rt_vec_get_ptr(raw);
            let elem_size = chic_rt_vec_elem_size(raw);
            let mut acc = 0u64;
            for idx in 0..1024usize {
                if idx >= len {
                    std::hint::unreachable_unchecked();
                }
                let ptr = inline_vec_ptr_at(base_ptr, elem_size, idx) as *const u32;
                acc += (*ptr) as u64;
            }
            black_box(acc)
        })
    });
}

criterion_group!(
    container_intrinsic_benches,
    bench_runtime_ptr_at,
    bench_inline_ptr_at
);
criterion_main!(container_intrinsic_benches);

fn value_const_ptr<T>(value: &T) -> ValueConstPtr {
    ValueConstPtr {
        ptr: (value as *const T).cast(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}
