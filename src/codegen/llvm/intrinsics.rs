#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Description of a tensor copy intrinsic or kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorCopyIntrinsic {
    pub symbol: &'static str,
    pub requires_alignment: usize,
    pub contiguous_only: bool,
}

/// Pick an intrinsic for a tensor copy when the layouts allow it.
///
/// Contiguous pairs prefer an aligned memcpy variant; everything else falls back
/// to an explicit loop nest.
pub fn select_copy_intrinsic(
    src_contiguous: bool,
    dst_contiguous: bool,
    alignment: usize,
) -> Option<TensorCopyIntrinsic> {
    if src_contiguous && dst_contiguous {
        if alignment >= 32 {
            Some(TensorCopyIntrinsic {
                symbol: "llvm.memcpy.p0.p0.i64",
                requires_alignment: 32,
                contiguous_only: true,
            })
        } else if alignment >= 16 {
            Some(TensorCopyIntrinsic {
                symbol: "llvm.memcpy.p0.p0.i64",
                requires_alignment: 16,
                contiguous_only: true,
            })
        } else if alignment >= 8 {
            Some(TensorCopyIntrinsic {
                symbol: "llvm.memcpy.p0.p0.i64",
                requires_alignment: 8,
                contiguous_only: true,
            })
        } else {
            Some(TensorCopyIntrinsic {
                symbol: "llvm.memcpy.p0.p0.i64",
                requires_alignment: 1,
                contiguous_only: true,
            })
        }
    } else {
        None
    }
}

/// Rounding behaviour for quantized arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantRoundingMode {
    NearestEven,
    TowardZero,
}

/// Vendor kernel choice for quantized ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantizedKernel {
    pub symbol: &'static str,
    pub per_channel: bool,
    pub saturating: bool,
}

/// Select a quantized kernel when the policy is supported by a vendor intrinsic.
pub fn select_quantized_kernel(
    op: &str,
    per_channel: bool,
    rounding: QuantRoundingMode,
    saturating: bool,
) -> Option<QuantizedKernel> {
    if rounding != QuantRoundingMode::NearestEven {
        return None;
    }
    match op {
        "qgemm" | "QGemm" => Some(QuantizedKernel {
            symbol: "llvm.chic.quant.qgemm",
            per_channel,
            saturating,
        }),
        "qconv" | "QConv" => Some(QuantizedKernel {
            symbol: "llvm.chic.quant.qconv",
            per_channel,
            saturating,
        }),
        _ => None,
    }
}
