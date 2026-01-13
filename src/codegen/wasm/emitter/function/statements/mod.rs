mod assign;
mod block;
mod mmio;
mod rvalues;
mod strings;

use crate::mir::{FloatValue, FloatWidth};

#[derive(Clone, Copy)]
pub(super) enum InterpolatedOperandKind {
    Str,
    String,
    Bool,
    Char,
    SignedInt { bits: u32 },
    UnsignedInt { bits: u32 },
    Float { bits: u32 },
}

pub(super) fn short_type_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

pub(super) fn minimal_signed_bits(value: i128) -> u32 {
    if value >= -0x80 && value <= 0x7F {
        8
    } else if value >= -0x8000 && value <= 0x7FFF {
        16
    } else if value >= -0x8000_0000 && value <= 0x7FFF_FFFF {
        32
    } else if value >= -0x8000_0000_0000_0000 && value <= 0x7FFF_FFFF_FFFF_FFFF {
        64
    } else {
        128
    }
}

pub(super) fn minimal_unsigned_bits(value: u128) -> u32 {
    if value <= 0xFF {
        8
    } else if value <= 0xFFFF {
        16
    } else if value <= 0xFFFF_FFFF {
        32
    } else if value <= 0xFFFF_FFFF_FFFF_FFFF {
        64
    } else {
        128
    }
}

pub(super) fn float_constant_bits(value: FloatValue) -> u32 {
    match value.width {
        FloatWidth::F16 => 16,
        FloatWidth::F32 => 32,
        FloatWidth::F64 => 64,
        FloatWidth::F128 => 128,
    }
}
