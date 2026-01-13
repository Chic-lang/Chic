#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Rounding policy supported by the WASM backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmRoundingMode {
    NearestEven,
    TowardZero,
}

/// Quantization policy metadata carried into lowering.
#[derive(Debug, Clone)]
pub struct WasmQuantPolicy {
    pub scales: Vec<f32>,
    pub zero_points: Vec<i32>,
    pub per_channel_axis: Option<usize>,
    pub rounding: WasmRoundingMode,
    pub saturate: bool,
}

fn round_value(value: f64, mode: WasmRoundingMode) -> f64 {
    match mode {
        WasmRoundingMode::NearestEven => {
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
        WasmRoundingMode::TowardZero => value.trunc(),
    }
}

fn clamp(value: i64, bits: u32, signed: bool) -> i32 {
    if signed {
        let max = (1i64 << (bits - 1)) - 1;
        let min = -(1i64 << (bits - 1));
        value.clamp(min, max) as i32
    } else {
        let max = (1i64 << bits) - 1;
        value.clamp(0, max) as i32
    }
}

fn channel_index<T: Copy + Default>(values: &[T], channel: usize) -> T {
    if values.is_empty() {
        T::default()
    } else {
        values[channel % values.len()]
    }
}

/// Quantize a scalar with deterministic rounding/saturation.
#[must_use]
pub fn quantize_scalar(
    value: f32,
    policy: &WasmQuantPolicy,
    channel: usize,
    bits: u32,
    signed: bool,
) -> i32 {
    let scale = channel_index(&policy.scales, channel);
    let zero = channel_index(&policy.zero_points, channel) as f64;
    let scaled = (value as f64) / f64::from(scale) + zero;
    let rounded = round_value(scaled, policy.rounding);
    let quant = rounded as i64;
    if policy.saturate {
        clamp(quant, bits, signed)
    } else {
        quant as i32
    }
}

/// Emit a pseudo-WAT sequence showing the lowering steps.
#[must_use]
pub fn emit_quant_sequence(
    name: &str,
    value: f32,
    policy: &WasmQuantPolicy,
    channel: usize,
    bits: u32,
    signed: bool,
) -> String {
    let quantized = quantize_scalar(value, policy, channel, bits, signed);
    let mut wat = String::new();
    wat.push_str(&format!(
        ";; quantize {name} channel={channel} bits={bits} signed={signed}\n"
    ));
    wat.push_str(&format!(
        "  ;; scale={} zero={} rounding={:?} saturate={}\n",
        channel_index(&policy.scales, channel),
        channel_index(&policy.zero_points, channel),
        policy.rounding,
        policy.saturate
    ));
    wat.push_str(&format!("  (local.set ${name} (i32.const {quantized}))\n"));
    wat
}
