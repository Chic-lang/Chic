#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

use crate::codegen::llvm::intrinsics::{
    QuantRoundingMode, QuantizedKernel, select_quantized_kernel,
};

/// Quantization policy metadata threaded through lowering.
#[derive(Debug, Clone)]
pub struct QuantPolicy {
    pub scales: Vec<f32>,
    pub zero_points: Vec<i32>,
    pub per_channel_axis: Option<usize>,
    pub rounding: QuantRoundingMode,
    pub saturate: bool,
}

impl QuantPolicy {
    #[must_use]
    pub fn per_channel(&self) -> bool {
        self.per_channel_axis.is_some()
    }
}

fn round_value(value: f64, mode: QuantRoundingMode) -> f64 {
    match mode {
        QuantRoundingMode::NearestEven => {
            let floor = value.floor();
            let frac = value - floor;
            if (frac.abs() - 0.5).abs() > f64::EPSILON {
                value.round()
            } else if (floor as i64) % 2 == 0 {
                floor
            } else {
                floor + value.signum()
            }
        }
        QuantRoundingMode::TowardZero => value.trunc(),
    }
}

fn clamp_value(value: i64, bits: u32, signed: bool) -> i64 {
    if signed {
        let max = (1i64 << (bits - 1)) - 1;
        let min = -(1i64 << (bits - 1));
        value.clamp(min, max)
    } else {
        let max = (1i64 << bits) - 1;
        value.clamp(0, max)
    }
}

fn channel_index<T: Copy>(values: &[T], channel: usize) -> T
where
    T: Default,
{
    if values.is_empty() {
        T::default()
    } else {
        values[channel % values.len()]
    }
}

/// Quantize a scalar using the provided policy.
#[must_use]
pub fn quantize_scalar(
    value: f32,
    policy: &QuantPolicy,
    channel: usize,
    bits: u32,
    signed: bool,
) -> i32 {
    let scale = channel_index(&policy.scales, channel);
    let zero = channel_index(&policy.zero_points, channel) as f64;
    let scaled = (value as f64) / f64::from(scale) + zero;
    let rounded = round_value(scaled, policy.rounding);
    let mut quant = rounded as i64;
    if policy.saturate {
        quant = clamp_value(quant, bits, signed);
    }
    quant as i32
}

/// Dequantize a previously quantized scalar.
#[must_use]
pub fn dequantize_scalar(value: i32, policy: &QuantPolicy, channel: usize) -> f32 {
    let scale = channel_index(&policy.scales, channel);
    let zero = channel_index(&policy.zero_points, channel);
    (value - zero) as f32 * scale
}

/// Choose a vendor intrinsic when available; otherwise signal that a loop nest should be emitted.
#[must_use]
pub fn choose_kernel(op: &str, policy: &QuantPolicy) -> Option<QuantizedKernel> {
    select_quantized_kernel(op, policy.per_channel(), policy.rounding, policy.saturate)
}
