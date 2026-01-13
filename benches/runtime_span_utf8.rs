use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use chic::runtime::span::{ChicReadOnlySpan, ChicSpan, chic_rt_span_copy_to};
use chic::runtime::string::{
    ChicStr, ChicString, chic_rt_string_as_slice, chic_rt_string_from_slice,
};
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

fn bench_string_copy_utf8(c: &mut Criterion) {
    c.bench_function("runtime_string_copy_utf8_16_bytes", |b| {
        b.iter(|| {
            let slice = ChicStr {
                ptr: b"allocation-free".as_ptr(),
                len: 15,
            };
            let text: ChicString = unsafe { chic_rt_string_from_slice(slice) };
            let view = unsafe { chic_rt_string_as_slice(&text) };
            let readonly = ChicReadOnlySpan {
                data: ValueConstPtr {
                    ptr: view.ptr,
                    size: view.len,
                    align: 1,
                },
                len: view.len,
                elem_size: 1,
                elem_align: 1,
            };
            let mut buffer = [0u8; 32];
            let dest = ChicSpan {
                data: ValueMutPtr {
                    ptr: buffer.as_mut_ptr(),
                    size: buffer.len(),
                    align: 1,
                },
                len: buffer.len(),
                elem_size: 1,
                elem_align: 1,
            };
            let status = unsafe { chic_rt_span_copy_to(&readonly, &dest) };
            black_box(status);
            black_box(buffer[0]);
        });
    });
}

criterion_group!(runtime_span_utf8, bench_string_copy_utf8);
criterion_main!(runtime_span_utf8);
