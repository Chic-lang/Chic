use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::error::Error;
use crate::mir::{BlockId, ConstValue, DecimalIntrinsic, DecimalIntrinsicKind, Operand, Place};

use super::runtime::{decimal_runtime_symbol, runtime_spec};
use super::shared::{
    TypedValue, collect_decimal_parts, emit_vectorized_runtime_result, vectorize_hint_constant,
};
use super::{DECIMAL_FLAG_VECTORIZE, DECIMAL_INTRINSIC_RESULT_CANONICAL};

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn try_emit_decimal_call(
        &mut self,
        canonical: &str,
        repr: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<bool, Error> {
        let canonical_lower = canonical.to_ascii_lowercase();
        let repr_lower = repr.to_ascii_lowercase();
        if let Some(spec) = runtime_spec(&canonical_lower, &repr_lower) {
            self.externals.insert(spec.symbol);
            self.emit_decimal_runtime_call(&spec, args, destination, target)?;
            return Ok(true);
        }
        if let Some(spec) = super::wrappers::wrapper_spec(&canonical_lower, canonical) {
            self.emit_decimal_wrapper_call(&spec, args, destination, target)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn emit_decimal_runtime_by_repr(
        &mut self,
        repr: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<bool, Error> {
        let repr_lower = repr.to_ascii_lowercase();
        if let Some(spec) = runtime_spec("", &repr_lower) {
            self.externals.insert(spec.symbol);
            self.emit_decimal_runtime_call(&spec, args, destination, target)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn emit_decimal_intrinsic_value(
        &mut self,
        decimal: &DecimalIntrinsic,
    ) -> Result<TypedValue, Error> {
        let op = decimal_op_from_kind(decimal.kind);

        let mut decimal_operands: Vec<Operand> = Vec::with_capacity(3);
        decimal_operands.push(decimal.lhs.clone());
        decimal_operands.push(decimal.rhs.clone());
        if let Some(addend) = &decimal.addend {
            decimal_operands.push(addend.clone());
        }

        let rounding_mode_ty = self.decimal_rounding_mode_ty()?;
        let flags_ty = self.uint_ty()?;
        let variant_ty = self.decimal_intrinsic_variant_ty()?;

        let decimal_parts = collect_decimal_parts(self, &decimal_operands)?;
        let rounding_operand = self.emit_typed_operand(&decimal.rounding, &rounding_mode_ty)?;
        let rounding_value = self.encode_decimal_rounding(&rounding_operand)?;

        if let Some(vectorized) = vectorize_hint_constant(&decimal.vectorize) {
            let flags_value = if vectorized {
                self.emit_const_uint(DECIMAL_FLAG_VECTORIZE, &flags_ty)?
            } else {
                self.emit_const_uint(0, &flags_ty)?
            };
            let variant_value = self.emit_const_enum(
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
                &variant_ty,
            )?;
            let symbol = decimal_runtime_symbol(op, false)
                .ok_or_else(|| Error::Codegen(format!("unsupported decimal runtime op `{op}`")))?;
            self.externals.insert(symbol);
            let (status, value) = self.emit_decimal_runtime_components(
                symbol,
                &decimal_parts,
                &rounding_value,
                &flags_value,
            )?;
            return self.assemble_decimal_intrinsic_result(&status, &value, &variant_value);
        }

        emit_vectorized_runtime_result(
            self,
            op,
            &decimal_parts,
            &rounding_value,
            &decimal.vectorize,
        )
    }

    pub(crate) fn emit_decimal_intrinsic_assign(
        &mut self,
        place: &Place,
        decimal: &DecimalIntrinsic,
    ) -> Result<(), Error> {
        let result = self.emit_decimal_intrinsic_value(decimal)?;
        if place.projection.is_empty() {
            let ty_string = result.ty().to_string();
            if let Some(slot) = self.local_tys.get_mut(place.local.0) {
                *slot = Some(ty_string.clone());
            }
            self.decimal_local_structs
                .insert(place.local.0, DECIMAL_INTRINSIC_RESULT_CANONICAL);
        }
        let value_ref = result.into_value_ref();
        self.store_place(place, &value_ref)?;
        Ok(())
    }

    pub(super) fn emit_decimal_intrinsic_fixed(
        &mut self,
        op: &str,
        decimal_args: &[Operand],
        rounding_operand: Option<&Operand>,
        rounding_const: Option<ConstValue>,
        flags: u128,
        variant: (&'static str, &'static str, i128),
        simd: bool,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let rounding_mode_ty = self.decimal_rounding_mode_ty()?;
        let flags_ty = self.uint_ty()?;
        let variant_ty = self.decimal_intrinsic_variant_ty()?;

        let decimal_parts = collect_decimal_parts(self, decimal_args)?;

        let rounding_value = if let Some(operand) = rounding_operand {
            let operand = self.emit_typed_operand(operand, &rounding_mode_ty)?;
            self.encode_decimal_rounding(&operand)?
        } else if let Some(ConstValue::Enum {
            type_name,
            variant,
            discriminant,
        }) = rounding_const.as_ref()
        {
            let value =
                self.emit_const_enum(type_name, variant, *discriminant, &rounding_mode_ty)?;
            self.encode_decimal_rounding(&value)?
        } else {
            return Err(Error::Codegen(format!(
                "`{}` intrinsic missing rounding operand",
                op
            )));
        };
        let flags_value = self.emit_const_uint(flags, &flags_ty)?;
        let variant_value = self.emit_const_enum(variant.0, variant.1, variant.2, &variant_ty)?;

        let symbol = decimal_runtime_symbol(op, simd)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal runtime op `{}`", op)))?;
        self.externals.insert(symbol);

        let (status, value) = self.emit_decimal_runtime_components(
            symbol,
            &decimal_parts,
            &rounding_value,
            &flags_value,
        )?;
        let result = self.assemble_decimal_intrinsic_result(&status, &value, &variant_value)?;
        self.assign_decimal_intrinsic_result(&result, destination, target)
    }

    pub(super) fn emit_decimal_intrinsic_with_options(
        &mut self,
        op: &str,
        decimal_args: &[Operand],
        rounding_operand: &Operand,
        vectorize_operand: &Operand,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let rounding_mode_ty = self.decimal_rounding_mode_ty()?;

        let decimal_parts = collect_decimal_parts(self, decimal_args)?;

        let rounding_operand = self.emit_typed_operand(rounding_operand, &rounding_mode_ty)?;
        let rounding_value = self.encode_decimal_rounding(&rounding_operand)?;
        let result = emit_vectorized_runtime_result(
            self,
            op,
            &decimal_parts,
            &rounding_value,
            vectorize_operand,
        )?;
        self.assign_decimal_intrinsic_result(&result, destination, target)
    }
}

pub(super) fn decimal_op_from_kind(kind: DecimalIntrinsicKind) -> &'static str {
    match kind {
        DecimalIntrinsicKind::Add => "add",
        DecimalIntrinsicKind::Sub => "sub",
        DecimalIntrinsicKind::Mul => "mul",
        DecimalIntrinsicKind::Div => "div",
        DecimalIntrinsicKind::Rem => "rem",
        DecimalIntrinsicKind::Fma => "fma",
    }
}
