use std::sync::OnceLock;

use chic::decimal::{DECIMAL_FLAG_VECTORIZE, Decimal128, DecimalRoundingMode};
use chic::runtime::decimal::{
    Decimal128Parts, DecimalConstPtr, DecimalMutPtr, DecimalRoundingAbi, DecimalRuntimeStatus,
    chic_rt_decimal_dot, chic_rt_decimal_dot_simd, chic_rt_decimal_matmul,
    chic_rt_decimal_matmul_simd, chic_rt_decimal_sum, chic_rt_decimal_sum_simd,
};
use chic::support::cpu::{self, CpuFeatures};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const MAT_ROWS: usize = 32;
const MAT_SHARED: usize = 32;
const MAT_COLS: usize = 32;

fn to_parts(value: Decimal128) -> Decimal128Parts {
    let [lo, mid, hi, flags] = value.to_bits();
    Decimal128Parts { lo, mid, hi, flags }
}

fn sum_dataset() -> &'static [Decimal128Parts] {
    static DATA: OnceLock<Vec<Decimal128Parts>> = OnceLock::new();
    DATA.get_or_init(|| {
        (0..1024)
            .map(|idx| {
                let value = ((idx as i32 * 97) - 128) as i128;
                Decimal128::from_i128(value).expect("decimal conversion")
            })
            .map(to_parts)
            .collect()
    })
    .as_slice()
}

fn dot_dataset() -> (&'static [Decimal128Parts], &'static [Decimal128Parts]) {
    static DATA: OnceLock<(Vec<Decimal128Parts>, Vec<Decimal128Parts>)> = OnceLock::new();
    let pair = DATA.get_or_init(|| {
        let lhs = (0..2048)
            .map(|idx| {
                let value = ((idx as i32 * 37) - 256) as i128;
                Decimal128::from_i128(value).expect("decimal conversion")
            })
            .map(to_parts)
            .collect::<Vec<_>>();
        let rhs = (0..2048)
            .map(|idx| {
                let value = ((idx as i32 * 53) - 128) as i128;
                Decimal128::from_i128(value).expect("decimal conversion")
            })
            .map(to_parts)
            .collect::<Vec<_>>();
        (lhs, rhs)
    });
    (pair.0.as_slice(), pair.1.as_slice())
}

fn matmul_dataset() -> (&'static [Decimal128Parts], &'static [Decimal128Parts]) {
    static DATA: OnceLock<(Vec<Decimal128Parts>, Vec<Decimal128Parts>)> = OnceLock::new();
    let pair = DATA.get_or_init(|| {
        let left = (0..(MAT_ROWS * MAT_SHARED))
            .map(|idx| {
                let value = ((idx as i32 * 11) - 64) as i128;
                Decimal128::from_i128(value).expect("decimal conversion")
            })
            .map(to_parts)
            .collect::<Vec<_>>();
        let right = (0..(MAT_SHARED * MAT_COLS))
            .map(|idx| {
                let value = ((idx as i32 * 17) - 32) as i128;
                Decimal128::from_i128(value).expect("decimal conversion")
            })
            .map(to_parts)
            .collect::<Vec<_>>();
        (left, right)
    });
    (pair.0.as_slice(), pair.1.as_slice())
}

fn rounding(mode: DecimalRoundingMode) -> DecimalRoundingAbi {
    DecimalRoundingAbi {
        value: mode.as_discriminant(),
    }
}

fn const_ptr(slice: &[Decimal128Parts]) -> DecimalConstPtr {
    DecimalConstPtr {
        ptr: slice.as_ptr(),
    }
}

fn mut_ptr(slice: &mut [Decimal128Parts]) -> DecimalMutPtr {
    DecimalMutPtr {
        ptr: slice.as_mut_ptr(),
    }
}

fn bench_decimal_sum_scalar(c: &mut Criterion) {
    let data = sum_dataset();
    c.bench_function("decimal_sum_scalar", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::none());
        b.iter(|| {
            let result = chic_rt_decimal_sum(
                const_ptr(data),
                data.len(),
                rounding(DecimalRoundingMode::TiesToEven),
                0,
            );
            assert_eq!(result.status, DecimalRuntimeStatus::Success);
            black_box(result.value);
        });
        drop(guard);
    });
}

fn bench_decimal_sum_simd(c: &mut Criterion) {
    let data = sum_dataset();
    c.bench_function("decimal_sum_simd", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::new(true, true, true, true));
        b.iter(|| {
            let result = chic_rt_decimal_sum_simd(
                const_ptr(data),
                data.len(),
                rounding(DecimalRoundingMode::TiesToEven),
                DECIMAL_FLAG_VECTORIZE,
            );
            assert_eq!(result.status, DecimalRuntimeStatus::Success);
            black_box(result.value);
        });
        drop(guard);
    });
}

fn bench_decimal_dot_scalar(c: &mut Criterion) {
    let (lhs, rhs) = dot_dataset();
    c.bench_function("decimal_dot_scalar", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::none());
        b.iter(|| {
            let result = chic_rt_decimal_dot(
                const_ptr(lhs),
                const_ptr(rhs),
                lhs.len(),
                rounding(DecimalRoundingMode::TiesToEven),
                0,
            );
            assert_eq!(result.status, DecimalRuntimeStatus::Success);
            black_box(result.value);
        });
        drop(guard);
    });
}

fn bench_decimal_dot_simd(c: &mut Criterion) {
    let (lhs, rhs) = dot_dataset();
    c.bench_function("decimal_dot_simd", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::new(true, true, true, true));
        b.iter(|| {
            let result = chic_rt_decimal_dot_simd(
                const_ptr(lhs),
                const_ptr(rhs),
                lhs.len(),
                rounding(DecimalRoundingMode::TiesToEven),
                DECIMAL_FLAG_VECTORIZE,
            );
            assert_eq!(result.status, DecimalRuntimeStatus::Success);
            black_box(result.value);
        });
        drop(guard);
    });
}

fn bench_decimal_matmul_scalar(c: &mut Criterion) {
    let (left, right) = matmul_dataset();
    c.bench_function("decimal_matmul_scalar", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::none());
        b.iter(|| {
            let mut dest = vec![to_parts(Decimal128::zero()); MAT_ROWS * MAT_COLS];
            let status = chic_rt_decimal_matmul(
                const_ptr(left),
                MAT_ROWS,
                MAT_SHARED,
                const_ptr(right),
                MAT_COLS,
                mut_ptr(dest.as_mut_slice()),
                rounding(DecimalRoundingMode::TiesToEven),
                0,
            );
            assert_eq!(status, DecimalRuntimeStatus::Success);
            black_box(&dest);
        });
        drop(guard);
    });
}

fn bench_decimal_matmul_simd(c: &mut Criterion) {
    let (left, right) = matmul_dataset();
    c.bench_function("decimal_matmul_simd", |b| {
        let guard = cpu::override_for_testing(CpuFeatures::new(true, true, true, true));
        b.iter(|| {
            let mut dest = vec![to_parts(Decimal128::zero()); MAT_ROWS * MAT_COLS];
            let status = chic_rt_decimal_matmul_simd(
                const_ptr(left),
                MAT_ROWS,
                MAT_SHARED,
                const_ptr(right),
                MAT_COLS,
                mut_ptr(dest.as_mut_slice()),
                rounding(DecimalRoundingMode::TiesToEven),
                DECIMAL_FLAG_VECTORIZE,
            );
            assert_eq!(status, DecimalRuntimeStatus::Success);
            black_box(&dest);
        });
        drop(guard);
    });
}

fn decimal_fast_benches(c: &mut Criterion) {
    bench_decimal_sum_scalar(c);
    bench_decimal_sum_simd(c);
    bench_decimal_dot_scalar(c);
    bench_decimal_dot_simd(c);
    bench_decimal_matmul_scalar(c);
    bench_decimal_matmul_simd(c);
}

criterion_group!(decimal_fast, decimal_fast_benches);
criterion_main!(decimal_fast);
