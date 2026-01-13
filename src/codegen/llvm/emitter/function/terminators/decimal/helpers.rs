use std::fmt::Write;

use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::{ConstValue, Ty, TypeLayout};

use super::shared::TypedValue;
use super::{DECIMAL_INTRINSIC_RESULT_CANONICAL, DECIMAL_PARTS_TY};
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;

impl<'a> FunctionEmitter<'a> {
    fn struct_field_indices(
        &self,
        type_name: &str,
        field_names: &[&str],
    ) -> Result<(String, Vec<usize>), Error> {
        let ty_str = map_type_owned(&Ty::named(type_name), Some(self.type_layouts))?
            .ok_or_else(|| Error::Codegen(format!("{type_name} type missing LLVM mapping")))?;
        let layout = self
            .type_layouts
            .layout_for_name(type_name)
            .ok_or_else(|| Error::Codegen(format!("type layout for `{type_name}` missing")))?;
        let struct_layout = match layout {
            TypeLayout::Struct(struct_layout) => struct_layout,
            _ => {
                return Err(Error::Codegen(format!(
                    "`{type_name}` is not a struct in layout metadata"
                )));
            }
        };
        let mut fields = struct_layout.fields.clone();
        fields.sort_by_key(|field| field.offset.unwrap_or(0));
        let mut aggregate_index = 0usize;
        let mut offset = 0usize;
        let mut indices = vec![None; field_names.len()];
        for field in &fields {
            let field_offset = field.offset.ok_or_else(|| {
                Error::Codegen(format!(
                    "field `{}` on `{type_name}` missing offset metadata",
                    field.name
                ))
            })?;
            if field_offset > offset {
                aggregate_index += 1;
                offset = field_offset;
            }
            if let Some(position) = field_names.iter().position(|name| *name == field.name) {
                indices[position] = Some(aggregate_index);
            }
            aggregate_index += 1;
            let (size, _) = self
                .type_layouts
                .size_and_align_for_ty(&field.ty)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "field `{}` on `{type_name}` missing size metadata",
                        field.name
                    ))
                })?;
            offset = offset
                .checked_add(size)
                .ok_or_else(|| Error::Codegen("struct layout exceeds range".into()))?;
        }
        let resolved = field_names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                indices[index].ok_or_else(|| Error::Codegen(format!("{name} field index missing")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((ty_str, resolved))
    }

    pub(super) fn emit_typed_operand(
        &mut self,
        operand: &crate::mir::Operand,
        ty: &str,
    ) -> Result<TypedValue, Error> {
        let value = self.emit_operand(operand, Some(ty))?;
        Ok(TypedValue::new(value.repr().to_string(), ty))
    }

    pub(super) fn emit_const_uint(&mut self, value: u128, ty: &str) -> Result<TypedValue, Error> {
        self.emit_typed_operand(
            &crate::mir::Operand::Const(crate::mir::ConstOperand::new(ConstValue::UInt(value))),
            ty,
        )
    }

    pub(super) fn emit_const_enum(
        &mut self,
        type_name: &str,
        variant: &str,
        discriminant: i128,
        ty: &str,
    ) -> Result<TypedValue, Error> {
        self.emit_typed_operand(
            &crate::mir::Operand::Const(crate::mir::ConstOperand::new(ConstValue::Enum {
                type_name: type_name.to_string(),
                variant: variant.to_string(),
                discriminant,
            })),
            ty,
        )
    }

    pub(super) fn assemble_decimal_intrinsic_result(
        &mut self,
        status: &TypedValue,
        value: &TypedValue,
        variant: &TypedValue,
    ) -> Result<TypedValue, Error> {
        let (result_ty, status_idx, value_idx, variant_idx) =
            self.decimal_intrinsic_result_layout()?;
        let insert_status = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert_status} = insertvalue {result_ty} undef, {} {}, {}",
            status.ty, status.repr, status_idx
        )
        .ok();
        let insert_value = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert_value} = insertvalue {result_ty} {insert_status}, {} {}, {}",
            value.ty, value.repr, value_idx
        )
        .ok();
        let insert_variant = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert_variant} = insertvalue {result_ty} {insert_value}, {} {}, {}",
            variant.ty, variant.repr, variant_idx
        )
        .ok();
        Ok(TypedValue::new(insert_variant, &result_ty))
    }

    pub(super) fn assign_decimal_intrinsic_result(
        &mut self,
        result: &TypedValue,
        destination: Option<&crate::mir::Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if let Some(place) = destination {
            if let Some(slot) = self.local_tys.get_mut(place.local.0) {
                *slot = Some(result.ty.clone());
            }
            self.decimal_local_structs
                .insert(place.local.0, DECIMAL_INTRINSIC_RESULT_CANONICAL);
            self.store_place(place, &ValueRef::new(result.repr.clone(), &result.ty))?;
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn decimal_intrinsic_result_layout(
        &self,
    ) -> Result<(String, usize, usize, usize), Error> {
        let (ty_str, indices) = self.struct_field_indices(
            DECIMAL_INTRINSIC_RESULT_CANONICAL,
            &["Status", "Value", "Variant"],
        )?;
        Ok((ty_str, indices[0], indices[1], indices[2]))
    }

    pub(super) fn decimal_ty(&self) -> Result<String, Error> {
        map_type_owned(&Ty::named("decimal"), Some(self.type_layouts))?
            .ok_or_else(|| Error::Codegen("decimal type missing LLVM mapping".into()))
    }

    pub(super) fn decimal_status_ty(&self) -> Result<String, Error> {
        map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalStatus"),
            Some(self.type_layouts),
        )?
        .ok_or_else(|| {
            Error::Codegen("DecimalStatus type missing LLVM mapping for runtime bridge".into())
        })
    }

    pub(super) fn decimal_rounding_mode_ty(&self) -> Result<String, Error> {
        map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalRoundingMode"),
            Some(self.type_layouts),
        )?
        .ok_or_else(|| Error::Codegen("DecimalRoundingMode type missing LLVM mapping".into()))
    }

    pub(super) fn decimal_vectorize_hint_ty(&self) -> Result<String, Error> {
        map_type_owned(
            &Ty::named(super::DECIMAL_VECTORIZE_CANONICAL),
            Some(self.type_layouts),
        )?
        .ok_or_else(|| Error::Codegen("DecimalVectorizeHint type missing LLVM mapping".into()))
    }

    pub(super) fn decimal_rounding_encoding_ty(&self) -> Result<String, Error> {
        map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalRoundingEncoding"),
            Some(self.type_layouts),
        )?
        .ok_or_else(|| Error::Codegen("DecimalRoundingEncoding type missing LLVM mapping".into()))
    }

    pub(super) fn decimal_rounding_encoding_layout(&self) -> Result<(String, usize), Error> {
        let type_name = "Std::Numeric::Decimal::DecimalRoundingEncoding";
        let ty_str = self.decimal_rounding_encoding_ty()?;
        let layout = self
            .type_layouts
            .layout_for_name(type_name)
            .ok_or_else(|| Error::Codegen(format!("type layout for `{type_name}` missing")))?;
        let struct_layout = match layout {
            TypeLayout::Struct(struct_layout) => struct_layout,
            _ => {
                return Err(Error::Codegen(format!(
                    "`{type_name}` is not a struct in layout metadata"
                )));
            }
        };
        let mut value_index = None;
        for (index, field) in struct_layout.fields.iter().enumerate() {
            if field.name == "Value" {
                value_index = Some(index);
                break;
            }
        }
        let value_index = value_index
            .ok_or_else(|| Error::Codegen("DecimalRoundingEncoding lacks Value".into()))?;
        Ok((ty_str, value_index))
    }

    pub(super) fn encode_decimal_rounding(
        &mut self,
        rounding: &TypedValue,
    ) -> Result<TypedValue, Error> {
        let (encoding_ty, field_index) = self.decimal_rounding_encoding_layout()?;
        if rounding.ty == encoding_ty {
            return Ok(rounding.clone());
        }
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = insertvalue {encoding_ty} undef, {} {}, {}",
            rounding.ty, rounding.repr, field_index
        )
        .ok();
        Ok(TypedValue::new(tmp, &encoding_ty))
    }

    pub(super) fn decimal_intrinsic_variant_ty(&self) -> Result<String, Error> {
        map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicVariant"),
            Some(self.type_layouts),
        )?
        .ok_or_else(|| Error::Codegen("DecimalIntrinsicVariant type missing LLVM mapping".into()))
    }

    pub(super) fn uint_ty(&self) -> Result<String, Error> {
        map_type_owned(&Ty::named("uint"), Some(self.type_layouts))?
            .ok_or_else(|| Error::Codegen("`uint` type missing LLVM mapping".into()))
    }

    pub(super) fn decimal_runtime_call_layout(&self) -> Result<(String, usize, usize), Error> {
        let (ty_str, indices) = self.struct_field_indices(
            "Std::Numeric::Decimal::DecimalRuntimeCall",
            &["Status", "Value"],
        )?;
        Ok((ty_str, indices[0], indices[1]))
    }

    pub(super) fn decimal_value_to_parts(&mut self, value: &ValueRef) -> Result<String, Error> {
        let repr = value.repr();
        let lo = self.new_temp();
        writeln!(&mut self.builder, "  {lo} = trunc i128 {repr} to i32").ok();

        let shift32 = self.new_temp();
        writeln!(&mut self.builder, "  {shift32} = lshr i128 {repr}, 32").ok();
        let mid = self.new_temp();
        writeln!(&mut self.builder, "  {mid} = trunc i128 {shift32} to i32").ok();

        let shift64 = self.new_temp();
        writeln!(&mut self.builder, "  {shift64} = lshr i128 {repr}, 64").ok();
        let hi = self.new_temp();
        writeln!(&mut self.builder, "  {hi} = trunc i128 {shift64} to i32").ok();

        let shift96 = self.new_temp();
        writeln!(&mut self.builder, "  {shift96} = lshr i128 {repr}, 96").ok();
        let flags = self.new_temp();
        writeln!(&mut self.builder, "  {flags} = trunc i128 {shift96} to i32").ok();

        let insert0 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert0} = insertvalue {DECIMAL_PARTS_TY} undef, i32 {lo}, 0"
        )
        .ok();
        let insert1 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert1} = insertvalue {DECIMAL_PARTS_TY} {insert0}, i32 {mid}, 1"
        )
        .ok();
        let insert2 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert2} = insertvalue {DECIMAL_PARTS_TY} {insert1}, i32 {hi}, 2"
        )
        .ok();
        let insert3 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {insert3} = insertvalue {DECIMAL_PARTS_TY} {insert2}, i32 {flags}, 3"
        )
        .ok();
        Ok(insert3)
    }

    pub(super) fn decimal_parts_to_value(&mut self, parts: &str) -> Result<String, Error> {
        let lo = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {lo} = extractvalue {DECIMAL_PARTS_TY} {parts}, 0"
        )
        .ok();
        let mid = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {mid} = extractvalue {DECIMAL_PARTS_TY} {parts}, 1"
        )
        .ok();
        let hi = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {hi} = extractvalue {DECIMAL_PARTS_TY} {parts}, 2"
        )
        .ok();
        let flags = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {flags} = extractvalue {DECIMAL_PARTS_TY} {parts}, 3"
        )
        .ok();

        let lo_wide = self.new_temp();
        writeln!(&mut self.builder, "  {lo_wide} = zext i32 {lo} to i128").ok();
        let mid_wide = self.new_temp();
        writeln!(&mut self.builder, "  {mid_wide} = zext i32 {mid} to i128").ok();
        let mid_shift = self.new_temp();
        writeln!(&mut self.builder, "  {mid_shift} = shl i128 {mid_wide}, 32").ok();
        let accum0 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {accum0} = or i128 {lo_wide}, {mid_shift}"
        )
        .ok();

        let hi_wide = self.new_temp();
        writeln!(&mut self.builder, "  {hi_wide} = zext i32 {hi} to i128").ok();
        let hi_shift = self.new_temp();
        writeln!(&mut self.builder, "  {hi_shift} = shl i128 {hi_wide}, 64").ok();
        let accum1 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {accum1} = or i128 {accum0}, {hi_shift}"
        )
        .ok();

        let flags_wide = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {flags_wide} = zext i32 {flags} to i128"
        )
        .ok();
        let flags_shift = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {flags_shift} = shl i128 {flags_wide}, 96"
        )
        .ok();
        let combined = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {combined} = or i128 {accum1}, {flags_shift}"
        )
        .ok();
        Ok(combined)
    }
}
