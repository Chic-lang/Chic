#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; decimal runtime lives in the native runtime."
);

mod abi;
mod convert;
mod native;

pub use abi::{
    DECIMAL_INTRINSICS, DecimalIntrinsicEntry, DecimalIntrinsicVariant, DecimalRuntimeStatus,
};
pub use convert::{
    Decimal128Parts, DecimalConstPtr, DecimalMutPtr, DecimalRoundingAbi, DecimalRuntimeResult,
};
pub use native::{
    chic_rt_decimal_add, chic_rt_decimal_add_out, chic_rt_decimal_clone, chic_rt_decimal_div,
    chic_rt_decimal_div_out, chic_rt_decimal_dot, chic_rt_decimal_dot_out, chic_rt_decimal_fma,
    chic_rt_decimal_fma_out, chic_rt_decimal_matmul, chic_rt_decimal_mul, chic_rt_decimal_mul_out,
    chic_rt_decimal_rem, chic_rt_decimal_rem_out, chic_rt_decimal_sub, chic_rt_decimal_sub_out,
    chic_rt_decimal_sum, chic_rt_decimal_sum_out,
};
