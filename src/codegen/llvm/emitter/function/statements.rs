use std::fmt::Write;

use crate::codegen::llvm::signatures::resolve_function_name;
use crate::drop_glue::drop_glue_symbol_for;
use crate::error::Error;
use crate::frontend::diagnostics::Span;
use crate::mir::{
    AggregateKind, ConstOperand, ConstValue, InlineAsm, InlineAsmOperandKind, InlineAsmRegister,
    InlineAsmRegisterClass, InlineAsmTemplatePiece, NumericIntrinsicKind, NumericWidth, Operand,
    Place, ProjectionElem, Rvalue, Statement, StatementKind, Ty, TypeLayout,
};
use crate::target::TargetArch;

use super::builder::FunctionEmitter;
use super::values::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_statement(&mut self, statement: &Statement) -> Result<(), Error> {
        match &statement.kind {
            StatementKind::Assign { place, value } => {
                if let Rvalue::Aggregate { kind, fields } = value {
                    self.emit_aggregate_assignment(place, kind, fields, statement.span)?;
                    return Ok(());
                }
                if let Rvalue::NumericIntrinsic(intrinsic) = value {
                    self.emit_numeric_intrinsic_assign(place, intrinsic)?;
                    return Ok(());
                }
                if let Rvalue::DecimalIntrinsic(decimal) = value {
                    self.emit_decimal_intrinsic_assign(place, decimal)?;
                    return Ok(());
                }
                if self.place_is_string(place)? && self.emit_string_assignment(place, value)? {
                    return Ok(());
                }
                if self.place_is_vec(place)? && self.emit_vec_assignment(place, value)? {
                    return Ok(());
                }
                if self.place_is_rc(place)? && self.emit_rc_assignment(place, value)? {
                    return Ok(());
                }
                if self.place_is_arc(place)? && self.emit_arc_assignment(place, value)? {
                    return Ok(());
                }
                let ty = if place.projection.is_empty() && self.is_reference_param(place.local.0) {
                    Some(self.param_value_type(place.local.0)?)
                } else {
                    self.place_type(place)?
                };
                let value = self.emit_rvalue(value, ty.as_deref())?;
                self.store_place(place, &value)?;
            }
            StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::EnterUnsafe
            | StatementKind::ExitUnsafe
            | StatementKind::Nop
            | StatementKind::MarkFallibleHandled { .. }
            | StatementKind::EnqueueKernel { .. }
            | StatementKind::EnqueueCopy { .. }
            | StatementKind::RecordEvent { .. }
            | StatementKind::WaitEvent { .. }
            | StatementKind::Assert { .. } => {}
            StatementKind::Pending(pending) => {
                let detail = pending
                    .detail
                    .as_deref()
                    .unwrap_or("pending statement with no detail");
                return Err(Error::Codegen(format!(
                    "pending statement {:?} encountered in {}: {detail}",
                    pending.kind, self.function.name,
                )));
            }
            StatementKind::Borrow { .. }
            | StatementKind::DeferDrop { .. }
            | StatementKind::Retag { .. }
            | StatementKind::Eval(_) => {}
            StatementKind::Deinit(place) => {
                self.emit_deinit_statement(place)?;
            }
            StatementKind::DefaultInit { place } => {
                self.emit_zero_init_statement(place)?;
            }
            StatementKind::ZeroInit { place } => {
                self.emit_zero_init_statement(place)?;
            }
            StatementKind::ZeroInitRaw { pointer, length } => {
                self.emit_zero_init_raw_statement(pointer, length)?;
            }
            StatementKind::Drop { place, .. } => {
                if self.place_is_string(place)? {
                    self.emit_string_drop(place)?;
                } else if self.place_is_vec(place)? {
                    self.emit_vec_drop(place)?;
                } else if self.place_is_rc(place)? {
                    self.emit_rc_drop(place)?;
                } else if self.place_is_arc(place)? {
                    self.emit_arc_drop(place)?;
                } else {
                    let ty = self.mir_ty_of_place(place)?;
                    if !self.emit_drop_glue_for(place, &ty)? {
                        self.emit_drop_missing(place)?;
                    }
                }
            }
            StatementKind::MmioStore { target, value } => {
                self.emit_mmio_store(target, value)?;
            }
            StatementKind::StaticStore { id, value } => {
                let llvm_ty = self.static_llvm_type(*id)?;
                let operand = self.emit_operand(value, Some(&llvm_ty))?;
                let global = self.static_symbol(*id)?;
                writeln!(
                    &mut self.builder,
                    "  store {llvm_ty} {}, ptr {global}",
                    operand.repr()
                )
                .ok();
            }
            StatementKind::InlineAsm(asm) => {
                self.emit_inline_asm(asm)?;
            }
            StatementKind::AtomicStore {
                target,
                value,
                order,
            } => {
                let ty = self.place_type(target)?.ok_or_else(|| {
                    Error::Codegen("atomic store requires addressable type".into())
                })?;
                Self::ensure_atomic_ty(&ty)?;
                let operand = self.emit_operand(value, Some(&ty))?;
                let ptr = self.place_ptr(target)?;
                let align = self.atomic_alignment_for_place(target)?;
                let ordering = Self::llvm_atomic_ordering(*order);
                let alias_suffix = self.alias_suffix_for_place(target).unwrap_or_default();
                writeln!(
                    &mut self.builder,
                    "  store atomic {ty} {}, ptr {ptr} {ordering}, align {align}{alias_suffix}",
                    operand.repr(),
                )
                .ok();
            }
            StatementKind::AtomicFence { order, scope } => {
                let ordering = Self::llvm_fence_order(*scope, *order);
                writeln!(&mut self.builder, "  fence {ordering}").ok();
            }
        }
        Ok(())
    }

    fn emit_aggregate_assignment(
        &mut self,
        place: &Place,
        kind: &AggregateKind,
        fields: &[Operand],
        span: Option<Span>,
    ) -> Result<(), Error> {
        match kind {
            AggregateKind::Tuple | AggregateKind::Array => {
                for (index, field) in fields.iter().enumerate() {
                    let mut field_place = place.clone();
                    field_place
                        .projection
                        .push(ProjectionElem::Field(index as u32));
                    let stmt = Statement {
                        span,
                        kind: StatementKind::Assign {
                            place: field_place,
                            value: Rvalue::Use(field.clone()),
                        },
                    };
                    self.emit_statement(&stmt)?;
                }
                Ok(())
            }
            AggregateKind::Adt { name, variant } => {
                if variant.is_some() {
                    return Err(Error::Codegen(
                        "enum variants in aggregates are not yet supported in LLVM backend".into(),
                    ));
                }
                let Some(layout) = self.type_layouts.types.get(name.as_str()) else {
                    return Err(Error::Codegen(format!(
                        "type layout for aggregate `{}` missing in LLVM backend",
                        name
                    )));
                };
                let struct_layout = match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data,
                    other => {
                        return Err(Error::Codegen(format!(
                            "aggregate assignment only supports struct/class layouts in LLVM backend; found {other:?}"
                        )));
                    }
                };
                if struct_layout.fields.len() != fields.len() {
                    return Err(Error::Codegen(format!(
                        "aggregate for `{}` provided {} fields but layout expects {}",
                        name,
                        fields.len(),
                        struct_layout.fields.len()
                    )));
                }

                for (field, value) in struct_layout.fields.iter().zip(fields.iter()) {
                    let mut field_place = place.clone();
                    field_place
                        .projection
                        .push(ProjectionElem::Field(field.index));
                    let stmt = Statement {
                        span,
                        kind: StatementKind::Assign {
                            place: field_place,
                            value: Rvalue::Use(value.clone()),
                        },
                    };
                    self.emit_statement(&stmt)?;
                }
                Ok(())
            }
        }
    }

    fn llvm_int_ty_for_width(&self, width: NumericWidth) -> &'static str {
        match width {
            NumericWidth::W8 => "i8",
            NumericWidth::W16 => "i16",
            NumericWidth::W32 => "i32",
            NumericWidth::W64 | NumericWidth::Pointer => "i64",
            NumericWidth::W128 => "i128",
        }
    }

    fn width_bits(&self, width: NumericWidth) -> u32 {
        match width {
            NumericWidth::W8 => 8,
            NumericWidth::W16 => 16,
            NumericWidth::W32 => 32,
            NumericWidth::W64 | NumericWidth::Pointer => 64,
            NumericWidth::W128 => 128,
        }
    }

    fn overflow_intrinsic_name(
        &self,
        kind: NumericIntrinsicKind,
        signed: bool,
        bits: u32,
    ) -> Option<&'static str> {
        match (kind, signed, bits) {
            (NumericIntrinsicKind::TryAdd, true, 8) => Some("llvm.sadd.with.overflow.i8"),
            (NumericIntrinsicKind::TryAdd, true, 16) => Some("llvm.sadd.with.overflow.i16"),
            (NumericIntrinsicKind::TryAdd, true, 32) => Some("llvm.sadd.with.overflow.i32"),
            (NumericIntrinsicKind::TryAdd, true, 64) => Some("llvm.sadd.with.overflow.i64"),
            (NumericIntrinsicKind::TryAdd, true, 128) => Some("llvm.sadd.with.overflow.i128"),
            (NumericIntrinsicKind::TryAdd, false, 8) => Some("llvm.uadd.with.overflow.i8"),
            (NumericIntrinsicKind::TryAdd, false, 16) => Some("llvm.uadd.with.overflow.i16"),
            (NumericIntrinsicKind::TryAdd, false, 32) => Some("llvm.uadd.with.overflow.i32"),
            (NumericIntrinsicKind::TryAdd, false, 64) => Some("llvm.uadd.with.overflow.i64"),
            (NumericIntrinsicKind::TryAdd, false, 128) => Some("llvm.uadd.with.overflow.i128"),
            (NumericIntrinsicKind::TrySub, true, 8) => Some("llvm.ssub.with.overflow.i8"),
            (NumericIntrinsicKind::TrySub, true, 16) => Some("llvm.ssub.with.overflow.i16"),
            (NumericIntrinsicKind::TrySub, true, 32) => Some("llvm.ssub.with.overflow.i32"),
            (NumericIntrinsicKind::TrySub, true, 64) => Some("llvm.ssub.with.overflow.i64"),
            (NumericIntrinsicKind::TrySub, true, 128) => Some("llvm.ssub.with.overflow.i128"),
            (NumericIntrinsicKind::TrySub, false, 8) => Some("llvm.usub.with.overflow.i8"),
            (NumericIntrinsicKind::TrySub, false, 16) => Some("llvm.usub.with.overflow.i16"),
            (NumericIntrinsicKind::TrySub, false, 32) => Some("llvm.usub.with.overflow.i32"),
            (NumericIntrinsicKind::TrySub, false, 64) => Some("llvm.usub.with.overflow.i64"),
            (NumericIntrinsicKind::TrySub, false, 128) => Some("llvm.usub.with.overflow.i128"),
            (NumericIntrinsicKind::TryMul, true, 8) => Some("llvm.smul.with.overflow.i8"),
            (NumericIntrinsicKind::TryMul, true, 16) => Some("llvm.smul.with.overflow.i16"),
            (NumericIntrinsicKind::TryMul, true, 32) => Some("llvm.smul.with.overflow.i32"),
            (NumericIntrinsicKind::TryMul, true, 64) => Some("llvm.smul.with.overflow.i64"),
            (NumericIntrinsicKind::TryMul, true, 128) => Some("llvm.smul.with.overflow.i128"),
            (NumericIntrinsicKind::TryMul, false, 8) => Some("llvm.umul.with.overflow.i8"),
            (NumericIntrinsicKind::TryMul, false, 16) => Some("llvm.umul.with.overflow.i16"),
            (NumericIntrinsicKind::TryMul, false, 32) => Some("llvm.umul.with.overflow.i32"),
            (NumericIntrinsicKind::TryMul, false, 64) => Some("llvm.umul.with.overflow.i64"),
            (NumericIntrinsicKind::TryMul, false, 128) => Some("llvm.umul.with.overflow.i128"),
            (NumericIntrinsicKind::TryNeg, true, 8) => Some("llvm.ssub.with.overflow.i8"),
            (NumericIntrinsicKind::TryNeg, true, 16) => Some("llvm.ssub.with.overflow.i16"),
            (NumericIntrinsicKind::TryNeg, true, 32) => Some("llvm.ssub.with.overflow.i32"),
            (NumericIntrinsicKind::TryNeg, true, 64) => Some("llvm.ssub.with.overflow.i64"),
            (NumericIntrinsicKind::TryNeg, true, 128) => Some("llvm.ssub.with.overflow.i128"),
            _ => None,
        }
    }

    fn coerce_int_value(
        &mut self,
        value: ValueRef,
        target_ty: &str,
        signed: bool,
    ) -> Result<ValueRef, Error> {
        if value.ty() == target_ty {
            return Ok(value);
        }
        let target_bits = target_ty
            .strip_prefix('i')
            .and_then(|bits| bits.parse::<u32>().ok())
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "numeric intrinsic target type `{target_ty}` is not an integer"
                ))
            })?;
        if value.ty() == "ptr" {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = ptrtoint ptr {} to {target_ty}",
                value.repr()
            )
            .ok();
            return Ok(ValueRef::new(tmp, target_ty));
        }
        let source_bits = value
            .ty()
            .strip_prefix('i')
            .and_then(|bits| bits.parse::<u32>().ok())
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "numeric intrinsic operand `{}` is not an integer type",
                    value.ty()
                ))
            })?;
        if source_bits == target_bits {
            return self.bitcast_value(&value, value.ty(), target_ty);
        }
        let tmp = self.new_temp();
        if source_bits > target_bits {
            writeln!(
                &mut self.builder,
                "  {tmp} = trunc {source_ty} {} to {target_ty}",
                value.repr(),
                source_ty = value.ty()
            )
            .ok();
        } else {
            let op = if signed { "sext" } else { "zext" };
            writeln!(
                &mut self.builder,
                "  {tmp} = {op} {source_ty} {} to {target_ty}",
                value.repr(),
                source_ty = value.ty()
            )
            .ok();
        }
        Ok(ValueRef::new(tmp, target_ty))
    }

    fn coerce_bool_value(&mut self, value: ValueRef, target_ty: &str) -> Result<ValueRef, Error> {
        if value.ty() == target_ty {
            return Ok(value);
        }
        let tmp = self.new_temp();
        if target_ty == "i1" {
            writeln!(
                &mut self.builder,
                "  {tmp} = trunc {source_ty} {} to i1",
                value.repr(),
                source_ty = value.ty()
            )
            .ok();
            Ok(ValueRef::new(tmp, "i1"))
        } else {
            writeln!(
                &mut self.builder,
                "  {tmp} = zext {source_ty} {} to {target_ty}",
                value.repr(),
                source_ty = value.ty()
            )
            .ok();
            Ok(ValueRef::new(tmp, target_ty))
        }
    }

    fn convert_to_i32(&mut self, value: ValueRef, signed: bool) -> Result<ValueRef, Error> {
        if value.ty() == "i32" {
            return Ok(value);
        }
        self.coerce_int_value(value, "i32", signed)
    }

    pub(super) fn emit_numeric_intrinsic_assign(
        &mut self,
        place: &Place,
        intrinsic: &crate::mir::NumericIntrinsic,
    ) -> Result<(), Error> {
        let int_ty = self.llvm_int_ty_for_width(intrinsic.width);
        let bits = self.width_bits(intrinsic.width);
        let signed = intrinsic.signed;

        let place_ty = if place.projection.is_empty() && self.is_reference_param(place.local.0) {
            Some(self.param_value_type(place.local.0)?)
        } else {
            self.place_type(place)?
        };

        let load_int_operand =
            |operand: &Operand, emitter: &mut FunctionEmitter<'_>| -> Result<ValueRef, Error> {
                let raw = emitter.emit_operand(operand, Some(int_ty))?;
                emitter.coerce_int_value(raw, int_ty, signed)
            };

        match intrinsic.kind {
            NumericIntrinsicKind::TryAdd
            | NumericIntrinsicKind::TrySub
            | NumericIntrinsicKind::TryMul
            | NumericIntrinsicKind::TryNeg => {
                let lhs = load_int_operand(&intrinsic.operands[0], self)?;
                let rhs = if intrinsic.kind == NumericIntrinsicKind::TryNeg {
                    let zero = ValueRef::new_literal("0".into(), int_ty);
                    zero
                } else {
                    load_int_operand(&intrinsic.operands[1], self)?
                };
                let overflow_name = self
                    .overflow_intrinsic_name(intrinsic.kind, signed, bits)
                    .ok_or_else(|| Error::Codegen("unsupported overflow intrinsic width".into()))?;
                self.externals.insert(overflow_name);
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {{ {int_ty}, i1 }} @{overflow_name}({int_ty} {}, {int_ty} {})",
                    lhs.repr(),
                    rhs.repr()
                )
                .ok();
                let value_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {value_tmp} = extractvalue {{ {int_ty}, i1 }} {tmp}, 0"
                )
                .ok();
                let ovf_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {ovf_tmp} = extractvalue {{ {int_ty}, i1 }} {tmp}, 1"
                )
                .ok();
                let success_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {success_tmp} = xor i1 {ovf_tmp}, true"
                )
                .ok();

                if let Some(out) = &intrinsic.out {
                    let value_ref = ValueRef::new(value_tmp.clone(), int_ty);
                    self.store_place(out, &value_ref)?;
                }

                let bool_ty = place_ty.unwrap_or_else(|| "i1".to_string());
                let bool_value =
                    self.coerce_bool_value(ValueRef::new(success_tmp, "i1"), &bool_ty)?;
                self.store_place(place, &bool_value)?;
                Ok(())
            }
            NumericIntrinsicKind::LeadingZeroCount
            | NumericIntrinsicKind::TrailingZeroCount
            | NumericIntrinsicKind::PopCount => {
                let operand = load_int_operand(&intrinsic.operands[0], self)?;
                let (intrin, needs_zero_flag) = match intrinsic.kind {
                    NumericIntrinsicKind::LeadingZeroCount => ("llvm.ctlz", true),
                    NumericIntrinsicKind::TrailingZeroCount => ("llvm.cttz", true),
                    NumericIntrinsicKind::PopCount => ("llvm.ctpop", false),
                    _ => unreachable!(),
                };
                let intrin_name = match (intrin, bits) {
                    ("llvm.ctlz", 8) => "llvm.ctlz.i8",
                    ("llvm.ctlz", 16) => "llvm.ctlz.i16",
                    ("llvm.ctlz", 32) => "llvm.ctlz.i32",
                    ("llvm.ctlz", 64) => "llvm.ctlz.i64",
                    ("llvm.ctlz", 128) => "llvm.ctlz.i128",
                    ("llvm.cttz", 8) => "llvm.cttz.i8",
                    ("llvm.cttz", 16) => "llvm.cttz.i16",
                    ("llvm.cttz", 32) => "llvm.cttz.i32",
                    ("llvm.cttz", 64) => "llvm.cttz.i64",
                    ("llvm.cttz", 128) => "llvm.cttz.i128",
                    ("llvm.ctpop", 8) => "llvm.ctpop.i8",
                    ("llvm.ctpop", 16) => "llvm.ctpop.i16",
                    ("llvm.ctpop", 32) => "llvm.ctpop.i32",
                    ("llvm.ctpop", 64) => "llvm.ctpop.i64",
                    ("llvm.ctpop", 128) => "llvm.ctpop.i128",
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported integer width for numeric intrinsic".into(),
                        ));
                    }
                };
                self.externals.insert(intrin_name);
                let tmp = self.new_temp();
                if needs_zero_flag {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = call {int_ty} @{intrin_name}({int_ty} {}, i1 false)",
                        operand.repr()
                    )
                    .ok();
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = call {int_ty} @{intrin_name}({int_ty} {})",
                        operand.repr()
                    )
                    .ok();
                }
                let result = ValueRef::new(tmp.clone(), int_ty);
                let as_i32 = self.convert_to_i32(result, false)?;
                let dest_ty = place_ty.unwrap_or_else(|| "i32".into());
                let final_value = if dest_ty == "i32" {
                    as_i32
                } else {
                    self.coerce_int_value(as_i32, &dest_ty, false)?
                };
                self.store_place(place, &final_value)?;
                Ok(())
            }
            NumericIntrinsicKind::RotateLeft | NumericIntrinsicKind::RotateRight => {
                let value = load_int_operand(&intrinsic.operands[0], self)?;
                let shift_raw = self.emit_operand(&intrinsic.operands[1], Some("i32"))?;
                let modulus = bits as i64;
                let shift_norm_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {shift_norm_tmp} = srem i32 {}, {modulus}",
                    shift_raw.repr()
                )
                .ok();
                let zero_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {zero_tmp} = icmp slt i32 {shift_norm_tmp}, 0"
                )
                .ok();
                let shift_fix_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {shift_fix_tmp} = add i32 {shift_norm_tmp}, {modulus}"
                )
                .ok();
                let shift_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {shift_tmp} = select i1 {zero_tmp}, i32 {shift_fix_tmp}, i32 {shift_norm_tmp}"
                )
                .ok();
                let shift_int =
                    self.coerce_int_value(ValueRef::new(shift_tmp.clone(), "i32"), int_ty, false)?;
                let width_const = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {width_const} = sub {int_ty} {bits}, {}",
                    shift_int.repr()
                )
                .ok();
                let (left_op, right_op) = if intrinsic.kind == NumericIntrinsicKind::RotateLeft {
                    ("shl", "lshr")
                } else {
                    ("lshr", "shl")
                };
                let left_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {left_tmp} = {left_op} {int_ty} {}, {}",
                    value.repr(),
                    shift_int.repr()
                )
                .ok();
                let right_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {right_tmp} = {right_op} {int_ty} {}, {width_const}",
                    value.repr()
                )
                .ok();
                let rotated_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {rotated_tmp} = or {int_ty} {left_tmp}, {right_tmp}"
                )
                .ok();
                let result = ValueRef::new(rotated_tmp, int_ty);
                self.store_place(place, &result)?;
                Ok(())
            }
            NumericIntrinsicKind::ReverseEndianness => {
                if bits == 8 {
                    let value = load_int_operand(&intrinsic.operands[0], self)?;
                    self.store_place(place, &value)?;
                    return Ok(());
                }
                let value = load_int_operand(&intrinsic.operands[0], self)?;
                let intrin_name = match bits {
                    16 => "llvm.bswap.i16",
                    32 => "llvm.bswap.i32",
                    64 => "llvm.bswap.i64",
                    _ => {
                        return Err(Error::Codegen(
                            "reverse-endianness requires 16/32/64-bit operands".into(),
                        ));
                    }
                };
                self.externals.insert(intrin_name);
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {int_ty} @{intrin_name}({int_ty} {})",
                    value.repr()
                )
                .ok();
                let result = ValueRef::new(tmp, int_ty);
                self.store_place(place, &result)?;
                Ok(())
            }
            NumericIntrinsicKind::IsPowerOfTwo => {
                let value = load_int_operand(&intrinsic.operands[0], self)?;
                let zero_cmp = self.new_temp();
                let cmp_op = if signed { "sgt" } else { "ne" };
                writeln!(
                    &mut self.builder,
                    "  {zero_cmp} = icmp {cmp_op} {int_ty} {}, 0",
                    value.repr()
                )
                .ok();
                let minus_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {minus_tmp} = sub {int_ty} {}, 1",
                    value.repr()
                )
                .ok();
                let and_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {and_tmp} = and {int_ty} {}, {minus_tmp}",
                    value.repr()
                )
                .ok();
                let pow_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {pow_tmp} = icmp eq {int_ty} {and_tmp}, 0"
                )
                .ok();
                let result_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {result_tmp} = and i1 {zero_cmp}, {pow_tmp}"
                )
                .ok();
                let bool_ty = place_ty.unwrap_or_else(|| "i1".into());
                let result = self.coerce_bool_value(ValueRef::new(result_tmp, "i1"), &bool_ty)?;
                self.store_place(place, &result)?;
                Ok(())
            }
        }
    }

    pub(super) fn emit_deinit_statement(&mut self, place: &Place) -> Result<(), Error> {
        let ty = self.mir_ty_of_place(place)?;
        let Some(symbol) = self.dispose_symbol_for_ty(&ty) else {
            return Ok(());
        };
        let canonical = resolve_function_name(self.signatures, symbol).ok_or_else(|| {
            Error::Codegen(format!(
                "unable to resolve function `{symbol}` in LLVM backend"
            ))
        })?;
        let signature = self.signatures.get(&canonical).ok_or_else(|| {
            Error::Codegen(format!("LLVM signature metadata missing for `{canonical}`"))
        })?;
        let ptr = self.place_ptr(place)?;
        let param_ty = signature
            .params
            .get(0)
            .cloned()
            .unwrap_or_else(|| "ptr".to_string());
        let argument = if param_ty == "ptr" {
            ptr.clone()
        } else if param_ty.contains('*') {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = bitcast ptr {ptr} to {param_ty}"
            )
            .ok();
            tmp
        } else {
            let tmp = self.new_temp();
            writeln!(&mut self.builder, "  {tmp} = load {param_ty}, ptr {ptr}").ok();
            tmp
        };
        let callee = format!("@{}", signature.symbol);
        if let Some(ret) = &signature.ret {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ret} {callee}({param_ty} {argument})"
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call void {callee}({param_ty} {argument})"
            )
            .ok();
        }
        Ok(())
    }

    pub(super) fn emit_drop_missing(&mut self, place: &Place) -> Result<(), Error> {
        let ptr = self.place_ptr(place)?;
        self.externals.insert("chic_rt_drop_missing");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_drop_missing(ptr {ptr})"
        )
        .ok();
        Ok(())
    }

    fn emit_zero_init_statement(&mut self, place: &Place) -> Result<(), Error> {
        let ty = self.mir_ty_of_place(place)?;
        let Some((size, align)) = self.type_layouts.size_and_align_for_ty(&ty) else {
            return Ok(());
        };
        if size == 0 {
            return Ok(());
        }
        let mut ptr = self.place_ptr(place)?;
        if self.is_reference_param(place.local.0) && place.projection.is_empty() {
            let loaded = self.new_temp();
            writeln!(&mut self.builder, "  {loaded} = load ptr, ptr {ptr}").ok();
            ptr = loaded;
        }
        self.emit_memset_call(&ptr, size as i64, align.max(1) as u32, Some(place))
    }

    fn emit_zero_init_raw_statement(
        &mut self,
        pointer: &Operand,
        length: &Operand,
    ) -> Result<(), Error> {
        if let Some(len) = Self::constant_length_from_operand(length) {
            if len == 0 {
                return Ok(());
            }
            let dest = self.emit_operand(pointer, Some("ptr"))?;
            return self.emit_memset_call(dest.repr(), len as i64, 1, None);
        }
        let dest = self.emit_operand(pointer, Some("ptr"))?;
        let len = self.emit_operand(length, Some("i64"))?;
        self.externals.insert("chic_rt_zero_init");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_zero_init(ptr {}, i64 {})",
            dest.repr(),
            len.repr()
        )
        .ok();
        Ok(())
    }

    fn emit_memset_call(
        &mut self,
        pointer: &str,
        size: i64,
        align: u32,
        place: Option<&Place>,
    ) -> Result<(), Error> {
        self.externals.insert("llvm.memset.p0.i64");
        let alias_suffix = place
            .and_then(|p| self.alias_suffix_for_place(p))
            .unwrap_or_default();
        let align = align.max(1);
        writeln!(
            &mut self.builder,
            "  call void @llvm.memset.p0.i64(ptr align {align} {pointer}, i8 0, i64 {size}, i1 false){alias_suffix}"
        )
        .ok();
        Ok(())
    }

    fn constant_length_from_operand(operand: &Operand) -> Option<u64> {
        if let Operand::Const(constant) = operand {
            match constant.value() {
                ConstValue::Int(value) | ConstValue::Int32(value) if *value >= 0 => {
                    (*value).try_into().ok()
                }
                ConstValue::UInt(value) => (*value).try_into().ok(),
                _ => None,
            }
        } else {
            None
        }
    }

    pub(super) fn drop_glue_symbol_for_ty(&self, ty: &Ty) -> Option<String> {
        match ty {
            Ty::Unknown | Ty::Unit => return None,
            Ty::String
            | Ty::Vec(_)
            | Ty::Span(_)
            | Ty::ReadOnlySpan(_)
            | Ty::Rc(_)
            | Ty::Arc(_) => return None,
            Ty::Tuple(_) => return None,
            _ => {}
        }

        let mut canonical = ty.canonical_name();
        if let Some(resolved) = self.type_layouts.resolve_type_key(&canonical) {
            canonical = resolved.to_string();
        }
        if matches!(ty, Ty::Named(_))
            && self
                .type_layouts
                .layout_for_name(&canonical)
                .is_some_and(|layout| {
                    matches!(layout, TypeLayout::Struct(_) | TypeLayout::Class(_))
                })
        {
            return None;
        }
        if self.type_layouts.ty_requires_drop(ty)
            || self.type_layouts.type_requires_drop(&canonical)
        {
            Some(drop_glue_symbol_for(&canonical))
        } else {
            None
        }
    }

    pub(super) fn emit_drop_glue_for(&mut self, place: &Place, ty: &Ty) -> Result<bool, Error> {
        let Some(symbol) = self.drop_glue_symbol_for_ty(ty) else {
            return Ok(false);
        };

        let canonical =
            resolve_function_name(self.signatures, &symbol).unwrap_or_else(|| symbol.clone());
        let signature = self.signatures.get(&canonical).ok_or_else(|| {
            Error::Codegen(format!(
                "LLVM signature metadata missing for `{}`",
                canonical
            ))
        })?;

        let ptr = self.place_ptr(place)?;
        let param_ty = signature
            .params
            .get(0)
            .cloned()
            .unwrap_or_else(|| "ptr".to_string());
        let glue_symbol = format!("@{}", signature.symbol);
        if param_ty == "ptr" {
            writeln!(&mut self.builder, "  call void {glue_symbol}(ptr {ptr})").ok();
        } else if param_ty.contains('*') {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = bitcast ptr {ptr} to {param_ty}"
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  call void {glue_symbol}({param_ty} {tmp})"
            )
            .ok();
        } else {
            let tmp = self.new_temp();
            writeln!(&mut self.builder, "  {tmp} = load {param_ty}, ptr {ptr}").ok();
            writeln!(
                &mut self.builder,
                "  call void {glue_symbol}({param_ty} {tmp})"
            )
            .ok();
        }
        Ok(true)
    }

    fn emit_inline_asm(&mut self, asm: &InlineAsm) -> Result<(), Error> {
        if !matches!(self.arch, TargetArch::X86_64 | TargetArch::Aarch64) {
            return Err(Error::Codegen(format!(
                "inline assembly is not supported for target `{}` in the LLVM backend",
                self.arch.as_str()
            )));
        }

        let mut constraints = Vec::new();
        let mut arg_values = Vec::new();
        let mut arg_types = Vec::new();
        let mut output_places = Vec::new();
        let mut output_types = Vec::new();

        for operand in &asm.operands {
            let base = self.inline_asm_reg_constraint(&operand.reg)?;
            match &operand.kind {
                InlineAsmOperandKind::In { value } => {
                    constraints.push(base);
                    let ty = self.operand_type(value)?.ok_or_else(|| {
                        Error::Codegen(
                            "inline assembly input operand type is unknown in LLVM backend".into(),
                        )
                    })?;
                    let value_ref = self.emit_operand(value, Some(&ty))?;
                    arg_values.push(value_ref);
                    arg_types.push(ty);
                }
                InlineAsmOperandKind::Out { place, late } => {
                    let mut constraint = format!("={}", base);
                    if !*late {
                        constraint.insert(0, '&');
                    }
                    constraints.push(constraint);
                    let ty = self.place_type(place)?.ok_or_else(|| {
                        Error::Codegen(
                            "inline assembly output place has unknown type in LLVM backend".into(),
                        )
                    })?;
                    output_places.push(place.clone());
                    output_types.push(ty);
                }
                InlineAsmOperandKind::InOut {
                    input,
                    output,
                    late,
                } => {
                    let mut constraint = format!("+{}", base);
                    if !*late {
                        constraint.insert(0, '&');
                    }
                    constraints.push(constraint);
                    let ty = match self.operand_type(input)? {
                        Some(ty) => Some(ty),
                        None => self.place_type(output)?,
                    }
                    .ok_or_else(|| {
                        Error::Codegen(
                            "inline assembly inout operand type is unknown in LLVM backend".into(),
                        )
                    })?;
                    let value_ref = self.emit_operand(input, Some(&ty))?;
                    arg_values.push(value_ref);
                    arg_types.push(ty.clone());
                    output_places.push(output.clone());
                    output_types.push(ty);
                }
                InlineAsmOperandKind::Const { value } => {
                    constraints.push("i".to_string());
                    let ty = self.operand_type(value)?.ok_or_else(|| {
                        Error::Codegen(
                            "inline assembly const operand type is unknown in LLVM backend".into(),
                        )
                    })?;
                    let value_ref = self.emit_operand(value, Some(&ty))?;
                    arg_values.push(value_ref);
                    arg_types.push(ty);
                }
                InlineAsmOperandKind::Sym { symbol } => {
                    constraints.push("s".to_string());
                    let const_operand =
                        Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.clone())));
                    let ty = "ptr".to_string();
                    let value_ref = self.emit_operand(&const_operand, Some(&ty))?;
                    arg_values.push(value_ref);
                    arg_types.push(ty);
                }
            }
        }

        let mut clobbers = Vec::new();
        if !(asm.options.nomem || asm.options.readonly || asm.options.pure) {
            clobbers.push("~{memory}".to_string());
        }
        if !asm.options.preserves_flags {
            match self.arch {
                TargetArch::X86_64 => {
                    clobbers.push("~{flags}".to_string());
                    clobbers.push("~{fpsr}".to_string());
                    clobbers.push("~{dirflag}".to_string());
                }
                TargetArch::Aarch64 => {
                    clobbers.push("~{nzcv}".to_string());
                }
            }
        }
        for reg in &asm.clobbers {
            let base = self.inline_asm_reg_constraint(reg)?;
            let clobber = if base.starts_with('{') {
                format!("~{base}")
            } else {
                format!("~{{{base}}}")
            };
            clobbers.push(clobber);
        }
        constraints.extend(clobbers);

        let template = self.render_inline_asm_template(&asm.template);
        let escaped_template = template.replace('\\', "\\\\").replace('"', "\\\"");
        let constraint_str = constraints.join(",");

        let ret_ty = match output_types.len() {
            0 => "void".to_string(),
            1 => output_types[0].clone(),
            _ => format!("{{ {} }}", output_types.join(", ")),
        };

        let args_repr = arg_types
            .iter()
            .zip(arg_values.iter())
            .map(|(ty, val)| format!("{ty} {}", val.repr()))
            .collect::<Vec<_>>()
            .join(", ");

        let sideeffect = if asm.options.volatile || !(asm.options.pure || asm.options.readonly) {
            " sideeffect"
        } else {
            ""
        };
        let alignstack = if asm.options.alignstack {
            " alignstack"
        } else {
            ""
        };
        let dialect = if asm.options.intel_syntax && self.arch == TargetArch::X86_64 {
            " inteldialect"
        } else {
            ""
        };

        let mut attrs = Vec::new();
        if asm.options.noreturn {
            attrs.push("noreturn");
        }
        if asm.options.nomem || asm.options.pure {
            attrs.push("readnone");
        } else if asm.options.readonly {
            attrs.push("readonly");
        }
        let attr_suffix = if attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", attrs.join(" "))
        };

        if ret_ty == "void" {
            writeln!(
                &mut self.builder,
                "  call void asm{sideeffect}{alignstack}{dialect} \"{escaped_template}\", \"{constraint_str}\"({args_repr}){attr_suffix}"
            )
            .ok();
            return Ok(());
        }

        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = call {ret_ty} asm{sideeffect}{alignstack}{dialect} \"{escaped_template}\", \"{constraint_str}\"({args_repr}){attr_suffix}"
        )
        .ok();

        if output_places.len() == 1 {
            let value_ref = ValueRef::new(tmp, &output_types[0]);
            self.store_place(&output_places[0], &value_ref)?;
            return Ok(());
        }

        for (index, (place, ty)) in output_places.iter().zip(output_types.iter()).enumerate() {
            let extract = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {extract} = extractvalue {ret_ty} {tmp}, {index}"
            )
            .ok();
            let value_ref = ValueRef::new(extract, ty);
            self.store_place(place, &value_ref)?;
        }
        Ok(())
    }

    fn render_inline_asm_template(&self, pieces: &[InlineAsmTemplatePiece]) -> String {
        let mut template = String::new();
        for piece in pieces {
            match piece {
                InlineAsmTemplatePiece::Literal(text) => template.push_str(text),
                InlineAsmTemplatePiece::Placeholder {
                    operand_idx,
                    modifier,
                    ..
                } => {
                    template.push('$');
                    template.push_str(&operand_idx.to_string());
                    if let Some(modifier) = modifier {
                        template.push(':');
                        template.push_str(modifier);
                    }
                }
            }
        }
        template
    }

    fn inline_asm_reg_constraint(&self, reg: &InlineAsmRegister) -> Result<String, Error> {
        match reg {
            InlineAsmRegister::Explicit(name) => Ok(format!("{{{}}}", name)),
            InlineAsmRegister::Class(class) => match (self.arch, class) {
                (
                    TargetArch::X86_64 | TargetArch::Aarch64,
                    InlineAsmRegisterClass::Reg
                    | InlineAsmRegisterClass::Reg8
                    | InlineAsmRegisterClass::Reg16
                    | InlineAsmRegisterClass::Reg32
                    | InlineAsmRegisterClass::Reg64,
                ) => Ok("r".into()),
                (
                    TargetArch::X86_64,
                    InlineAsmRegisterClass::Xmm
                    | InlineAsmRegisterClass::Ymm
                    | InlineAsmRegisterClass::Zmm,
                ) => Ok("x".into()),
                (TargetArch::X86_64 | TargetArch::Aarch64, InlineAsmRegisterClass::Vreg) => {
                    Ok("v".into())
                }
                (TargetArch::X86_64, InlineAsmRegisterClass::Kreg) => Ok("k".into()),
                (arch, class) => Err(Error::Codegen(format!(
                    "register class {:?} is not supported for inline assembly on {}",
                    class,
                    arch.as_str()
                ))),
            },
        }
    }

    pub(super) fn atomic_alignment_for_place(&self, place: &Place) -> Result<usize, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(self.atomic_alignment_for_ty(&ty))
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::CpuIsaTier;
    use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
    use crate::codegen::llvm::signatures::LlvmFunctionSignature;
    use crate::codegen::llvm::types::map_type_owned;
    use crate::mir::{
        Abi, AggregateKind, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
        LocalKind, MirBody, MirFunction, Operand, ParamMode, Place, Rvalue, Statement,
        StatementKind, TupleTy, Ty, TypeLayout, TypeLayoutTable,
    };
    use crate::mir::{AtomicFenceScope, AtomicOrdering};
    use crate::mir::{AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, TypeRepr};
    use crate::target::TargetArch;
    use std::collections::{BTreeSet, HashMap, HashSet};

    fn with_emitter<F, R>(
        local_tys: Vec<Ty>,
        ptrs: Vec<Option<&str>>,
        mut type_layouts: TypeLayoutTable,
        f: F,
    ) -> (R, String, BTreeSet<&'static str>)
    where
        F: FnOnce(&mut FunctionEmitter<'_>) -> R,
    {
        type_layouts.finalize_auto_traits();
        let mut body = MirBody::new(0, None);
        for ty in &local_tys {
            body.locals.push(LocalDecl::new(
                None,
                ty.clone(),
                false,
                None,
                LocalKind::Local,
            ));
        }
        let function = MirFunction {
            name: "demo".into(),
            kind: FunctionKind::Function,
            signature: FnSig::empty(),
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        let signatures: Box<HashMap<String, LlvmFunctionSignature>> = Box::new(HashMap::new());
        let mut externals: Box<BTreeSet<&'static str>> = Box::new(BTreeSet::new());
        let vtable_symbols: Box<HashSet<String>> = Box::new(HashSet::new());
        let trait_vtables: Box<Vec<_>> = Box::new(Vec::new());
        let class_vtables: Box<Vec<_>> = Box::new(Vec::new());
        let statics: Box<Vec<crate::mir::StaticVar>> = Box::new(Vec::new());
        let str_literals: Box<HashMap<crate::mir::StrId, StrLiteralInfo>> =
            Box::new(HashMap::new());
        let mut metadata = MetadataRegistry::new();
        let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
        let mut emitter = FunctionEmitter::new(
            Box::leak(Box::new(function)),
            &signatures,
            &mut externals,
            &vtable_symbols,
            &trait_vtables,
            &class_vtables,
            CpuIsaTier::Baseline,
            &[CpuIsaTier::Baseline],
            TargetArch::Aarch64,
            &target,
            &statics,
            &str_literals,
            Box::leak(Box::new(type_layouts)),
            &mut metadata,
            None,
        );
        let llvm_tys = emitter
            .function
            .body
            .locals
            .iter()
            .map(|decl| map_type_owned(&decl.ty, Some(emitter.type_layouts)).expect("map type"))
            .collect();
        emitter.set_local_types_for_tests(llvm_tys);
        emitter.local_ptrs = ptrs.into_iter().map(|p| p.map(str::to_string)).collect();
        let result = f(&mut emitter);
        let ir = emitter.ir().to_string();
        (result, ir, *externals)
    }

    #[test]
    fn aggregate_assignment_expands_tuple_fields() {
        let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
        let name = tuple_ty.canonical_name();
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            name.clone(),
            TypeLayout::Struct(StructLayout {
                name,
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![
                    FieldLayout {
                        name: "0".into(),
                        ty: Ty::named("int"),
                        index: 0,
                        offset: Some(0),
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: None,
                    },
                    FieldLayout {
                        name: "1".into(),
                        ty: Ty::named("int"),
                        index: 1,
                        offset: Some(4),
                        span: None,
                        mmio: None,
                        display_name: None,
                        is_required: false,
                        is_nullable: false,
                        is_readonly: false,
                        view_of: None,
                    },
                ],
                positional: Vec::new(),
                list: None,
                size: Some(8),
                align: Some(4),
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );
        let (result, ir, _) = with_emitter(
            vec![tuple_ty, Ty::named("int"), Ty::named("int")],
            vec![Some("%tuple"), Some("%lhs"), Some("%rhs")],
            layouts,
            |emitter| {
                let value = Rvalue::Aggregate {
                    kind: AggregateKind::Tuple,
                    fields: vec![
                        Operand::Copy(Place::new(LocalId(1))),
                        Operand::Copy(Place::new(LocalId(2))),
                    ],
                };
                let stmt = Statement {
                    span: None,
                    kind: StatementKind::Assign {
                        place: Place::new(LocalId(0)),
                        value,
                    },
                };
                emitter.emit_statement(&stmt)
            },
        );

        result.expect("aggregate assignment should lower");
        assert!(
            ir.contains("%tuple"),
            "tuple destination pointer should be used in stores"
        );
        assert!(
            ir.contains("store i32"),
            "tuple element assignments should lower to stores"
        );
    }

    #[test]
    fn aggregate_assignment_to_enum_variant_is_error() {
        let (result, _, _) = with_emitter(
            vec![Ty::Tuple(TupleTy::new(vec![Ty::named("int")]))],
            vec![Some("%tuple")],
            TypeLayoutTable::default(),
            |emitter| {
                let value = Rvalue::Aggregate {
                    kind: AggregateKind::Adt {
                        name: "Demo::Enum".into(),
                        variant: Some("Variant".into()),
                    },
                    fields: Vec::new(),
                };
                let stmt = Statement {
                    span: None,
                    kind: StatementKind::Assign {
                        place: Place::new(LocalId(0)),
                        value,
                    },
                };
                emitter.emit_statement(&stmt)
            },
        );
        let err = result.expect_err("enum aggregates should be rejected");
        assert!(
            err.to_string().contains("enum variants"),
            "unexpected error message: {err:?}"
        );
    }

    #[test]
    fn zero_init_loads_reference_param_and_memsets() {
        let mut layouts = TypeLayoutTable::default();
        layouts.finalize_auto_traits();
        let mut ref_param = LocalDecl::new(None, Ty::named("int"), false, None, LocalKind::Arg(0));
        ref_param.param_mode = Some(ParamMode::Ref);

        let mut body = MirBody::new(0, None);
        body.locals.push(ref_param);
        let function = Box::new(MirFunction {
            name: "demo".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![Ty::named("int")],
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),

                lends_to_return: None,

                variadic: false,
            },
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        });
        let mut metadata = MetadataRegistry::new();
        let signatures: HashMap<String, LlvmFunctionSignature> = HashMap::new();
        let mut externals: BTreeSet<&'static str> = BTreeSet::new();
        let vtable_symbols: HashSet<String> = HashSet::new();
        let trait_vtables = Vec::new();
        let class_vtables = Vec::new();
        let statics: Vec<crate::mir::StaticVar> = Vec::new();
        let str_literals: HashMap<crate::mir::StrId, StrLiteralInfo> = HashMap::new();
        let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
        let mut emitter = FunctionEmitter::new(
            Box::leak(function),
            &signatures,
            &mut externals,
            &vtable_symbols,
            &trait_vtables,
            &class_vtables,
            CpuIsaTier::Baseline,
            &[CpuIsaTier::Baseline],
            TargetArch::Aarch64,
            &target,
            &statics,
            &str_literals,
            Box::leak(Box::new(layouts)),
            &mut metadata,
            None,
        );
        emitter.local_ptrs = vec![Some("%arg".into())];
        emitter.local_tys = vec![Some("ptr".into())];

        let stmt = Statement {
            span: None,
            kind: StatementKind::ZeroInit {
                place: Place::new(LocalId(0)),
            },
        };
        emitter
            .emit_statement(&stmt)
            .expect("zero init should lower");
        let ir = emitter.ir().to_string();
        assert!(
            ir.contains("load ptr, ptr %arg"),
            "reference params should be loaded before memset"
        );
        assert!(
            ir.contains("llvm.memset.p0.i64"),
            "zero init should emit memset"
        );
    }

    #[test]
    fn zero_init_raw_calls_runtime_for_dynamic_length() {
        let (_, ir, _) = with_emitter(
            vec![
                Ty::Pointer(Box::new(crate::mir::PointerTy::new(Ty::named("int"), true))),
                Ty::named("int"),
            ],
            vec![Some("%ptr"), Some("%len")],
            TypeLayoutTable::default(),
            |emitter| {
                let stmt = Statement {
                    span: None,
                    kind: StatementKind::ZeroInitRaw {
                        pointer: Operand::Copy(Place::new(LocalId(0))),
                        length: Operand::Copy(Place::new(LocalId(1))),
                    },
                };
                emitter.emit_statement(&stmt)
            },
        );
        assert!(ir.contains("chic_rt_zero_init"));
    }

    #[test]
    fn zero_init_raw_constant_length_uses_memset() {
        let (_, ir, externals) = with_emitter(
            vec![Ty::Pointer(Box::new(crate::mir::PointerTy::new(
                Ty::named("int"),
                true,
            )))],
            vec![Some("%ptr")],
            TypeLayoutTable::default(),
            |emitter| {
                let stmt = Statement {
                    span: None,
                    kind: StatementKind::ZeroInitRaw {
                        pointer: Operand::Copy(Place::new(LocalId(0))),
                        length: Operand::Const(ConstOperand::new(ConstValue::UInt(16))),
                    },
                };
                emitter.emit_statement(&stmt)
            },
        );
        assert!(externals.contains("llvm.memset.p0.i64"));
        assert!(
            ir.contains("llvm.memset.p0.i64"),
            "constant-length zero init should emit memset directly"
        );
    }

    #[test]
    fn drop_dispatch_prefers_string_drop() {
        let (_, ir, _) = with_emitter(
            vec![Ty::String],
            vec![Some("%str")],
            TypeLayoutTable::default(),
            |emitter| {
                let stmt = Statement {
                    span: None,
                    kind: StatementKind::Drop {
                        place: Place::new(LocalId(0)),
                        target: crate::mir::BlockId(0),
                        unwind: None,
                    },
                };
                emitter.emit_statement(&stmt)
            },
        );
        assert!(ir.contains("chic_rt_string_drop"));
    }

    #[test]
    fn pending_statement_reports_error() {
        let (result, _, _) = with_emitter(
            vec![Ty::named("int")],
            vec![Some("%x")],
            TypeLayoutTable::default(),
            |emitter| {
                emitter.emit_statement(&Statement {
                    span: None,
                    kind: StatementKind::Pending(crate::mir::PendingStatement {
                        kind: crate::mir::PendingStatementKind::Expression,
                        detail: Some("pending".into()),
                    }),
                })
            },
        );
        assert!(result.is_err(), "pending statements should error");
    }

    #[test]
    fn zero_init_raw_zero_length_is_noop() {
        let (result, ir, externals) = with_emitter(
            vec![Ty::Pointer(Box::new(crate::mir::PointerTy::new(
                Ty::named("int"),
                true,
            )))],
            vec![Some("%ptr")],
            TypeLayoutTable::default(),
            |emitter| {
                emitter.emit_statement(&Statement {
                    span: None,
                    kind: StatementKind::ZeroInitRaw {
                        pointer: Operand::Copy(Place::new(LocalId(0))),
                        length: Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    },
                })
            },
        );
        result.expect("zero-length zero-init should succeed");
        assert!(ir.trim().is_empty(), "no IR should be emitted for len=0");
        assert!(
            externals.is_empty(),
            "no externals should be recorded for zero-length zero-init"
        );
    }

    #[test]
    fn atomic_store_and_fence_lower() {
        let (store_result, store_ir, _) = with_emitter(
            vec![Ty::named("int")],
            vec![Some("%ptr")],
            TypeLayoutTable::default(),
            |emitter| {
                emitter.emit_statement(&Statement {
                    span: None,
                    kind: StatementKind::AtomicStore {
                        target: Place::new(LocalId(0)),
                        value: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                        order: AtomicOrdering::Acquire,
                    },
                })
            },
        );
        store_result.expect("atomic store should lower");
        assert!(
            store_ir.contains("store atomic"),
            "atomic store should emit atomic store IR"
        );

        let (fence_result, fence_ir, _) =
            with_emitter(vec![], vec![], TypeLayoutTable::default(), |emitter| {
                emitter.emit_statement(&Statement {
                    span: None,
                    kind: StatementKind::AtomicFence {
                        order: AtomicOrdering::SeqCst,
                        scope: AtomicFenceScope::Full,
                    },
                })
            });
        fence_result.expect("atomic fence should lower");
        assert!(fence_ir.contains("fence seq_cst"));
    }

    #[test]
    fn deinit_statement_invokes_registered_function() {
        let ty = Ty::named("Demo::NeedsDeinit");
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            ty.canonical_name(),
            TypeLayout::Struct(StructLayout {
                name: ty.canonical_name(),
                repr: TypeRepr::Default,
                packing: None,
                fields: Vec::new(),
                positional: Vec::new(),
                list: None,
                size: Some(1),
                align: Some(1),
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: Some("Demo::NeedsDeinit::dispose".into()),
                class: None,
            }),
        );

        let mut signatures = HashMap::new();
        signatures.insert(
            "Demo::NeedsDeinit::dispose".into(),
            LlvmFunctionSignature {
                symbol: "Demo::NeedsDeinit::dispose".into(),
                ret: None,
                params: vec!["ptr".into()],
                param_attrs: vec![Vec::new()],
                dynamic: None,
                c_abi: None,
                variadic: false,
                weak: false,
            },
        );

        let mut externals = BTreeSet::new();
        let mut metadata = MetadataRegistry::new();
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            None,
            ty.clone(),
            false,
            None,
            LocalKind::Local,
        ));
        let vtable_symbols: HashSet<String> = HashSet::new();
        let trait_vtables = Vec::new();
        let class_vtables = Vec::new();
        let statics: Vec<crate::mir::StaticVar> = Vec::new();
        let str_literals: HashMap<crate::mir::StrId, StrLiteralInfo> = HashMap::new();
        let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
        let mut emitter = FunctionEmitter::new(
            Box::leak(Box::new(MirFunction {
                name: "Demo::caller".into(),
                kind: FunctionKind::Function,
                signature: FnSig::empty(),
                body,
                is_async: false,
                async_result: None,
                is_generator: false,
                span: None,
                optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
                extern_spec: None,
                is_weak: false,
                is_weak_import: false,
            })),
            &signatures,
            &mut externals,
            &vtable_symbols,
            &trait_vtables,
            &class_vtables,
            CpuIsaTier::Baseline,
            &[CpuIsaTier::Baseline],
            TargetArch::Aarch64,
            &target,
            &statics,
            &str_literals,
            Box::leak(Box::new(layouts)),
            &mut metadata,
            None,
        );
        emitter.local_ptrs = vec![Some("%obj".into())];
        emitter.local_tys = vec![Some("ptr".into())];

        emitter
            .emit_statement(&Statement {
                span: None,
                kind: StatementKind::Deinit(Place::new(LocalId(0))),
            })
            .expect("deinit should lower");
        assert!(
            emitter
                .ir()
                .contains("call void @Demo::NeedsDeinit::dispose"),
            "dispose call should be emitted with resolved signature"
        );
    }

    #[test]
    fn drop_missing_is_emitted_for_plain_types() {
        let (_, ir, externals) = with_emitter(
            vec![Ty::named("int")],
            vec![Some("%int")],
            TypeLayoutTable::default(),
            |emitter| {
                emitter.emit_statement(&Statement {
                    span: None,
                    kind: StatementKind::Drop {
                        place: Place::new(LocalId(0)),
                        target: crate::mir::BlockId(0),
                        unwind: None,
                    },
                })
            },
        );
        assert!(externals.contains("chic_rt_drop_missing"));
        assert!(ir.contains("chic_rt_drop_missing"));
    }
}
