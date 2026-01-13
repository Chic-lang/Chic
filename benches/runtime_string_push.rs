use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use chic::runtime::string::{
    ChicStr, ChicString, StringError, chic_rt_string_append_unsigned, chic_rt_string_drop,
    chic_rt_string_new, chic_rt_string_push_slice,
};

fn bench_push_inline(c: &mut Criterion) {
    let payload = vec![b'a'; ChicString::INLINE_CAPACITY - 4];
    let slice = ChicStr {
        ptr: payload.as_ptr(),
        len: payload.len(),
    };
    c.bench_function("runtime_string_push_inline", |b| {
        b.iter_batched(
            || unsafe { chic_rt_string_new() },
            |mut string| {
                let status = unsafe { chic_rt_string_push_slice(&mut string, slice) };
                assert_eq!(status, StringError::Success as i32);
                unsafe { chic_rt_string_drop(&mut string) };
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_push_heap(c: &mut Criterion) {
    let payload = vec![b'b'; 512];
    let slice = ChicStr {
        ptr: payload.as_ptr(),
        len: payload.len(),
    };
    c.bench_function("runtime_string_push_heap", |b| {
        b.iter_batched(
            || unsafe { chic_rt_string_new() },
            |mut string| {
                let status = unsafe { chic_rt_string_push_slice(&mut string, slice) };
                assert_eq!(status, StringError::Success as i32);
                unsafe { chic_rt_string_drop(&mut string) };
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_append_unsigned(c: &mut Criterion) {
    let slice = ChicStr::empty();
    c.bench_function("runtime_string_append_unsigned", |b| {
        b.iter_batched(
            || unsafe { chic_rt_string_new() },
            |mut string| {
                let status =
                    unsafe { chic_rt_string_append_unsigned(&mut string, 42, 0, 64, 0, 0, slice) };
                assert_eq!(status, StringError::Success as i32);
                unsafe { chic_rt_string_drop(&mut string) };
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    runtime_string_benches,
    bench_push_inline,
    bench_push_heap,
    bench_append_unsigned
);
criterion_main!(runtime_string_benches);
