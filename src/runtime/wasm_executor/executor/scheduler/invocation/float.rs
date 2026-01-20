use crate::mir::{FloatStatusFlags, RoundingMode};
use crate::runtime::float_env::record_flags;

fn record_demote_flags(src: f64, dst: f32) {
    let mut flags = FloatStatusFlags::default();
    if src.is_nan() {
        flags.invalid = true;
    }
    if src.is_infinite() {
        flags.overflow = true;
    }
    if (dst as f64) != src {
        flags.inexact = true;
    }
    if dst == 0.0 && src != 0.0 {
        flags.underflow = true;
    }
    record_flags(flags);
}

fn next_up_f32(value: f32) -> f32 {
    if value.is_nan() || value == f32::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f32::from_bits(1);
    }
    let bits = value.to_bits();
    let next = if value.is_sign_negative() {
        bits.wrapping_sub(1)
    } else {
        bits.wrapping_add(1)
    };
    f32::from_bits(next)
}

fn next_down_f32(value: f32) -> f32 {
    if value.is_nan() || value == f32::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f32::from_bits(1);
    }
    let bits = value.to_bits();
    let next = if value.is_sign_negative() {
        bits.wrapping_add(1)
    } else {
        bits.wrapping_sub(1)
    };
    f32::from_bits(next)
}

fn next_up_f64(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    let next = if value.is_sign_negative() {
        bits.wrapping_sub(1)
    } else {
        bits.wrapping_add(1)
    };
    f64::from_bits(next)
}

fn next_down_f64(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    let next = if value.is_sign_negative() {
        bits.wrapping_add(1)
    } else {
        bits.wrapping_sub(1)
    };
    f64::from_bits(next)
}

pub(super) fn adjust_rounding_f32(original: f64, nearest: f32, mode: RoundingMode) -> f32 {
    if (nearest as f64) == original || nearest.is_infinite() || nearest.is_nan() {
        return nearest;
    }
    match mode {
        RoundingMode::NearestTiesToEven => nearest,
        RoundingMode::NearestTiesToAway => {
            if original > 0.0 {
                next_up_f32(nearest)
            } else {
                next_down_f32(nearest)
            }
        }
        RoundingMode::TowardZero => {
            if original > 0.0 {
                next_down_f32(nearest)
            } else {
                next_up_f32(nearest)
            }
        }
        RoundingMode::TowardPositive => next_up_f32(nearest),
        RoundingMode::TowardNegative => next_down_f32(nearest),
    }
}

pub(super) fn adjust_rounding_f64(original: f64, nearest: f64, mode: RoundingMode) -> f64 {
    if nearest == original || nearest.is_infinite() || nearest.is_nan() {
        return nearest;
    }
    match mode {
        RoundingMode::NearestTiesToEven => nearest,
        RoundingMode::NearestTiesToAway => {
            if original > 0.0 {
                next_up_f64(nearest)
            } else {
                next_down_f64(nearest)
            }
        }
        RoundingMode::TowardZero => {
            if original > 0.0 {
                next_down_f64(nearest)
            } else {
                next_up_f64(nearest)
            }
        }
        RoundingMode::TowardPositive => next_up_f64(nearest),
        RoundingMode::TowardNegative => next_down_f64(nearest),
    }
}

pub(super) fn convert_int_to_f32(value: i128, mode: RoundingMode) -> f32 {
    let nearest = (value as f32).to_bits();
    let nearest = f32::from_bits(nearest);
    let adjusted = adjust_rounding_f32(value as f64, nearest, mode);
    let flags = FloatStatusFlags {
        overflow: adjusted.is_infinite(),
        inexact: (adjusted as f64) != value as f64,
        ..FloatStatusFlags::default()
    };
    record_flags(flags);
    adjusted
}

pub(super) fn convert_int_to_f64(value: i128, mode: RoundingMode) -> f64 {
    let nearest = value as f64;
    let adjusted = adjust_rounding_f64(value as f64, nearest, mode);
    let flags = FloatStatusFlags {
        overflow: adjusted.is_infinite(),
        inexact: adjusted != value as f64,
        ..FloatStatusFlags::default()
    };
    record_flags(flags);
    adjusted
}

pub(super) fn round_f64_to_f32(value: f64, mode: RoundingMode) -> f32 {
    if value.is_nan() {
        let demoted = demote_nan_to_f32(value);
        record_demote_flags(value, demoted);
        return demoted;
    }
    let nearest = value as f32;
    let adjusted = adjust_rounding_f32(value, nearest, mode);
    record_demote_flags(value, adjusted);
    adjusted
}

pub(super) fn round_value(value: f64, mode: RoundingMode) -> f64 {
    match mode {
        RoundingMode::NearestTiesToEven => round_ties_to_even(value),
        RoundingMode::NearestTiesToAway => value.round(),
        RoundingMode::TowardZero => value.trunc(),
        RoundingMode::TowardPositive => value.ceil(),
        RoundingMode::TowardNegative => value.floor(),
    }
}

fn round_ties_to_even(value: f64) -> f64 {
    // Fast path: non-ties follow the standard round-to-nearest.
    let rounded = value.round();
    let diff = (value - rounded).abs();
    if diff < 0.5 || diff > 0.5 {
        return rounded;
    }
    // Ties: pick the even integer.
    let floor = value.floor();
    let ceil = value.ceil();
    if floor % 2.0 == 0.0 { floor } else { ceil }
}

fn demote_nan_to_f32(value: f64) -> f32 {
    let bits = value.to_bits();
    let sign = ((bits >> 63) as u32) << 31;
    let fraction = bits & 0x000F_FFFF_FFFF_FFFF;
    let payload = ((fraction >> (52 - 22)) as u32) & 0x003F_FFFF;
    let quiet_payload = payload | (1 << 22);
    let bits32 = sign | (0xFF << 23) | quiet_payload;
    f32::from_bits(bits32)
}

pub(super) fn record_conversion_flags(value: f64, rounded: f64, min: f64, max: f64) {
    let mut flags = FloatStatusFlags::default();
    if value.is_nan() {
        flags.invalid = true;
        record_flags(flags);
        return;
    }
    if rounded.is_infinite() || rounded < min || rounded > max {
        flags.invalid = true;
        flags.overflow = true;
    }
    if rounded != value {
        flags.inexact = true;
    }
    record_flags(flags);
}

pub(super) fn record_arithmetic_flags(lhs: f64, rhs: f64, exact: f64, rounded: f64, is_div: bool) {
    let mut flags = FloatStatusFlags::default();
    if exact.is_nan() {
        flags.invalid = true;
    }
    if is_div && rhs == 0.0 && lhs.is_finite() {
        flags.div_by_zero = true;
    }
    if rounded.is_infinite() && exact.is_finite() {
        flags.overflow = true;
    }
    if rounded == 0.0 && exact != 0.0 {
        flags.underflow = true;
        flags.inexact = true;
    } else if rounded != exact {
        flags.inexact = true;
    }
    if flags.any() {
        if std::env::var_os("CHIC_DEBUG_WASM_ARITH").is_some() {
            eprintln!(
                "[wasm-arith] lhs={lhs:?} rhs={rhs:?} exact={exact:?} rounded={rounded:?} flags={flags:?} div={is_div}"
            );
        }
        record_flags(flags);
    }
}
