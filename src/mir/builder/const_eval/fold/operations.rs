use crate::decimal::{Decimal128, DecimalRoundingMode};
use crate::frontend::diagnostics::Span;
use crate::mir::{BinOp, ConstValue, FloatValue, FloatWidth, Ty};
use crate::syntax::numeric::{NumericLiteralMetadata, NumericLiteralType};

use super::super::ConstEvalResult;
use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};

pub(crate) enum DecimalBinaryKind {
    Add,
    Sub,
    Mul,
}

fn signed_value(value: &ConstValue) -> Option<i128> {
    match value {
        ConstValue::Int(v) | ConstValue::Int32(v) => Some(*v),
        _ => None,
    }
}

fn float_result_width(left: &FloatValue, right: &FloatValue) -> FloatWidth {
    use FloatWidth::*;
    match (left.width, right.width) {
        (F128, _) | (_, F128) => F128,
        (F64, _) | (_, F64) => F64,
        (F32, _) | (_, F32) => F32,
        _ => F16,
    }
}

fn compose_float(value: f64, width: FloatWidth) -> FloatValue {
    FloatValue::from_f64_as(value, width)
}

impl<'a> ConstEvalContext<'a> {
    fn eval_float_op<F>(&self, left: FloatValue, right: FloatValue, op: F) -> ConstEvalResult
    where
        F: Fn(f64, f64) -> f64,
    {
        let width = float_result_width(&left, &right);
        let lhs = left.to_f64();
        let rhs = right.to_f64();
        let result = compose_float(op(lhs, rhs), width);
        ConstEvalResult::new(ConstValue::Float(result))
    }

    pub(crate) fn binary_numeric_op<F>(
        &self,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
        decimal_kind: DecimalBinaryKind,
        op: F,
    ) -> Result<ConstEvalResult, ConstEvalError>
    where
        F: Fn(i128, i128) -> Option<i128>,
    {
        let left_value = left.value;
        let right_value = right.value;
        if let (Some(a), Some(b)) = (signed_value(&left_value), signed_value(&right_value)) {
            return op(a, b)
                .map(ConstValue::Int)
                .map(ConstEvalResult::new)
                .ok_or_else(|| ConstEvalError {
                    message: "arithmetic operation overflowed the supported range".into(),
                    span,
                });
        }
        match (left_value, right_value) {
            (ConstValue::UInt(a), ConstValue::UInt(b)) => op(
                i128::try_from(a).unwrap_or(i128::MAX),
                i128::try_from(b).unwrap_or(i128::MAX),
            )
            .and_then(|value| u128::try_from(value).ok())
            .map(ConstValue::UInt)
            .map(ConstEvalResult::new)
            .ok_or_else(|| ConstEvalError {
                message: "arithmetic operation overflowed the supported range".into(),
                span,
            }),
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(self.eval_float_op(a, b, |lhs, rhs| lhs + rhs))
            }
            (ConstValue::Decimal(a), ConstValue::Decimal(b)) => {
                return self.decimal_binary_apply(decimal_kind, a, b, span);
            }
            (lhs, rhs) => Err(ConstEvalError {
                message: format!(
                    "arithmetic operation requires matching numeric operands, got {lhs:?} and {rhs:?}"
                ),
                span,
            }),
        }
    }

    fn decimal_binary_apply(
        &self,
        kind: DecimalBinaryKind,
        left: Decimal128,
        right: Decimal128,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let rounding = DecimalRoundingMode::TiesToEven;
        let result = match kind {
            DecimalBinaryKind::Add => left.add(right, rounding),
            DecimalBinaryKind::Sub => left.sub(right, rounding),
            DecimalBinaryKind::Mul => left.mul(right, rounding),
        }
        .map_err(|err| ConstEvalError {
            message: err.to_string(),
            span,
        })?;
        Ok(ConstEvalResult::new(ConstValue::Decimal(result)))
    }

    pub(crate) fn binary_div(
        &self,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match (left.value, right.value) {
            (_, ConstValue::Int(0) | ConstValue::UInt(0)) => Err(ConstEvalError {
                message: "division by zero in constant expression".into(),
                span,
            }),
            (_, ConstValue::Float(v)) if v.to_f64() == 0.0 => Err(ConstEvalError {
                message: "division by zero in constant expression".into(),
                span,
            }),
            (ConstValue::Int(a), ConstValue::Int(b)) => a
                .checked_div(b)
                .map(ConstValue::Int)
                .map(ConstEvalResult::new)
                .ok_or_else(|| ConstEvalError {
                    message: "integer division overflowed the supported range".into(),
                    span,
                }),
            (ConstValue::UInt(a), ConstValue::UInt(b)) => a
                .checked_div(b)
                .map(ConstValue::UInt)
                .map(ConstEvalResult::new)
                .ok_or_else(|| ConstEvalError {
                    message: "integer division overflowed the supported range".into(),
                    span,
                }),
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                Ok(self.eval_float_op(a, b, |lhs, rhs| lhs / rhs))
            }
            (ConstValue::Decimal(a), ConstValue::Decimal(b)) => {
                if b.as_decimal().is_zero() {
                    return Err(ConstEvalError {
                        message: "division by zero in constant expression".into(),
                        span,
                    });
                }
                let result =
                    a.div(b, DecimalRoundingMode::TiesToEven)
                        .map_err(|err| ConstEvalError {
                            message: err.to_string(),
                            span,
                        })?;
                Ok(ConstEvalResult::new(ConstValue::Decimal(result)))
            }
            (lhs, rhs) => Err(ConstEvalError {
                message: format!(
                    "division requires matching numeric operands, got {lhs:?} and {rhs:?}"
                ),
                span,
            }),
        }
    }

    pub(crate) fn binary_rem(
        &self,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match (left.value, right.value) {
            (_, ConstValue::Int(0) | ConstValue::UInt(0)) => Err(ConstEvalError {
                message: "remainder by zero in constant expression".into(),
                span,
            }),
            (ConstValue::Int(a), ConstValue::Int(b)) => a
                .checked_rem(b)
                .map(ConstValue::Int)
                .map(ConstEvalResult::new)
                .ok_or_else(|| ConstEvalError {
                    message: "integer remainder overflowed the supported range".into(),
                    span,
                }),
            (ConstValue::UInt(a), ConstValue::UInt(b)) => a
                .checked_rem(b)
                .map(ConstValue::UInt)
                .map(ConstEvalResult::new)
                .ok_or_else(|| ConstEvalError {
                    message: "integer remainder overflowed the supported range".into(),
                    span,
                }),
            (ConstValue::Decimal(a), ConstValue::Decimal(b)) => {
                if b.as_decimal().is_zero() {
                    return Err(ConstEvalError {
                        message: "remainder by zero in constant expression".into(),
                        span,
                    });
                }
                let result = a.rem(b).map_err(|err| ConstEvalError {
                    message: err.to_string(),
                    span,
                })?;
                Ok(ConstEvalResult::new(ConstValue::Decimal(result)))
            }
            (lhs, rhs) => Err(ConstEvalError {
                message: format!(
                    "remainder requires matching integer operands, got {lhs:?} and {rhs:?}"
                ),
                span,
            }),
        }
    }

    pub(crate) fn binary_integer<F>(
        &self,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
        op: F,
    ) -> Result<ConstEvalResult, ConstEvalError>
    where
        F: Fn(u128, u128) -> (u128, u128),
    {
        match (left.value, right.value) {
            (ConstValue::Int(a), ConstValue::Int(b)) => {
                let (lhs, rhs) = op(a as u128, b as u128);
                Ok(ConstEvalResult::new(ConstValue::Int(
                    (lhs as i128) & (rhs as i128),
                )))
            }
            (ConstValue::UInt(a), ConstValue::UInt(b)) => {
                let (lhs, rhs) = op(a, b);
                Ok(ConstEvalResult::new(ConstValue::UInt(lhs & rhs)))
            }
            (lhs, rhs) => Err(ConstEvalError {
                message: format!(
                    "bitwise operation requires matching integer operands, got {lhs:?} and {rhs:?}"
                ),
                span,
            }),
        }
    }

    pub(crate) fn binary_shift(
        &self,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
        is_left: bool,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let shift = match right.value {
            ConstValue::Int(value) if value >= 0 => value as u32,
            ConstValue::UInt(value) => u32::try_from(value).map_err(|_| ConstEvalError {
                message: "shift amount exceeds supported range".into(),
                span,
            })?,
            other => {
                return Err(ConstEvalError {
                    message: format!("shift amount must be an integer, got {other:?}"),
                    span,
                });
            }
        };

        match left.value {
            ConstValue::Int(value) => {
                let result = if is_left {
                    value.checked_shl(shift)
                } else {
                    value.checked_shr(shift)
                };
                result
                    .map(ConstValue::Int)
                    .map(ConstEvalResult::new)
                    .ok_or_else(|| ConstEvalError {
                        message: "shift operation overflowed the supported range".into(),
                        span,
                    })
            }
            ConstValue::UInt(value) => {
                let result = if is_left {
                    value.checked_shl(shift)
                } else {
                    value.checked_shr(shift)
                };
                result
                    .map(ConstValue::UInt)
                    .map(ConstEvalResult::new)
                    .ok_or_else(|| ConstEvalError {
                        message: "shift operation overflowed the supported range".into(),
                        span,
                    })
            }
            other => Err(ConstEvalError {
                message: format!("shift requires integer operand, got {other:?}"),
                span,
            }),
        }
    }

    pub(crate) fn binary_bool(
        &self,
        op: BinOp,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match (left.value, right.value) {
            (ConstValue::Bool(lhs), ConstValue::Bool(rhs)) => {
                let value = match op {
                    BinOp::And => lhs && rhs,
                    BinOp::Or => lhs || rhs,
                    _ => unreachable!(),
                };
                Ok(ConstEvalResult::new(ConstValue::Bool(value)))
            }
            (lhs, rhs) => Err(ConstEvalError {
                message: format!(
                    "logical operation requires boolean operands, got {lhs:?} and {rhs:?}"
                ),
                span,
            }),
        }
    }

    pub(crate) fn binary_compare(
        &self,
        op: BinOp,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let bool_value = match (left.value, right.value) {
            (ConstValue::Int(a), ConstValue::Int(b)) => diagnostics::compare_numbers(op, a, b),
            (ConstValue::UInt(a), ConstValue::UInt(b)) => diagnostics::compare_numbers(op, a, b),
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                diagnostics::compare_numbers(op, a.to_f64(), b.to_f64())
            }
            (ConstValue::Decimal(a), ConstValue::Decimal(b)) => {
                let ordering = a.as_decimal().cmp(b.as_decimal());
                match op {
                    BinOp::Lt => ordering == std::cmp::Ordering::Less,
                    BinOp::Le => ordering != std::cmp::Ordering::Greater,
                    BinOp::Gt => ordering == std::cmp::Ordering::Greater,
                    BinOp::Ge => ordering != std::cmp::Ordering::Less,
                    _ => {
                        return Err(ConstEvalError {
                            message: format!(
                                "comparison operator `{op:?}` is not supported for decimal values"
                            ),
                            span,
                        });
                    }
                }
            }
            (lhs, rhs) => {
                return Err(ConstEvalError {
                    message: format!(
                        "comparison requires matching numeric operands, got {lhs:?} and {rhs:?}"
                    ),
                    span,
                });
            }
        };
        Ok(ConstEvalResult::new(ConstValue::Bool(bool_value)))
    }

    pub(crate) fn const_equal(&self, left: &ConstValue, right: &ConstValue) -> bool {
        match (left, right) {
            (ConstValue::Int(a), ConstValue::Int(b)) => a == b,
            (ConstValue::UInt(a), ConstValue::UInt(b)) => a == b,
            (ConstValue::Float(a), ConstValue::Float(b)) => a == b,
            (ConstValue::Decimal(a), ConstValue::Decimal(b)) => a.as_decimal() == b.as_decimal(),
            (ConstValue::Bool(a), ConstValue::Bool(b)) => a == b,
            (ConstValue::Char(a), ConstValue::Char(b)) => a == b,
            (ConstValue::Str { value: a, .. }, ConstValue::Str { value: b, .. }) => a == b,
            (ConstValue::RawStr(a), ConstValue::RawStr(b)) => a == b,
            (
                ConstValue::Enum {
                    type_name: a_ty,
                    variant: a_variant,
                    discriminant: a_disc,
                },
                ConstValue::Enum {
                    type_name: b_ty,
                    variant: b_variant,
                    discriminant: b_disc,
                },
            ) => a_ty == b_ty && a_variant == b_variant && a_disc == b_disc,
            (
                ConstValue::Struct {
                    type_name: a_ty,
                    fields: a_fields,
                },
                ConstValue::Struct {
                    type_name: b_ty,
                    fields: b_fields,
                },
            ) => {
                if a_ty != b_ty || a_fields.len() != b_fields.len() {
                    return false;
                }
                a_fields
                    .iter()
                    .zip(b_fields)
                    .all(|((a_name, a_value), (b_name, b_value))| {
                        a_name == b_name && self.const_equal(a_value, b_value)
                    })
            }
            (ConstValue::Null, ConstValue::Null) => true,
            (ConstValue::Unit, ConstValue::Unit) => true,
            _ => false,
        }
    }

    pub(crate) fn convert_value_to_type(
        &mut self,
        value: ConstValue,
        literal: Option<NumericLiteralMetadata>,
        ty: &Ty,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        match ty {
            Ty::Named(name) => {
                self.convert_named_value(value, name.as_str(), literal.as_ref(), span)
            }
            Ty::String | Ty::Str => self.convert_string(value, span),
            Ty::Nullable(inner) => match value {
                ConstValue::Null => Ok(ConstValue::Null),
                other => {
                    let inner_value = self.convert_value_to_type(other, literal, inner, span)?;
                    Ok(inner_value)
                }
            },
            Ty::Unit => match value {
                ConstValue::Unit => Ok(ConstValue::Unit),
                other => Err(ConstEvalError {
                    message: format!("expected unit value, found {other:?}"),
                    span,
                }),
            },
            _ => Err(ConstEvalError {
                message: format!(
                    "constant declarations of type `{}` are not yet supported",
                    ty.canonical_name()
                ),
                span,
            }),
        }
    }

    pub(crate) fn convert_named_value(
        &mut self,
        value: ConstValue,
        name: &str,
        literal: Option<&NumericLiteralMetadata>,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        let base = diagnostics::simple_name(name);
        let base_lower = base.to_ascii_lowercase();
        let base_contains_int128 = base_lower.contains("int128") || base_lower.contains("i128");
        let base_contains_uint128 = base_lower.contains("uint128") || base_lower.contains("u128");
        if let ConstValue::Enum { type_name, .. } = &value {
            if type_name == name || diagnostics::simple_name(type_name) == base {
                return Ok(value);
            }
        }
        if let ConstValue::Struct { type_name, .. } = &value {
            if type_name == name || diagnostics::simple_name(type_name) == base {
                return Ok(value);
            }
        }
        let pointer_bits = (super::super::super::pointer_size() * 8) as u32;
        match base_lower.as_str() {
            "bool" => match value {
                ConstValue::Bool(_) => Ok(value),
                ConstValue::Int(v) | ConstValue::Int32(v) => Ok(ConstValue::Bool(v != 0)),
                ConstValue::UInt(v) => Ok(ConstValue::Bool(v != 0)),
                other => Err(ConstEvalError {
                    message: format!("cannot convert {other:?} to `bool`"),
                    span,
                }),
            },
            "char" => match value {
                ConstValue::Char(_) => Ok(value),
                ConstValue::Int(v) | ConstValue::Int32(v) => {
                    let scalar = u16::try_from(v).map_err(|_| ConstEvalError {
                        message: format!("value `{v}` does not fit in char"),
                        span,
                    })?;
                    Ok(ConstValue::Char(scalar))
                }
                ConstValue::UInt(v) => {
                    let scalar = u16::try_from(v).map_err(|_| ConstEvalError {
                        message: format!("value `{v}` does not fit in char"),
                        span,
                    })?;
                    Ok(ConstValue::Char(scalar))
                }
                other => Err(ConstEvalError {
                    message: format!("cannot convert {other:?} to `char`"),
                    span,
                }),
            },
            "string" | "System::String" | "Std::String" => self.convert_string(value, span),
            "str" | "System::Str" | "Std::Str" => self.convert_string(value, span),
            "sbyte" => self.convert_signed(value, 8, literal, "sbyte", span),
            "byte" => self.convert_unsigned(value, 8, literal, "byte", span),
            "short" => self.convert_signed(value, 16, literal, "short", span),
            "ushort" => self.convert_unsigned(value, 16, literal, "ushort", span),
            "int" => self.convert_signed(value, 32, literal, &base_lower, span),
            "uint" => self.convert_unsigned(value, 32, literal, &base_lower, span),
            "nint" | "isize" => {
                self.convert_signed(value, pointer_bits, literal, &base_lower, span)
            }
            "nuint" | "usize" => {
                self.convert_unsigned(value, pointer_bits, literal, &base_lower, span)
            }
            "long" => self.convert_signed(value, 64, literal, "long", span),
            "ulong" => self.convert_unsigned(value, 64, literal, "ulong", span),
            "int128" | "i128" => self.convert_signed(value, 128, literal, "int128", span),
            "uint128" | "u128" => self.convert_unsigned(value, 128, literal, "uint128", span),
            "float" => self.convert_float(value, literal, "float", span),
            "double" => self.convert_float(value, literal, "double", span),
            "decimal" => self.convert_decimal(value, span),
            "decimalintrinsicresult" => self.convert_decimal_intrinsic_result(value, name, span),
            other => {
                if base_contains_int128 {
                    return self.convert_signed(value, 128, literal, "int128", span);
                }
                if base_contains_uint128 {
                    return self.convert_unsigned(value, 128, literal, "uint128", span);
                }
                Err(ConstEvalError {
                    message: format!("constants of type `{other}` are not supported"),
                    span,
                })
            }
        }
    }

    pub(crate) fn convert_signed(
        &self,
        value: ConstValue,
        bits: u32,
        literal: Option<&NumericLiteralMetadata>,
        target_name: &str,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        let value = match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => v,
            ConstValue::UInt(v) => i128::try_from(v).map_err(|_| ConstEvalError {
                message: self.literal_overflow_message(
                    literal,
                    v.to_string(),
                    target_name,
                    bits,
                    true,
                ),
                span,
            })?,
            ConstValue::Char(ch) => ch as i128,
            other => {
                return Err(ConstEvalError {
                    message: format!("cannot convert {other:?} to signed integer"),
                    span,
                });
            }
        };
        let (min, max) = if bits == 128 {
            (i128::MIN, i128::MAX)
        } else {
            let max = (1i128 << (bits - 1)) - 1;
            let min = -max - 1;
            (min, max)
        };
        if value < min || value > max {
            return Err(ConstEvalError {
                message: self.literal_overflow_message(
                    literal,
                    value.to_string(),
                    target_name,
                    bits,
                    true,
                ),
                span,
            });
        }
        Ok(ConstValue::Int(value))
    }

    pub(crate) fn convert_unsigned(
        &self,
        value: ConstValue,
        bits: u32,
        literal: Option<&NumericLiteralMetadata>,
        target_name: &str,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        let value = match value {
            ConstValue::UInt(v) => v,
            ConstValue::Int(v) | ConstValue::Int32(v) if v >= 0 => v as u128,
            ConstValue::Char(ch) => ch as u128,
            other => {
                return Err(ConstEvalError {
                    message: format!("cannot convert {other:?} to unsigned integer"),
                    span,
                });
            }
        };
        let max = if bits >= 128 {
            u128::MAX
        } else {
            (1u128 << bits) - 1
        };
        if value > max {
            return Err(ConstEvalError {
                message: self.literal_overflow_message(
                    literal,
                    value.to_string(),
                    target_name,
                    bits,
                    false,
                ),
                span,
            });
        }
        Ok(ConstValue::UInt(value))
    }

    pub(crate) fn convert_float(
        &self,
        value: ConstValue,
        literal: Option<&NumericLiteralMetadata>,
        target_name: &str,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        let target_width = match target_name.to_ascii_lowercase().as_str() {
            "float16" | "half" | "f16" => FloatWidth::F16,
            "float" | "single" | "f32" => FloatWidth::F32,
            "double" | "float64" | "f64" => FloatWidth::F64,
            "float128" | "quad" | "f128" => FloatWidth::F128,
            _ => FloatWidth::F64,
        };
        match value {
            ConstValue::Float(existing) => {
                let coerced = compose_float(existing.to_f64(), target_width);
                Ok(ConstValue::Float(coerced))
            }
            ConstValue::Int(v) | ConstValue::Int32(v) => {
                Ok(ConstValue::Float(compose_float(v as f64, target_width)))
            }
            ConstValue::UInt(v) => Ok(ConstValue::Float(compose_float(v as f64, target_width))),
            ConstValue::Decimal(v) => v
                .to_f64()
                .map(|float| ConstValue::Float(compose_float(float, target_width)))
                .ok_or_else(|| ConstEvalError {
                    message: self.float_overflow_message(literal, target_name),
                    span,
                }),
            other => Err(ConstEvalError {
                message: format!("cannot convert {other:?} to floating-point value"),
                span,
            }),
        }
    }

    pub(crate) fn convert_decimal(
        &self,
        value: ConstValue,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        let decimal = match value {
            ConstValue::Decimal(existing) => return Ok(ConstValue::Decimal(existing)),
            ConstValue::Int(v) | ConstValue::Int32(v) => Decimal128::from_i128(v),
            ConstValue::UInt(v) => Decimal128::from_u128(v),
            ConstValue::Float(v) => Decimal128::from_f64(v.to_f64()),
            other => {
                return Err(ConstEvalError {
                    message: format!("cannot convert {other:?} to `decimal`"),
                    span,
                });
            }
        }
        .map_err(|err| ConstEvalError {
            message: err.to_string(),
            span,
        })?;
        Ok(ConstValue::Decimal(decimal))
    }

    fn literal_overflow_message(
        &self,
        literal: Option<&NumericLiteralMetadata>,
        value: String,
        target: &str,
        bits: u32,
        signed: bool,
    ) -> String {
        let literal_desc = literal
            .and_then(|meta| {
                if meta.explicit_suffix {
                    meta.suffix_str()
                        .map(|suffix| format!("literal with suffix `{suffix}`"))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "literal".to_string());
        let width_desc = if bits >= 128 {
            "128-bit".to_string()
        } else {
            format!("{bits}-bit")
        };
        let signed_desc = if signed { "signed" } else { "unsigned" };
        format!(
            "{literal_desc} value `{value}` does not fit in {width_desc} {signed_desc} integer `{target}`"
        )
    }

    fn float_overflow_message(
        &self,
        literal: Option<&NumericLiteralMetadata>,
        target: &str,
    ) -> String {
        if let Some(meta) = literal {
            if meta.explicit_suffix {
                if let Some(suffix) = meta.suffix_str() {
                    return format!(
                        "literal with suffix `{suffix}` cannot be represented as `{target}`"
                    );
                }
            }
            match meta.literal_type {
                NumericLiteralType::Float16 => {
                    return format!(
                        "literal of type `float16` cannot be represented as `{target}`"
                    );
                }
                NumericLiteralType::Float32 => {
                    return format!("literal of type `float` cannot be represented as `{target}`");
                }
                NumericLiteralType::Float64 => {
                    return format!("literal of type `double` cannot be represented as `{target}`");
                }
                NumericLiteralType::Float128 => {
                    return format!(
                        "literal of type `float128` cannot be represented as `{target}`"
                    );
                }
                NumericLiteralType::Decimal => {
                    return format!("decimal literal cannot be represented as `{target}`");
                }
                _ => {}
            }
        }
        format!("value cannot be represented as `{target}`")
    }

    pub(crate) fn convert_decimal_intrinsic_result(
        &self,
        value: ConstValue,
        ty_name: &str,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        match value {
            ConstValue::Struct { type_name, fields } => {
                if type_name != ty_name
                    && diagnostics::simple_name(&type_name) != diagnostics::simple_name(ty_name)
                {
                    return Err(ConstEvalError {
                        message: format!("cannot convert struct `{type_name}` into `{ty_name}`"),
                        span,
                    });
                }
                let mut status: Option<ConstValue> = None;
                let mut decimal_value: Option<ConstValue> = None;
                let mut variant: Option<ConstValue> = None;
                for (name, field) in fields {
                    match name.as_str() {
                        "Status" => status = Some(field),
                        "Value" => decimal_value = Some(field),
                        "Variant" => variant = Some(field),
                        _ => {}
                    }
                }
                let Some(status) = status else {
                    return Err(ConstEvalError {
                        message: format!(
                            "`{ty_name}` constant missing `Status` field in struct literal"
                        ),
                        span,
                    });
                };
                let Some(decimal_value) = decimal_value else {
                    return Err(ConstEvalError {
                        message: format!(
                            "`{ty_name}` constant missing `Value` field in struct literal"
                        ),
                        span,
                    });
                };
                let Some(variant) = variant else {
                    return Err(ConstEvalError {
                        message: format!(
                            "`{ty_name}` constant missing `Variant` field in struct literal"
                        ),
                        span,
                    });
                };
                Ok(ConstValue::Struct {
                    type_name: ty_name.to_string(),
                    fields: vec![
                        ("Status".to_string(), status),
                        ("Value".to_string(), decimal_value),
                        ("Variant".to_string(), variant),
                    ],
                })
            }
            other => Err(ConstEvalError {
                message: format!("cannot convert {other:?} to `{ty_name}`"),
                span,
            }),
        }
    }

    pub(crate) fn convert_string(
        &self,
        value: ConstValue,
        span: Option<Span>,
    ) -> Result<ConstValue, ConstEvalError> {
        match value {
            ConstValue::RawStr(text) => Ok(ConstValue::RawStr(text)),
            ConstValue::Str { id, value } => Ok(ConstValue::Str { id, value }),
            ConstValue::Null => Ok(ConstValue::Null),
            other => Err(ConstEvalError {
                message: format!("cannot convert {other:?} to string constant"),
                span,
            }),
        }
    }
}
