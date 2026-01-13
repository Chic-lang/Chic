use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::error::Error;
use crate::mir::{ConstValue, Operand};
use std::fmt::Write;

use super::{DECIMAL_FLAG_VECTORIZE, DECIMAL_VECTORIZE_CANONICAL, decimal_runtime_symbol};

#[derive(Clone)]
pub(crate) struct TypedValue {
    pub(super) repr: String,
    pub(super) ty: String,
}

impl TypedValue {
    pub(crate) fn new(repr: impl Into<String>, ty: &str) -> Self {
        Self {
            repr: repr.into(),
            ty: ty.to_string(),
        }
    }

    pub(crate) fn ty(&self) -> &str {
        &self.ty
    }

    pub(crate) fn into_value_ref(self) -> ValueRef {
        ValueRef::new(self.repr, &self.ty)
    }
}

/// Collect the encoded decimal parts for each operand so callers can marshal
/// inputs without repeating the `emit_operand` → `decimal_value_to_parts` dance.
pub(super) fn collect_decimal_parts(
    emitter: &mut FunctionEmitter<'_>,
    operands: &[Operand],
) -> Result<Vec<String>, Error> {
    let decimal_ty = emitter.decimal_ty()?;
    let mut decimal_parts = Vec::with_capacity(operands.len());
    for operand in operands {
        let value = emitter.emit_operand(operand, Some(&decimal_ty))?;
        let parts = emitter.decimal_value_to_parts(&value)?;
        decimal_parts.push(parts);
    }
    Ok(decimal_parts)
}

/// Return a constant vectorize hint if the operand is a compile-time enum.
pub(super) fn vectorize_hint_constant(vectorize: &Operand) -> Option<bool> {
    let Operand::Const(const_value) = vectorize else {
        return None;
    };
    let ConstValue::Enum {
        type_name, variant, ..
    } = &const_value.value
    else {
        return None;
    };
    if type_name != DECIMAL_VECTORIZE_CANONICAL {
        return None;
    }
    match variant.as_str() {
        "Decimal" => Some(true),
        "None" => Some(false),
        _ => None,
    }
}

/// Emit both scalar and SIMD runtime paths and select the appropriate result.
pub(super) fn emit_vectorized_runtime_result(
    emitter: &mut FunctionEmitter<'_>,
    op: &str,
    decimal_parts: &[String],
    rounding_value: &TypedValue,
    vectorize_operand: &Operand,
) -> Result<TypedValue, Error> {
    let variant_ty = emitter.decimal_intrinsic_variant_ty()?;
    let flags_ty = emitter.uint_ty()?;
    if let Some(vectorized) = vectorize_hint_constant(vectorize_operand) {
        let flags = if vectorized {
            DECIMAL_FLAG_VECTORIZE
        } else {
            0
        };
        let flags_value = emitter.emit_const_uint(flags, &flags_ty)?;
        let variant_scalar = emitter.emit_const_enum(
            "Std::Numeric::Decimal::DecimalIntrinsicVariant",
            "Scalar",
            0,
            &variant_ty,
        )?;

        let scalar_symbol = decimal_runtime_symbol(op, false)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal runtime op `{op}`")))?;

        emitter.externals.insert(scalar_symbol);

        let (status_scalar, value_scalar) = emitter.emit_decimal_runtime_components(
            scalar_symbol,
            decimal_parts,
            rounding_value,
            &flags_value,
        )?;

        return emitter.assemble_decimal_intrinsic_result(
            &status_scalar,
            &value_scalar,
            &variant_scalar,
        );
    }

    let vectorize_ty = emitter.decimal_vectorize_hint_ty()?;
    let vectorize_value = emitter.emit_typed_operand(vectorize_operand, &vectorize_ty)?;
    let is_vectorized = emitter.new_temp();
    writeln!(
        &mut emitter.builder,
        "  {is_vectorized} = icmp ne {ty} {repr}, 0",
        ty = vectorize_value.ty,
        repr = vectorize_value.repr
    )
    .ok();

    let scalar_flags = emitter.emit_const_uint(0, &flags_ty)?;
    let vector_flags = emitter.emit_const_uint(DECIMAL_FLAG_VECTORIZE, &flags_ty)?;
    let selected_flags = emitter.new_temp();
    writeln!(
        &mut emitter.builder,
        "  {selected_flags} = select i1 {is_vectorized}, {flags_ty} {vector}, {flags_ty} {scalar}",
        flags_ty = flags_ty,
        vector = vector_flags.repr,
        scalar = scalar_flags.repr,
    )
    .ok();

    let variant_scalar = emitter.emit_const_enum(
        "Std::Numeric::Decimal::DecimalIntrinsicVariant",
        "Scalar",
        0,
        &variant_ty,
    )?;

    let scalar_symbol = decimal_runtime_symbol(op, false)
        .ok_or_else(|| Error::Codegen(format!("unsupported decimal runtime op `{op}`")))?;

    emitter.externals.insert(scalar_symbol);

    let (status_scalar, value_scalar) = emitter.emit_decimal_runtime_components(
        scalar_symbol,
        decimal_parts,
        rounding_value,
        &TypedValue::new(selected_flags, &flags_ty),
    )?;

    emitter.assemble_decimal_intrinsic_result(&status_scalar, &value_scalar, &variant_scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{ConstOperand, DecimalIntrinsicKind};

    #[test]
    fn vectorize_hint_constant_handles_valid_enum() {
        let operand = Operand::Const(ConstOperand::new(ConstValue::Enum {
            type_name: DECIMAL_VECTORIZE_CANONICAL.into(),
            variant: "Decimal".into(),
            discriminant: 1,
        }));
        assert_eq!(Some(true), vectorize_hint_constant(&operand));

        let operand = Operand::Const(ConstOperand::new(ConstValue::Enum {
            type_name: DECIMAL_VECTORIZE_CANONICAL.into(),
            variant: "None".into(),
            discriminant: 0,
        }));
        assert_eq!(Some(false), vectorize_hint_constant(&operand));
    }

    #[test]
    fn vectorize_hint_constant_ignores_other_operands() {
        let operand = Operand::Const(ConstOperand::new(ConstValue::UInt(1)));
        assert_eq!(None, vectorize_hint_constant(&operand));

        let operand = Operand::Const(ConstOperand::new(ConstValue::Enum {
            type_name: "Std::Other::Enum".into(),
            variant: "Decimal".into(),
            discriminant: 0,
        }));
        assert_eq!(None, vectorize_hint_constant(&operand));
    }

    #[test]
    fn decimal_runtime_symbol_pairs_cover_basic_ops() {
        for op in ["add", "sub", "mul"] {
            let scalar = decimal_runtime_symbol(op, false)
                .unwrap_or_else(|| panic!("scalar symbol missing for {op}"));
            let simd = decimal_runtime_symbol(op, true)
                .unwrap_or_else(|| panic!("simd symbol missing for {op}"));
            assert!(
                scalar.contains(op),
                "expected scalar symbol `{scalar}` to reference `{op}`"
            );
            assert!(
                simd.contains(op),
                "expected simd symbol `{simd}` to reference `{op}`"
            );
        }

        assert!(
            decimal_runtime_symbol("noop", false).is_none(),
            "unknown ops should not resolve runtime symbols"
        );
    }

    #[test]
    fn decimal_intrinsic_operand_count_matches_helper_expectations() {
        assert_eq!(2, DecimalIntrinsicKind::Add.operand_count());
        assert_eq!(2, DecimalIntrinsicKind::Sub.operand_count());
        assert_eq!(3, DecimalIntrinsicKind::Fma.operand_count());
    }
}
