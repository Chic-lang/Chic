use std::fmt::Write;

use crate::codegen::llvm::types::{constrained_rounding_string, map_type_owned};
use crate::error::Error;
use crate::mir::TypeLayout;
use crate::mir::casts::{IntInfo, float_info, int_info};
use crate::mir::{CastKind, Operand, RoundingMode, Ty};

use super::super::builder::FunctionEmitter;
use super::value_ref::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_cast(
        &mut self,
        kind: CastKind,
        operand: &Operand,
        source: &Ty,
        target: &Ty,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        let source_name = source.canonical_name();
        let target_name = target.canonical_name();
        let source_ty = map_type_owned(source, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "cast source type `{source_name}` is not supported by LLVM backend"
            ))
        })?;
        let target_ty = map_type_owned(target, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "cast target type `{target_name}` is not supported by LLVM backend"
            ))
        })?;
        let pointer_size = crate::mir::pointer_size() as u32;
        let is_int_like =
            |ty: &str| ty.starts_with('i') && ty.chars().skip(1).all(|c| c.is_ascii_digit());
        let is_ptr_like = |ty: &str| ty == "ptr" || ty.ends_with('*');
        let int_info_for = |name: &str| -> Option<IntInfo> {
            if let Some(info) = int_info(&self.type_layouts.primitive_registry, name, pointer_size)
            {
                return Some(info);
            }
            let layout = self.type_layouts.layout_for_name(name)?;
            match layout {
                TypeLayout::Enum(enum_layout) => {
                    if let Some(info) = enum_layout.underlying_info {
                        return Some(info);
                    }
                    let bits = enum_layout.size.map(|size| size.saturating_mul(8) as u16)?;
                    if bits == 0 {
                        return None;
                    }
                    Some(IntInfo {
                        bits,
                        signed: !enum_layout.is_flags,
                    })
                }
                _ => None,
            }
        };

        let operand_value = self.emit_operand(operand, Some(&source_ty))?;
        let rounding = constrained_rounding_string(self.rounding_mode());
        let constrained_float_suffix = |ty: &str| -> Option<&str> {
            match ty {
                "half" => Some("f16"),
                "bfloat" => Some("bf16"),
                "float" => Some("f32"),
                "double" => Some("f64"),
                "fp128" => Some("f128"),
                "x86_fp80" => Some("f80"),
                _ => None,
            }
        };

        let result = match kind {
            CastKind::IntToInt => {
                if is_ptr_like(&source_ty) && is_int_like(&target_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = ptrtoint {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    return Ok(ValueRef::new(tmp, &target_ty));
                }

                if is_int_like(&source_ty) && is_ptr_like(&target_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = inttoptr {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    return Ok(ValueRef::new(tmp, &target_ty));
                }

                if source_ty == target_ty {
                    return Ok(operand_value);
                }

                let source_info = int_info_for(&source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let target_info = int_info_for(&target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;

                if source_info.bits == target_info.bits {
                    if source_ty == target_ty {
                        operand_value
                    } else {
                        self.bitcast_value(&operand_value, &source_ty, &target_ty)?
                    }
                } else if source_info.bits > target_info.bits {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = trunc {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                } else {
                    let op = if source_info.signed { "sext" } else { "zext" };
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = {op} {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                }
            }
            CastKind::IntToFloat => {
                let source_info = int_info_for(&source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let op = if source_info.signed {
                    "sitofp"
                } else {
                    "uitofp"
                };
                let constrained_ty = constrained_float_suffix(&target_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unsupported constrained float target type `{target_ty}` in int->float cast"
                    ))
                })?;
                let constrained_source = &source_ty;
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {target_ty} @llvm.experimental.constrained.{op}.{constrained_ty}.{constrained_source}({source_ty} {}, metadata !\"{rounding}\", metadata !\"fpexcept.strict\")",
                    operand_value.repr(),
                )
                .ok();
                ValueRef::new(tmp, &target_ty)
            }
            CastKind::FloatToInt => {
                let target_info = int_info_for(&target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                let op = if target_info.signed {
                    "fptosi"
                } else {
                    "fptoui"
                };
                let constrained_source = constrained_float_suffix(&source_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unsupported constrained float source type `{source_ty}` in float->int cast"
                    ))
                })?;
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {target_ty} @llvm.experimental.constrained.{op}.{target_ty}.{constrained_source}({source_ty} {}, metadata !\"fpexcept.strict\")",
                    operand_value.repr(),
                )
                .ok();
                ValueRef::new(tmp, &target_ty)
            }
            CastKind::FloatToFloat => {
                let source_info = float_info(&self.type_layouts.primitive_registry, &source_name)
                    .ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine float metadata for `{source_name}`"
                    ))
                })?;
                let target_info = float_info(&self.type_layouts.primitive_registry, &target_name)
                    .ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine float metadata for `{target_name}`"
                    ))
                })?;
                let constrained_target = constrained_float_suffix(&target_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unsupported constrained float target type `{target_ty}` in float cast"
                    ))
                })?;
                let constrained_source = constrained_float_suffix(&source_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unsupported constrained float source type `{source_ty}` in float cast"
                    ))
                })?;
                if source_info.bits == target_info.bits {
                    if source_ty == target_ty {
                        operand_value
                    } else {
                        self.bitcast_value(&operand_value, &source_ty, &target_ty)?
                    }
                } else if source_info.bits < target_info.bits {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = call {target_ty} @llvm.experimental.constrained.fpext.{constrained_target}.{constrained_source}({source_ty} {}, metadata !\"fpexcept.strict\")",
                        operand_value.repr(),
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                } else {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = call {target_ty} @llvm.experimental.constrained.fptrunc.{constrained_target}.{constrained_source}({source_ty} {}, metadata !\"{rounding}\", metadata !\"fpexcept.strict\")",
                        operand_value.repr(),
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                }
            }
            CastKind::PointerToInt => {
                if is_int_like(&target_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = ptrtoint {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                } else if is_ptr_like(&target_ty) {
                    return self.bitcast_value(&operand_value, &source_ty, &target_ty);
                } else {
                    return Err(Error::Codegen(format!(
                        "pointer cast target `{target_ty}` is not an integer or pointer type"
                    )));
                }
            }
            CastKind::IntToPointer => {
                if is_int_like(&source_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = inttoptr {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    ValueRef::new(tmp, &target_ty)
                } else if is_ptr_like(&source_ty) && is_ptr_like(&target_ty) {
                    return self.bitcast_value(&operand_value, &source_ty, &target_ty);
                } else {
                    return Err(Error::Codegen(format!(
                        "int-to-pointer cast source `{source_ty}` is not an integer type"
                    )));
                }
            }
            CastKind::DynTrait => {
                return Err(Error::Codegen(format!(
                    "cast kind `{kind:?}` is not supported by LLVM backend"
                )));
            }
            CastKind::Unknown => {
                if source_ty == target_ty {
                    return Ok(operand_value);
                }
                if is_int_like(&source_ty) && is_int_like(&target_ty) {
                    let parse_bits = |ty: &str| ty[1..].parse::<u32>().ok();
                    let src_bits = parse_bits(&source_ty);
                    let tgt_bits = parse_bits(&target_ty);
                    if let (Some(src), Some(tgt)) = (src_bits, tgt_bits) {
                        if src == tgt {
                            return self.bitcast_value(&operand_value, &source_ty, &target_ty);
                        }
                        let tmp = self.new_temp();
                        if src > tgt {
                            writeln!(
                                &mut self.builder,
                                "  {tmp} = trunc {source_ty} {} to {target_ty}",
                                operand_value.repr()
                            )
                            .ok();
                        } else {
                            writeln!(
                                &mut self.builder,
                                "  {tmp} = zext {source_ty} {} to {target_ty}",
                                operand_value.repr()
                            )
                            .ok();
                        }
                        return Ok(ValueRef::new(tmp, &target_ty));
                    }
                }
                if is_ptr_like(&source_ty) && is_int_like(&target_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = ptrtoint {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    return Ok(ValueRef::new(tmp, &target_ty));
                }
                if is_int_like(&source_ty) && is_ptr_like(&target_ty) {
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = inttoptr {source_ty} {} to {target_ty}",
                        operand_value.repr()
                    )
                    .ok();
                    return Ok(ValueRef::new(tmp, &target_ty));
                }
                self.bitcast_value(&operand_value, &source_ty, &target_ty)?
            }
        };

        if let Some(exp) = expected {
            if exp != target_ty {
                return self.bitcast_value(&result, &target_ty, exp);
            }
        }

        Ok(result)
    }

    pub(crate) fn bitcast_value(
        &mut self,
        value: &ValueRef,
        from_ty: &str,
        to_ty: &str,
    ) -> Result<ValueRef, Error> {
        if from_ty == to_ty {
            return Ok(ValueRef::new(value.repr().to_string(), to_ty));
        }
        let from_aggregate = from_ty.starts_with('{') || from_ty.starts_with('[');
        let to_aggregate = to_ty.starts_with('{') || to_ty.starts_with('[');
        if from_aggregate && to_aggregate {
            // LLVM does not permit bitcasting aggregate values directly; spill and reload
            // through a appropriately-typed pointer instead.
            let spill_ptr = self.new_temp();
            writeln!(&mut self.builder, "  {spill_ptr} = alloca {from_ty}").ok();
            writeln!(
                &mut self.builder,
                "  store {from_ty} {}, ptr {spill_ptr}",
                value.repr()
            )
            .ok();
            let cast_ptr = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cast_ptr} = bitcast {from_ty}* {spill_ptr} to {to_ty}*"
            )
            .ok();
            let load_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {load_tmp} = load {to_ty}, ptr {cast_ptr}"
            )
            .ok();
            return Ok(ValueRef::new(load_tmp, to_ty));
        }
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = bitcast {from_ty} {} to {to_ty}",
            value.repr()
        )
        .ok();
        Ok(ValueRef::new(tmp, to_ty))
    }

    #[inline]
    pub(crate) fn rounding_mode(&self) -> RoundingMode {
        self.current_rounding
            .unwrap_or(RoundingMode::NearestTiesToEven)
    }
}
