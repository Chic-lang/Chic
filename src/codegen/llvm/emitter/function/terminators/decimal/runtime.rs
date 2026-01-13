use std::fmt::Write;

use super::shared::TypedValue;
use super::{DECIMAL_PARTS_TY, DECIMAL_RUNTIME_RESULT_TY};
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::error::Error;
use crate::mir::{BlockId, Operand, Place};

pub(super) struct DecimalRuntimeSpec {
    pub symbol: &'static str,
    pub decimal_args: usize,
}

pub(super) fn runtime_spec(canonical_lower: &str, repr_lower: &str) -> Option<DecimalRuntimeSpec> {
    let matches = |suffix: &str| {
        canonical_lower == suffix
            || canonical_lower.ends_with(&format!("::{suffix}"))
            || repr_lower == suffix.replace("::", ".")
            || repr_lower.ends_with(&format!(".{suffix}"))
    };

    let (symbol, decimal_args) = if matches("runtimeintrinsics::chic_rt_decimal_add") {
        ("chic_rt_decimal_add", 2)
    } else if matches("runtimeintrinsics::chic_rt_decimal_sub") {
        ("chic_rt_decimal_sub", 2)
    } else if matches("runtimeintrinsics::chic_rt_decimal_mul") {
        ("chic_rt_decimal_mul", 2)
    } else if matches("runtimeintrinsics::chic_rt_decimal_div") {
        ("chic_rt_decimal_div", 2)
    } else if matches("runtimeintrinsics::chic_rt_decimal_rem") {
        ("chic_rt_decimal_rem", 2)
    } else if matches("runtimeintrinsics::chic_rt_decimal_fma") {
        ("chic_rt_decimal_fma", 3)
    } else {
        return None;
    };
    Some(DecimalRuntimeSpec {
        symbol,
        decimal_args,
    })
}

pub(crate) fn decimal_runtime_symbol(op: &str, simd: bool) -> Option<&'static str> {
    match (op, simd) {
        ("add", _) => Some("chic_rt_decimal_add"),
        ("sub", _) => Some("chic_rt_decimal_sub"),
        ("mul", _) => Some("chic_rt_decimal_mul"),
        ("div", _) => Some("chic_rt_decimal_div"),
        ("rem", _) => Some("chic_rt_decimal_rem"),
        ("fma", _) => Some("chic_rt_decimal_fma"),
        _ => None,
    }
}

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_decimal_runtime_components(
        &mut self,
        symbol: &str,
        decimal_parts: &[String],
        rounding: &TypedValue,
        flags: &TypedValue,
    ) -> Result<(TypedValue, TypedValue), Error> {
        let result_slot = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result_slot} = alloca {DECIMAL_RUNTIME_RESULT_TY}"
        )
        .ok();

        let mut call_args = Vec::with_capacity(decimal_parts.len() + 3);
        call_args.push(format!(
            "ptr sret({DECIMAL_RUNTIME_RESULT_TY}) align 4 {result_slot}"
        ));
        for part in decimal_parts {
            let slot = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot} = alloca {DECIMAL_PARTS_TY}, align 4"
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  store {DECIMAL_PARTS_TY} {part}, ptr {slot}, align 4"
            )
            .ok();
            call_args.push(format!("ptr {slot}"));
        }
        call_args.push(format!("{} {}", rounding.ty, rounding.repr));
        call_args.push(format!("{} {}", flags.ty, flags.repr));
        let args_repr = call_args.join(", ");

        writeln!(&mut self.builder, "  call void @{symbol}({args_repr})").ok();

        let call_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {call_tmp} = load {DECIMAL_RUNTIME_RESULT_TY}, ptr {result_slot}"
        )
        .ok();

        let status_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {status_tmp} = extractvalue {DECIMAL_RUNTIME_RESULT_TY} {call_tmp}, 0"
        )
        .ok();
        let parts_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {parts_tmp} = extractvalue {DECIMAL_RUNTIME_RESULT_TY} {call_tmp}, 1"
        )
        .ok();

        let decimal_tmp = self.decimal_parts_to_value(&parts_tmp)?;
        let status_ty = self.decimal_status_ty()?;
        let decimal_ty = self.decimal_ty()?;

        Ok((
            TypedValue::new(status_tmp, &status_ty),
            TypedValue::new(decimal_tmp, &decimal_ty),
        ))
    }

    pub(super) fn assemble_decimal_runtime_call(
        &mut self,
        status: &TypedValue,
        value: &TypedValue,
    ) -> Result<TypedValue, Error> {
        let (call_ty, status_index, value_index) = self.decimal_runtime_call_layout()?;
        let insert_status = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert_status} = insertvalue {call_ty} undef, {} {}, {}",
            status.ty, status.repr, status_index
        )
        .ok();
        let insert_value = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert_value} = insertvalue {call_ty} {insert_status}, {} {}, {}",
            value.ty, value.repr, value_index
        )
        .ok();
        Ok(TypedValue::new(insert_value, &call_ty))
    }

    pub(super) fn emit_decimal_runtime_call(
        &mut self,
        spec: &DecimalRuntimeSpec,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let expected_len = spec.decimal_args + 2;
        if args.len() != expected_len {
            return Err(Error::Codegen(format!(
                "`{}` expects {} arguments, found {}",
                spec.symbol,
                expected_len,
                args.len()
            )));
        }

        let decimal_ty = self.decimal_ty()?;
        let rounding_mode_ty = self.decimal_rounding_mode_ty()?;
        let flags_ty = self.uint_ty()?;

        let mut decimal_parts = Vec::with_capacity(spec.decimal_args);
        for operand in &args[..spec.decimal_args] {
            let value = self.emit_operand(operand, Some(&decimal_ty))?;
            let parts = self.decimal_value_to_parts(&value)?;
            decimal_parts.push(parts);
        }

        let rounding_operand =
            self.emit_typed_operand(&args[spec.decimal_args], &rounding_mode_ty)?;
        let rounding = self.encode_decimal_rounding(&rounding_operand)?;
        let flags = self.emit_typed_operand(&args[spec.decimal_args + 1], &flags_ty)?;

        let (status, value) =
            self.emit_decimal_runtime_components(spec.symbol, &decimal_parts, &rounding, &flags)?;
        let result = self.assemble_decimal_runtime_call(&status, &value)?;

        if let Some(place) = destination {
            if let Some(slot) = self.local_tys.get_mut(place.local.0) {
                *slot = Some(result.ty.clone());
            }
            self.decimal_local_structs
                .insert(place.local.0, "Std::Numeric::Decimal::DecimalRuntimeCall");
            self.store_place(place, &ValueRef::new(result.repr.clone(), &result.ty))?;
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }
}
