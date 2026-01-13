#![cfg_attr(chic_native_runtime, allow(dead_code))]

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalRuntimeStatus {
    Success = 0,
    Overflow = 1,
    DivideByZero = 2,
    InvalidRounding = 3,
    InvalidFlags = 4,
    InvalidPointer = 5,
    InvalidOperand = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalIntrinsicVariant {
    Scalar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalIntrinsicEntry {
    pub symbol: &'static str,
    pub variant: DecimalIntrinsicVariant,
}

pub const DECIMAL_INTRINSICS: &[DecimalIntrinsicEntry] = &[
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_add",
        variant: DecimalIntrinsicVariant::Scalar,
    },
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_sub",
        variant: DecimalIntrinsicVariant::Scalar,
    },
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_mul",
        variant: DecimalIntrinsicVariant::Scalar,
    },
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_div",
        variant: DecimalIntrinsicVariant::Scalar,
    },
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_rem",
        variant: DecimalIntrinsicVariant::Scalar,
    },
    DecimalIntrinsicEntry {
        symbol: "chic_rt_decimal_fma",
        variant: DecimalIntrinsicVariant::Scalar,
    },
];
