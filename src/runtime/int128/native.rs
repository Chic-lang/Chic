#![allow(unsafe_code)]

use crate::runtime::int128::{Int128Parts, UInt128Parts};

// External bindings to the Chic-native int128 intrinsics. When the native
// runtime archive is linked, these symbols are provided by the Chic runtime
// and the Rust implementations are skipped.
unsafe extern "C" {
    pub fn chic_rt_i128_add(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_sub(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_mul(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_div(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_rem(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_eq(lhs: *const Int128Parts, rhs: *const Int128Parts) -> i32;
    pub fn chic_rt_i128_neg(out: *mut Int128Parts, value: *const Int128Parts);
    pub fn chic_rt_i128_not(out: *mut Int128Parts, value: *const Int128Parts);
    pub fn chic_rt_i128_shl(out: *mut Int128Parts, lhs: *const Int128Parts, amount: u32);
    pub fn chic_rt_i128_shr(out: *mut Int128Parts, lhs: *const Int128Parts, amount: u32);
    pub fn chic_rt_i128_cmp(lhs: *const Int128Parts, rhs: *const Int128Parts) -> i32;
    pub fn chic_rt_i128_and(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );
    pub fn chic_rt_i128_or(out: *mut Int128Parts, lhs: *const Int128Parts, rhs: *const Int128Parts);
    pub fn chic_rt_i128_xor(
        out: *mut Int128Parts,
        lhs: *const Int128Parts,
        rhs: *const Int128Parts,
    );

    pub fn chic_rt_u128_add(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_sub(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_mul(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_div(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_rem(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_eq(lhs: *const UInt128Parts, rhs: *const UInt128Parts) -> i32;
    pub fn chic_rt_u128_not(out: *mut UInt128Parts, value: *const UInt128Parts);
    pub fn chic_rt_u128_shl(out: *mut UInt128Parts, lhs: *const UInt128Parts, amount: u32);
    pub fn chic_rt_u128_shr(out: *mut UInt128Parts, lhs: *const UInt128Parts, amount: u32);
    pub fn chic_rt_u128_cmp(lhs: *const UInt128Parts, rhs: *const UInt128Parts) -> i32;
    pub fn chic_rt_u128_and(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_or(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
    pub fn chic_rt_u128_xor(
        out: *mut UInt128Parts,
        lhs: *const UInt128Parts,
        rhs: *const UInt128Parts,
    );
}
