use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_elementwise(c: &mut Criterion) {
    let len = 1_024usize;
    let mut a: Vec<f64> = (0..len).map(|i| (i % 17) as f64).collect();
    let b: Vec<f64> = (0..len).map(|i| (i % 31) as f64).collect();
    let mut out = vec![0.0f64; len];

    c.bench_function("ndarray_elementwise_add_scalar", |bench| {
        bench.iter(|| {
            for i in 0..len {
                out[i] = a[i] + 2.0f64;
            }
            black_box(out[0])
        });
    });

    c.bench_function("ndarray_elementwise_add_vector", |bench| {
        bench.iter(|| {
            for i in 0..len {
                out[i] = a[i] + b[i];
            }
            black_box(out[1])
        });
    });

    // Mutate to avoid constant-folding
    for slot in a.iter_mut() {
        *slot += 1.0;
    }
}

fn bench_matmul(c: &mut Criterion) {
    let n = 32usize;
    let mut a = vec![0.0f64; n * n];
    let mut b = vec![0.0f64; n * n];
    for row in 0..n {
        for col in 0..n {
            a[row * n + col] = (row + col) as f64;
            b[row * n + col] = (row * 2 + col) as f64;
        }
    }
    let mut out = vec![0.0f64; n * n];

    c.bench_function("ndarray_matmul_naive_32", |bench| {
        bench.iter(|| {
            for row in 0..n {
                for col in 0..n {
                    let mut acc = 0.0f64;
                    for k in 0..n {
                        acc += a[row * n + k] * b[k * n + col];
                    }
                    out[row * n + col] = acc;
                }
            }
            black_box(out[0])
        });
    });
}

criterion_group!(ndarray, bench_elementwise, bench_matmul);
criterion_main!(ndarray);
