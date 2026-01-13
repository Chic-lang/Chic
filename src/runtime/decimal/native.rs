#![allow(unsafe_code)]

use super::abi::DecimalRuntimeStatus;
use super::convert::{
    Decimal128Parts, DecimalConstPtr, DecimalMutPtr, DecimalRoundingAbi, DecimalRuntimeResult,
};

unsafe extern "C" {
    pub fn chic_rt_decimal_add(
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_add_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_sub(
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_sub_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_mul(
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_mul_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_div(
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_div_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_rem(
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_rem_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        rhs: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_fma(
        lhs: *const Decimal128Parts,
        multiplicand: *const Decimal128Parts,
        addend: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_fma_out(
        out: *mut DecimalRuntimeResult,
        lhs: *const Decimal128Parts,
        multiplicand: *const Decimal128Parts,
        addend: *const Decimal128Parts,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_clone(src: DecimalConstPtr, dest: DecimalMutPtr)
    -> DecimalRuntimeStatus;
    pub fn chic_rt_decimal_sum(
        values: DecimalConstPtr,
        len: usize,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_sum_out(
        out: *mut DecimalRuntimeResult,
        values: DecimalConstPtr,
        len: usize,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_dot(
        lhs: DecimalConstPtr,
        rhs: DecimalConstPtr,
        len: usize,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeResult;
    pub fn chic_rt_decimal_dot_out(
        out: *mut DecimalRuntimeResult,
        lhs: DecimalConstPtr,
        rhs: DecimalConstPtr,
        len: usize,
        rounding: DecimalRoundingAbi,
        flags: u32,
    );
    pub fn chic_rt_decimal_matmul(
        left: DecimalConstPtr,
        left_rows: usize,
        left_cols: usize,
        right: DecimalConstPtr,
        right_cols: usize,
        destination: DecimalMutPtr,
        rounding: DecimalRoundingAbi,
        flags: u32,
    ) -> DecimalRuntimeStatus;
}
