use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use chic::runtime::span::{ChicReadOnlySpan, ChicSpan, chic_rt_span_copy_to};
use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

fn bench_span_copy(c: &mut Criterion) {
    c.bench_function("runtime_span_copy_256_bytes", |b| {
        b.iter(|| {
            let mut source_buf = [0u8; 256];
            let mut dest_buf = [0u8; 256];
            for (index, slot) in source_buf.iter_mut().enumerate() {
                *slot = (index & 0xFF) as u8;
            }
            let source = ChicReadOnlySpan {
                data: ValueConstPtr {
                    ptr: source_buf.as_ptr(),
                    size: source_buf.len(),
                    align: 1,
                },
                len: source_buf.len(),
                elem_size: 1,
                elem_align: 1,
            };
            let dest = ChicSpan {
                data: ValueMutPtr {
                    ptr: dest_buf.as_mut_ptr(),
                    size: dest_buf.len(),
                    align: 1,
                },
                len: dest_buf.len(),
                elem_size: 1,
                elem_align: 1,
            };
            let status = unsafe { chic_rt_span_copy_to(&source, &dest) };
            assert_eq!(status, 0);
            black_box(dest_buf[0]);
        });
    });
}

criterion_group!(runtime_span_stackalloc, bench_span_copy);
criterion_main!(runtime_span_stackalloc);
