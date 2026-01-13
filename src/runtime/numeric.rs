//! Numeric intrinsic metadata shared by codegen and runtime.
//!
//! This registry is intentionally metadata-only: the Chic numeric structs
//! already expose the intrinsic helpers (e.g. `Std.Int32.TryAdd`,
//! `Std.UInt64.RotateLeft`), but backends and interpreters need a canonical
//! list to drive intrinsic lowering and validation without hard-coding symbol
//! strings.

/// The logical width associated with a numeric intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericWidth {
    W8,
    W16,
    W32,
    W64,
    W128,
    Pointer,
}

/// Supported intrinsic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericIntrinsicKind {
    TryAdd,
    TrySub,
    TryMul,
    TryNeg,
    LeadingZeroCount,
    TrailingZeroCount,
    PopCount,
    RotateLeft,
    RotateRight,
    ReverseEndianness,
    IsPowerOfTwo,
}

/// Canonical description of a numeric intrinsic surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumericIntrinsicEntry {
    /// Fully qualified symbol for the Chic helper this intrinsic represents.
    pub symbol: &'static str,
    pub kind: NumericIntrinsicKind,
    pub width: NumericWidth,
    pub signed: bool,
    /// Number of value operands (excluding implicit `out` parameters).
    pub operands: u8,
}

macro_rules! intrinsic {
    ($symbol:expr, $kind:ident, $width:ident, $signed:expr, $operands:expr) => {
        NumericIntrinsicEntry {
            symbol: $symbol,
            kind: NumericIntrinsicKind::$kind,
            width: NumericWidth::$width,
            signed: $signed,
            operands: $operands,
        }
    };
}

macro_rules! intrinsic_for {
    ($ty:ident :: $method:ident, $kind:ident, $width:ident, $signed:expr, $operands:expr) => {
        intrinsic!(
            concat!("Std::", stringify!($ty), "::", stringify!($method)),
            $kind,
            $width,
            $signed,
            $operands
        )
    };
}

/// Canonical registry of numeric helpers surfaced as intrinsics.
pub static NUMERIC_INTRINSICS: &[NumericIntrinsicEntry] = &[
    intrinsic_for!(SByte::TryAdd, TryAdd, W8, true, 2),
    intrinsic_for!(SByte::TrySubtract, TrySub, W8, true, 2),
    intrinsic_for!(SByte::TryMultiply, TryMul, W8, true, 2),
    intrinsic_for!(SByte::TryNegate, TryNeg, W8, true, 1),
    intrinsic_for!(SByte::LeadingZeroCount, LeadingZeroCount, W8, true, 1),
    intrinsic_for!(SByte::TrailingZeroCount, TrailingZeroCount, W8, true, 1),
    intrinsic_for!(SByte::PopCount, PopCount, W8, true, 1),
    intrinsic_for!(SByte::RotateLeft, RotateLeft, W8, true, 2),
    intrinsic_for!(SByte::RotateRight, RotateRight, W8, true, 2),
    intrinsic_for!(SByte::ReverseEndianness, ReverseEndianness, W8, true, 1),
    intrinsic_for!(SByte::IsPowerOfTwo, IsPowerOfTwo, W8, true, 1),
    intrinsic_for!(Byte::TryAdd, TryAdd, W8, false, 2),
    intrinsic_for!(Byte::TrySubtract, TrySub, W8, false, 2),
    intrinsic_for!(Byte::TryMultiply, TryMul, W8, false, 2),
    intrinsic_for!(Byte::LeadingZeroCount, LeadingZeroCount, W8, false, 1),
    intrinsic_for!(Byte::TrailingZeroCount, TrailingZeroCount, W8, false, 1),
    intrinsic_for!(Byte::PopCount, PopCount, W8, false, 1),
    intrinsic_for!(Byte::RotateLeft, RotateLeft, W8, false, 2),
    intrinsic_for!(Byte::RotateRight, RotateRight, W8, false, 2),
    intrinsic_for!(Byte::ReverseEndianness, ReverseEndianness, W8, false, 1),
    intrinsic_for!(Byte::IsPowerOfTwo, IsPowerOfTwo, W8, false, 1),
    intrinsic_for!(Int16::TryAdd, TryAdd, W16, true, 2),
    intrinsic_for!(Int16::TrySubtract, TrySub, W16, true, 2),
    intrinsic_for!(Int16::TryMultiply, TryMul, W16, true, 2),
    intrinsic_for!(Int16::TryNegate, TryNeg, W16, true, 1),
    intrinsic_for!(Int16::LeadingZeroCount, LeadingZeroCount, W16, true, 1),
    intrinsic_for!(Int16::TrailingZeroCount, TrailingZeroCount, W16, true, 1),
    intrinsic_for!(Int16::PopCount, PopCount, W16, true, 1),
    intrinsic_for!(Int16::RotateLeft, RotateLeft, W16, true, 2),
    intrinsic_for!(Int16::RotateRight, RotateRight, W16, true, 2),
    intrinsic_for!(Int16::ReverseEndianness, ReverseEndianness, W16, true, 1),
    intrinsic_for!(Int16::IsPowerOfTwo, IsPowerOfTwo, W16, true, 1),
    intrinsic_for!(UInt16::TryAdd, TryAdd, W16, false, 2),
    intrinsic_for!(UInt16::TrySubtract, TrySub, W16, false, 2),
    intrinsic_for!(UInt16::TryMultiply, TryMul, W16, false, 2),
    intrinsic_for!(UInt16::LeadingZeroCount, LeadingZeroCount, W16, false, 1),
    intrinsic_for!(UInt16::TrailingZeroCount, TrailingZeroCount, W16, false, 1),
    intrinsic_for!(UInt16::PopCount, PopCount, W16, false, 1),
    intrinsic_for!(UInt16::RotateLeft, RotateLeft, W16, false, 2),
    intrinsic_for!(UInt16::RotateRight, RotateRight, W16, false, 2),
    intrinsic_for!(UInt16::ReverseEndianness, ReverseEndianness, W16, false, 1),
    intrinsic_for!(UInt16::IsPowerOfTwo, IsPowerOfTwo, W16, false, 1),
    intrinsic_for!(Int32::TryAdd, TryAdd, W32, true, 2),
    intrinsic_for!(Int32::TrySubtract, TrySub, W32, true, 2),
    intrinsic_for!(Int32::TryMultiply, TryMul, W32, true, 2),
    intrinsic_for!(Int32::TryNegate, TryNeg, W32, true, 1),
    intrinsic_for!(Int32::LeadingZeroCount, LeadingZeroCount, W32, true, 1),
    intrinsic_for!(Int32::TrailingZeroCount, TrailingZeroCount, W32, true, 1),
    intrinsic_for!(Int32::PopCount, PopCount, W32, true, 1),
    intrinsic_for!(Int32::RotateLeft, RotateLeft, W32, true, 2),
    intrinsic_for!(Int32::RotateRight, RotateRight, W32, true, 2),
    intrinsic_for!(Int32::ReverseEndianness, ReverseEndianness, W32, true, 1),
    intrinsic_for!(Int32::IsPowerOfTwo, IsPowerOfTwo, W32, true, 1),
    intrinsic_for!(UInt32::TryAdd, TryAdd, W32, false, 2),
    intrinsic_for!(UInt32::TrySubtract, TrySub, W32, false, 2),
    intrinsic_for!(UInt32::TryMultiply, TryMul, W32, false, 2),
    intrinsic_for!(UInt32::LeadingZeroCount, LeadingZeroCount, W32, false, 1),
    intrinsic_for!(UInt32::TrailingZeroCount, TrailingZeroCount, W32, false, 1),
    intrinsic_for!(UInt32::PopCount, PopCount, W32, false, 1),
    intrinsic_for!(UInt32::RotateLeft, RotateLeft, W32, false, 2),
    intrinsic_for!(UInt32::RotateRight, RotateRight, W32, false, 2),
    intrinsic_for!(UInt32::ReverseEndianness, ReverseEndianness, W32, false, 1),
    intrinsic_for!(UInt32::IsPowerOfTwo, IsPowerOfTwo, W32, false, 1),
    intrinsic_for!(Int64::TryAdd, TryAdd, W64, true, 2),
    intrinsic_for!(Int64::TrySubtract, TrySub, W64, true, 2),
    intrinsic_for!(Int64::TryMultiply, TryMul, W64, true, 2),
    intrinsic_for!(Int64::TryNegate, TryNeg, W64, true, 1),
    intrinsic_for!(Int64::LeadingZeroCount, LeadingZeroCount, W64, true, 1),
    intrinsic_for!(Int64::TrailingZeroCount, TrailingZeroCount, W64, true, 1),
    intrinsic_for!(Int64::PopCount, PopCount, W64, true, 1),
    intrinsic_for!(Int64::RotateLeft, RotateLeft, W64, true, 2),
    intrinsic_for!(Int64::RotateRight, RotateRight, W64, true, 2),
    intrinsic_for!(Int64::ReverseEndianness, ReverseEndianness, W64, true, 1),
    intrinsic_for!(Int64::IsPowerOfTwo, IsPowerOfTwo, W64, true, 1),
    intrinsic_for!(UInt64::TryAdd, TryAdd, W64, false, 2),
    intrinsic_for!(UInt64::TrySubtract, TrySub, W64, false, 2),
    intrinsic_for!(UInt64::TryMultiply, TryMul, W64, false, 2),
    intrinsic_for!(UInt64::LeadingZeroCount, LeadingZeroCount, W64, false, 1),
    intrinsic_for!(UInt64::TrailingZeroCount, TrailingZeroCount, W64, false, 1),
    intrinsic_for!(UInt64::PopCount, PopCount, W64, false, 1),
    intrinsic_for!(UInt64::RotateLeft, RotateLeft, W64, false, 2),
    intrinsic_for!(UInt64::RotateRight, RotateRight, W64, false, 2),
    intrinsic_for!(UInt64::ReverseEndianness, ReverseEndianness, W64, false, 1),
    intrinsic_for!(UInt64::IsPowerOfTwo, IsPowerOfTwo, W64, false, 1),
    intrinsic_for!(Int128::TryAdd, TryAdd, W128, true, 2),
    intrinsic_for!(Int128::TrySubtract, TrySub, W128, true, 2),
    intrinsic_for!(Int128::TryMultiply, TryMul, W128, true, 2),
    intrinsic_for!(Int128::TryNegate, TryNeg, W128, true, 1),
    intrinsic_for!(Int128::LeadingZeroCount, LeadingZeroCount, W128, true, 1),
    intrinsic_for!(Int128::TrailingZeroCount, TrailingZeroCount, W128, true, 1),
    intrinsic_for!(Int128::PopCount, PopCount, W128, true, 1),
    intrinsic_for!(Int128::RotateLeft, RotateLeft, W128, true, 2),
    intrinsic_for!(Int128::RotateRight, RotateRight, W128, true, 2),
    intrinsic_for!(Int128::ReverseEndianness, ReverseEndianness, W128, true, 1),
    intrinsic_for!(Int128::IsPowerOfTwo, IsPowerOfTwo, W128, true, 1),
    intrinsic_for!(UInt128::TryAdd, TryAdd, W128, false, 2),
    intrinsic_for!(UInt128::TrySubtract, TrySub, W128, false, 2),
    intrinsic_for!(UInt128::TryMultiply, TryMul, W128, false, 2),
    intrinsic_for!(UInt128::LeadingZeroCount, LeadingZeroCount, W128, false, 1),
    intrinsic_for!(
        UInt128::TrailingZeroCount,
        TrailingZeroCount,
        W128,
        false,
        1
    ),
    intrinsic_for!(UInt128::PopCount, PopCount, W128, false, 1),
    intrinsic_for!(UInt128::RotateLeft, RotateLeft, W128, false, 2),
    intrinsic_for!(UInt128::RotateRight, RotateRight, W128, false, 2),
    intrinsic_for!(
        UInt128::ReverseEndianness,
        ReverseEndianness,
        W128,
        false,
        1
    ),
    intrinsic_for!(UInt128::IsPowerOfTwo, IsPowerOfTwo, W128, false, 1),
];

/// Pointer-width aware view of the intrinsic registry (adds `nint`/`nuint` aliases).
pub fn numeric_intrinsics_with_pointer() -> Vec<NumericIntrinsicEntry> {
    let mut entries = NUMERIC_INTRINSICS.to_vec();
    entries.extend([
        intrinsic_for!(IntPtr::TryAdd, TryAdd, Pointer, true, 2),
        intrinsic_for!(UIntPtr::TryAdd, TryAdd, Pointer, false, 2),
        intrinsic_for!(IntPtr::TrySubtract, TrySub, Pointer, true, 2),
        intrinsic_for!(UIntPtr::TrySubtract, TrySub, Pointer, false, 2),
        intrinsic_for!(IntPtr::TryMultiply, TryMul, Pointer, true, 2),
        intrinsic_for!(UIntPtr::TryMultiply, TryMul, Pointer, false, 2),
        intrinsic_for!(IntPtr::TryNegate, TryNeg, Pointer, true, 1),
    ]);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_includes_pointer_variants() {
        let entries = numeric_intrinsics_with_pointer();
        let ptr_add = entries
            .iter()
            .find(|entry| entry.symbol.ends_with("IntPtr::TryAdd"))
            .expect("pointer add intrinsic present");
        assert_eq!(ptr_add.width, NumericWidth::Pointer);
        assert!(ptr_add.signed);

        let unsigned = entries
            .iter()
            .find(|entry| entry.symbol.ends_with("UIntPtr::TryAdd"))
            .expect("pointer unsigned add intrinsic present");
        assert_eq!(unsigned.width, NumericWidth::Pointer);
        assert!(!unsigned.signed);
    }
}
