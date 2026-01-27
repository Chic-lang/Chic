use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::{LLVM_STR_TYPE, LLVM_STRING_TYPE};
use crate::codegen::llvm::types::{
    constrained_rounding_string, infer_const_type, is_float_ty, map_type_owned, parse_vector_type,
};
use crate::error::Error;
use crate::mir::{BinOp, Operand, ProjectionElem, Ty, UnOp};

use super::super::builder::FunctionEmitter;
use super::value_ref::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_unary(
        &mut self,
        op: UnOp,
        operand: &Operand,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        let ty = expected.ok_or_else(|| {
            Error::Codegen("unary operation requires expected output type".into())
        })?;
        let mut operand_ty = ty.to_string();
        let mut value = self.emit_operand(operand, Some(ty))?;
        let mut wrap_result = None;
        if let Some(elem_ty) = single_field_struct_element_ty(&operand_ty) {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = extractvalue {operand_ty} {}, 0",
                value.repr()
            )
            .ok();
            value = ValueRef::new(tmp, &elem_ty);
            operand_ty = elem_ty;
            if !matches!(op, UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut) {
                wrap_result = Some(ty.to_string());
            }
        }
        let tmp = self.new_temp();
        if operand_ty == "ptr" && matches!(op, UnOp::Neg) {
            let int_ty = if self.pointer_width_bits() == 64 {
                "i64"
            } else {
                "i32"
            };
            let cast_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cast_tmp} = ptrtoint ptr {} to {int_ty}",
                value.repr()
            )
            .ok();
            let neg_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {neg_tmp} = sub {int_ty} 0, {cast_tmp}"
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  {tmp} = inttoptr {int_ty} {neg_tmp} to ptr"
            )
            .ok();
            return Ok(ValueRef::new(tmp, "ptr"));
        }
        match op {
            UnOp::Neg => {
                if is_float_ty(&operand_ty) {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = fsub {operand_ty} -0.0, {}",
                        value.repr()
                    )
                    .ok();
                } else if value.ty() == "ptr" {
                    let int_ty = if self.pointer_width_bits() == 64 {
                        "i64"
                    } else {
                        "i32"
                    };
                    let cast_tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {cast_tmp} = ptrtoint ptr {} to {int_ty}",
                        value.repr()
                    )
                    .ok();
                    let neg_tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {neg_tmp} = sub {int_ty} 0, {cast_tmp}"
                    )
                    .ok();
                    if operand_ty == "ptr" {
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = inttoptr {int_ty} {neg_tmp} to ptr"
                        )
                        .ok();
                    } else {
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = trunc {int_ty} {neg_tmp} to {operand_ty}"
                        )
                        .ok();
                    }
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = sub {operand_ty} 0, {}",
                        value.repr()
                    )
                    .ok();
                }
            }
            UnOp::UnaryPlus => {
                return Ok(value);
            }
            UnOp::Not => {
                if operand_ty == "i8" {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = xor {operand_ty} {}, 1",
                        value.repr()
                    )
                    .ok();
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = xor {operand_ty} {}, -1",
                        value.repr()
                    )
                    .ok();
                }
            }
            UnOp::BitNot => {
                writeln!(
                    &mut self.builder,
                    "  {tmp} = xor {operand_ty} {}, -1",
                    value.repr()
                )
                .ok();
            }
            UnOp::Increment => {
                if operand_ty == "ptr" {
                    return Err(Error::Codegen(
                        "increment operator is not supported for pointers".into(),
                    ));
                }
                if is_float_ty(&operand_ty) {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = fadd {operand_ty} {}, 1.0",
                        value.repr()
                    )
                    .ok();
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = add {operand_ty} {}, 1",
                        value.repr()
                    )
                    .ok();
                }
            }
            UnOp::Decrement => {
                if operand_ty == "ptr" {
                    return Err(Error::Codegen(
                        "decrement operator is not supported for pointers".into(),
                    ));
                }
                if is_float_ty(&operand_ty) {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = fsub {operand_ty} {}, 1.0",
                        value.repr()
                    )
                    .ok();
                } else {
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = sub {operand_ty} {}, 1",
                        value.repr()
                    )
                    .ok();
                }
            }
            UnOp::Deref => {
                let ptr = self.emit_operand(operand, Some("ptr"))?;
                writeln!(
                    &mut self.builder,
                    "  {tmp} = load {operand_ty}, ptr {}",
                    ptr.repr()
                )
                .ok();
            }
            UnOp::AddrOf | UnOp::AddrOfMut => {
                return Err(Error::Codegen(
                    "address-of expressions should be lowered via Rvalue::AddressOf".into(),
                ));
            }
        }
        let mut result = ValueRef::new(tmp, &operand_ty);
        if let Some(struct_ty) = wrap_result {
            if result.ty() != struct_ty {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = insertvalue {struct_ty} undef, {} {}, 0",
                    result.ty(),
                    result.repr()
                )
                .ok();
                result = ValueRef::new(tmp, &struct_ty);
            }
        }
        Ok(result)
    }

    pub(crate) fn emit_binary(
        &mut self,
        op: BinOp,
        left: &Operand,
        right: &Operand,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        let expected_ty = expected.ok_or_else(|| {
            Error::Codegen("binary operation requires expected output type".into())
        })?;
        let lhs_ty = self.operand_type(left)?;
        let rhs_ty = self.operand_type(right)?;
        if let Some(value) = self.try_emit_pointer_arithmetic(
            op,
            left,
            right,
            expected_ty,
            lhs_ty.as_deref(),
            rhs_ty.as_deref(),
        )? {
            return Ok(value);
        }
        // When generic type parameters are lowered as opaque pointers, arithmetic
        // on them would otherwise produce invalid LLVM IR (`add ptr`). Treat those
        // placeholders as integers of pointer width so generic numeric helpers can
        // still be emitted in the bootstrap pipeline.
        let mut operand_ty_override = None;
        if expected_ty == "ptr"
            && matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem
            )
            && (lhs_ty.as_deref() == Some("ptr") || rhs_ty.as_deref() == Some("ptr"))
        {
            operand_ty_override = Some(if self.pointer_width_bits() == 64 {
                "i64".to_string()
            } else {
                "i32".to_string()
            });
        }
        let is_compare = matches!(
            op,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        );
        let is_null_const = |operand: &Operand| {
            matches!(
                operand,
                Operand::Const(constant)
                    if matches!(constant.value, crate::mir::ConstValue::Null)
            )
        };
        let mut operand_ty = if is_compare {
            if let Some(ty) = self.comparison_operand_ty(left)? {
                ty
            } else if let Some(ty) = self.comparison_operand_ty(right)? {
                ty
            } else if let Some(ty) = self.operand_type(left)? {
                ty
            } else if let Some(ty) = self.operand_type(right)? {
                ty
            } else {
                expected_ty.to_string()
            }
        } else {
            expected_ty.to_string()
        };
        if let Some(override_ty) = operand_ty_override.clone() {
            operand_ty = override_ty;
        }
        if is_compare && (operand_ty.starts_with('{') || operand_ty.starts_with('[')) {
            let mut alt_ty = self.operand_type(left)?;
            if alt_ty.is_none() {
                alt_ty = self.operand_type(right)?;
            }
            if let Some(alt) = alt_ty {
                if !(alt.starts_with('{') || alt.starts_with('[')) {
                    operand_ty = alt;
                } else if (is_null_const(left) || is_null_const(right))
                    && struct_field_types(&operand_ty).is_none()
                {
                    operand_ty = "ptr".to_string();
                }
            } else if (is_null_const(left) || is_null_const(right))
                && struct_field_types(&operand_ty).is_none()
            {
                operand_ty = "ptr".to_string();
            }
        }
        let operand_storage_ty = operand_ty.clone();
        let mut lhs = self.emit_operand(left, Some(&operand_storage_ty))?;
        let mut rhs = self.emit_operand(right, Some(&operand_storage_ty))?;
        if !is_compare && operand_ty == "ptr" && expected_ty.starts_with('i') {
            let target = expected_ty.to_string();
            if lhs.ty() == "ptr" {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = ptrtoint ptr {} to {target}",
                    lhs.repr()
                )
                .ok();
                lhs = ValueRef::new(tmp, &target);
            }
            if rhs.ty() == "ptr" {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = ptrtoint ptr {} to {target}",
                    rhs.repr()
                )
                .ok();
                rhs = ValueRef::new(tmp, &target);
            }
            operand_ty = target;
        }
        if is_compare {
            if matches!(op, BinOp::Eq | BinOp::Ne)
                && (operand_storage_ty == LLVM_STRING_TYPE || operand_storage_ty == LLVM_STR_TYPE)
            {
                return Ok(self.emit_string_like_compare(
                    op,
                    &lhs,
                    &rhs,
                    &operand_storage_ty,
                    expected_ty,
                ));
            }
            if let Some(fields) = struct_field_types(&operand_storage_ty) {
                if fields.len() > 1 {
                    return Ok(self.emit_struct_compare(&operand_storage_ty, &fields, &lhs, &rhs));
                }
            }
        }
        let mut wrap_result = None;
        if let Some(elem_ty) = single_field_struct_element_ty(&operand_storage_ty) {
            let lhs_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {lhs_tmp} = extractvalue {operand_storage_ty} {}, 0",
                lhs.repr()
            )
            .ok();
            lhs = ValueRef::new(lhs_tmp, &elem_ty);
            let rhs_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {rhs_tmp} = extractvalue {operand_storage_ty} {}, 0",
                rhs.repr()
            )
            .ok();
            rhs = ValueRef::new(rhs_tmp, &elem_ty);
            operand_ty = elem_ty;
            if !is_compare && expected_ty.trim_start().starts_with('{') {
                wrap_result = Some(operand_storage_ty);
            }
        }
        let ctx = BinaryContext::new(&operand_ty, &lhs, &rhs);

        let mut value = match op {
            BinOp::Add => self.emit_numeric(&ctx, "add", "fadd"),
            BinOp::Sub => self.emit_numeric(&ctx, "sub", "fsub"),
            BinOp::Mul => self.emit_numeric(&ctx, "mul", "fmul"),
            BinOp::Div => self.emit_numeric(&ctx, "sdiv", "fdiv"),
            BinOp::Rem => self.emit_numeric(&ctx, "srem", "frem"),
            BinOp::BitAnd | BinOp::And => self.emit_bitwise(&ctx, "and"),
            BinOp::BitOr | BinOp::Or => self.emit_bitwise(&ctx, "or"),
            BinOp::BitXor => self.emit_bitwise(&ctx, "xor"),
            BinOp::Shl => self.emit_shift(&ctx, "shl"),
            BinOp::Shr => self.emit_shift(&ctx, "ashr"),
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.emit_compare_value(&ctx, op, expected_ty)
            }
            BinOp::NullCoalesce => {
                return Err(Error::Codegen(
                    "null-coalescing operator should be lowered before LLVM emission".into(),
                ));
            }
        };
        if expected_ty == "ptr" && value.ty() != "ptr" {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = inttoptr {} {} to ptr",
                value.ty(),
                value.repr()
            )
            .ok();
            value = ValueRef::new(tmp, "ptr");
        }
        if let Some(struct_ty) = wrap_result {
            if value.ty() != struct_ty {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = insertvalue {struct_ty} undef, {} {}, 0",
                    value.ty(),
                    value.repr()
                )
                .ok();
                value = ValueRef::new(tmp, &struct_ty);
            }
        }

        Ok(value)
    }

    fn emit_string_like_compare(
        &mut self,
        op: BinOp,
        lhs: &ValueRef,
        rhs: &ValueRef,
        operand_storage_ty: &str,
        result_ty: &str,
    ) -> ValueRef {
        self.externals.insert("memcmp");

        let (lhs_ptr, lhs_len) = self.emit_string_like_ptr_len(lhs, operand_storage_ty);
        let (rhs_ptr, rhs_len) = self.emit_string_like_ptr_len(rhs, operand_storage_ty);

        let len_eq_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {len_eq_tmp} = icmp eq i64 {}, {}",
            lhs_len.repr(),
            rhs_len.repr()
        )
        .ok();

        let len_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {len_tmp} = select i1 {len_eq_tmp}, i64 {}, i64 0",
            lhs_len.repr()
        )
        .ok();

        let memcmp_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {memcmp_tmp} = call i32 @memcmp(ptr {}, ptr {}, i64 {len_tmp})",
            lhs_ptr.repr(),
            rhs_ptr.repr()
        )
        .ok();

        let memcmp_eq_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {memcmp_eq_tmp} = icmp eq i32 {memcmp_tmp}, 0"
        )
        .ok();

        let eq_i1_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {eq_i1_tmp} = and i1 {len_eq_tmp}, {memcmp_eq_tmp}"
        )
        .ok();

        let mut cmp_i1 = ValueRef::new(eq_i1_tmp, "i1");
        if matches!(op, BinOp::Ne) {
            let ne_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {ne_tmp} = xor i1 {}, true",
                cmp_i1.repr()
            )
            .ok();
            cmp_i1 = ValueRef::new(ne_tmp, "i1");
        }

        let desired_ty = {
            let trimmed = result_ty.trim();
            if trimmed.is_empty() {
                "i8".to_string()
            } else {
                trimmed.to_string()
            }
        };
        if desired_ty == "i1" {
            return cmp_i1;
        }
        let out_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {out_tmp} = zext i1 {} to {desired_ty}",
            cmp_i1.repr()
        )
        .ok();
        ValueRef::new(out_tmp, &desired_ty)
    }

    fn emit_string_like_ptr_len(&mut self, value: &ValueRef, ty: &str) -> (ValueRef, ValueRef) {
        let ptr_field = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ptr_field} = extractvalue {ty} {}, 0",
            value.repr()
        )
        .ok();
        let len_field = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {len_field} = extractvalue {ty} {}, 1",
            value.repr()
        )
        .ok();

        let len_val = ValueRef::new(len_field, "i64");

        if ty == LLVM_STRING_TYPE {
            let cap_field = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {cap_field} = extractvalue {ty} {}, 2",
                value.repr()
            )
            .ok();

            let slot = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {slot} = alloca {LLVM_STRING_TYPE}, align 8"
            )
            .ok();
            writeln!(
                &mut self.builder,
                "  store {LLVM_STRING_TYPE} {}, ptr {slot}, align 8",
                value.repr()
            )
            .ok();

            let inline_ptr = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {inline_ptr} = getelementptr inbounds {LLVM_STRING_TYPE}, ptr {slot}, i32 0, i32 3, i32 0, i32 0"
            )
            .ok();

            let tag_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tag_tmp} = and i64 {cap_field}, -9223372036854775808"
            )
            .ok();
            let is_inline = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {is_inline} = icmp ne i64 {tag_tmp}, 0"
            )
            .ok();

            let heap_ptr = ValueRef::new(ptr_field, "ptr");
            let data_ptr = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {data_ptr} = select i1 {is_inline}, ptr {inline_ptr}, ptr {}",
                heap_ptr.repr()
            )
            .ok();
            return (ValueRef::new(data_ptr, "ptr"), len_val);
        }

        (ValueRef::new(ptr_field, "ptr"), len_val)
    }

    pub(crate) fn emit_numeric(
        &mut self,
        ctx: &BinaryContext<'_>,
        int_op: &str,
        float_op: &str,
    ) -> ValueRef {
        let op = if ctx.is_float { float_op } else { int_op };
        let tmp = self.new_temp();
        let ty = ctx.ty;
        let trimmed = ty.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if std::env::var("CHIC_DEBUG_NUMERIC_OPS").is_ok() {
                eprintln!(
                    "[chic-debug] skipping numeric op `{op}` for non-scalar type {ty}, returning lhs"
                );
            }
            return ValueRef::new(ctx.lhs.repr().to_string(), ty);
        }
        if ctx.is_float {
            let rounding = constrained_rounding_string(self.rounding_mode());
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ty} @llvm.experimental.constrained.{op}.{ty}({ty} {}, {ty} {}, metadata !\"{rounding}\", metadata !\"fpexcept.strict\")",
                ctx.lhs.repr(),
                ctx.rhs.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  {tmp} = {op} {ty} {}, {}",
                ctx.lhs.repr(),
                ctx.rhs.repr()
            )
            .ok();
        }
        ValueRef::new(tmp, ty)
    }

    pub(crate) fn emit_bitwise(&mut self, ctx: &BinaryContext<'_>, op: &str) -> ValueRef {
        let tmp = self.new_temp();
        let ty = ctx.ty;
        let trimmed = ty.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if std::env::var("CHIC_DEBUG_NUMERIC_OPS").is_ok() {
                eprintln!(
                    "[chic-debug] skipping bitwise op `{op}` for non-scalar type {ty}, returning lhs"
                );
            }
            return ValueRef::new(ctx.lhs.repr().to_string(), ty);
        }
        writeln!(
            &mut self.builder,
            "  {tmp} = {op} {ty} {}, {}",
            ctx.lhs.repr(),
            ctx.rhs.repr()
        )
        .ok();
        ValueRef::new(tmp, ty)
    }

    pub(crate) fn emit_shift(&mut self, ctx: &BinaryContext<'_>, op: &str) -> ValueRef {
        let tmp = self.new_temp();
        let ty = ctx.ty;
        let trimmed = ty.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if std::env::var("CHIC_DEBUG_NUMERIC_OPS").is_ok() {
                eprintln!(
                    "[chic-debug] skipping shift op `{op}` for non-scalar type {ty}, returning lhs"
                );
            }
            return ValueRef::new(ctx.lhs.repr().to_string(), ty);
        }
        writeln!(
            &mut self.builder,
            "  {tmp} = {op} {ty} {}, {}",
            ctx.lhs.repr(),
            ctx.rhs.repr()
        )
        .ok();
        ValueRef::new(tmp, ty)
    }

    pub(crate) fn emit_compare_value(
        &mut self,
        ctx: &BinaryContext<'_>,
        op: BinOp,
        result_ty: &str,
    ) -> ValueRef {
        let ty = ctx.ty.trim();
        if let Some((lanes, _elem)) = parse_vector_type(ty) {
            let cmp_tmp = self.new_temp();
            let cmp_ty = format!("<{lanes} x i1>");
            if ctx.is_float {
                let predicate = match op {
                    BinOp::Eq => "oeq",
                    BinOp::Ne => "one",
                    BinOp::Lt => "olt",
                    BinOp::Le => "ole",
                    BinOp::Gt => "ogt",
                    BinOp::Ge => "oge",
                    _ => unreachable!(),
                };
                writeln!(
                    &mut self.builder,
                    "  {cmp_tmp} = fcmp {predicate} {ty} {}, {}",
                    ctx.lhs.repr(),
                    ctx.rhs.repr()
                )
                .ok();
            } else {
                let predicate = match op {
                    BinOp::Eq => "eq",
                    BinOp::Ne => "ne",
                    BinOp::Lt => "slt",
                    BinOp::Le => "sle",
                    BinOp::Gt => "sgt",
                    BinOp::Ge => "sge",
                    _ => unreachable!(),
                };
                writeln!(
                    &mut self.builder,
                    "  {cmp_tmp} = icmp {predicate} {ty} {}, {}",
                    ctx.lhs.repr(),
                    ctx.rhs.repr()
                )
                .ok();
            }
            let desired_ty = {
                let trimmed = result_ty.trim();
                if trimmed.is_empty() {
                    format!("<{lanes} x i8>")
                } else {
                    trimmed.to_string()
                }
            };
            if desired_ty == cmp_ty {
                return ValueRef::new(cmp_tmp, &cmp_ty);
            }
            let result_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {result_tmp} = zext {cmp_ty} {cmp_tmp} to {desired_ty}"
            )
            .ok();
            return ValueRef::new(result_tmp, &desired_ty);
        }

        let cmp_tmp = self.new_temp();
        if ctx.is_float {
            let predicate = match op {
                BinOp::Eq => "oeq",
                BinOp::Ne => "one",
                BinOp::Lt => "olt",
                BinOp::Le => "ole",
                BinOp::Gt => "ogt",
                BinOp::Ge => "oge",
                _ => unreachable!(),
            };
            writeln!(
                &mut self.builder,
                "  {cmp_tmp} = fcmp {predicate} {ty} {}, {}",
                ctx.lhs.repr(),
                ctx.rhs.repr()
            )
            .ok();
        } else {
            let predicate = match op {
                BinOp::Eq => "eq",
                BinOp::Ne => "ne",
                BinOp::Lt => "slt",
                BinOp::Le => "sle",
                BinOp::Gt => "sgt",
                BinOp::Ge => "sge",
                _ => unreachable!(),
            };
            writeln!(
                &mut self.builder,
                "  {cmp_tmp} = icmp {predicate} {ty} {}, {}",
                ctx.lhs.repr(),
                ctx.rhs.repr()
            )
            .ok();
        }

        let desired_ty = {
            let trimmed = result_ty.trim();
            if trimmed.is_empty() {
                "i8".to_string()
            } else {
                trimmed.to_string()
            }
        };
        if desired_ty == "i1" {
            return ValueRef::new(cmp_tmp, "i1");
        }
        let result_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result_tmp} = zext i1 {cmp_tmp} to {desired_ty}"
        )
        .ok();
        ValueRef::new(result_tmp, &desired_ty)
    }

    fn emit_struct_compare(
        &mut self,
        struct_ty: &str,
        fields: &[String],
        lhs: &ValueRef,
        rhs: &ValueRef,
    ) -> ValueRef {
        let cmp_i1 = self.emit_struct_compare_i1(struct_ty, fields, lhs, rhs);
        let result_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result_tmp} = zext i1 {} to i8",
            cmp_i1.repr()
        )
        .ok();
        ValueRef::new(result_tmp, "i8")
    }

    fn emit_struct_compare_i1(
        &mut self,
        struct_ty: &str,
        fields: &[String],
        lhs: &ValueRef,
        rhs: &ValueRef,
    ) -> ValueRef {
        let mut accum: Option<ValueRef> = None;
        for (idx, field_ty) in fields.iter().enumerate() {
            let lhs_field_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {lhs_field_tmp} = extractvalue {struct_ty} {}, {idx}",
                lhs.repr()
            )
            .ok();
            let rhs_field_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {rhs_field_tmp} = extractvalue {struct_ty} {}, {idx}",
                rhs.repr()
            )
            .ok();
            let cmp_val = self.emit_type_compare_to_i1(
                field_ty,
                &ValueRef::new(lhs_field_tmp, field_ty),
                &ValueRef::new(rhs_field_tmp, field_ty),
            );
            accum = if let Some(prev) = accum {
                let and_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {and_tmp} = and i1 {}, {}",
                    prev.repr(),
                    cmp_val.repr()
                )
                .ok();
                Some(ValueRef::new(and_tmp, "i1"))
            } else {
                Some(cmp_val)
            };
        }
        accum.unwrap_or_else(|| ValueRef::new_literal("true".into(), "i1"))
    }

    fn emit_array_compare_i1(
        &mut self,
        len: usize,
        elem_ty: &str,
        lhs: &ValueRef,
        rhs: &ValueRef,
        array_ty: &str,
    ) -> ValueRef {
        let mut accum: Option<ValueRef> = None;
        for idx in 0..len {
            let lhs_elem_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {lhs_elem_tmp} = extractvalue {array_ty} {}, {idx}",
                lhs.repr()
            )
            .ok();
            let rhs_elem_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {rhs_elem_tmp} = extractvalue {array_ty} {}, {idx}",
                rhs.repr()
            )
            .ok();
            let cmp_val = self.emit_type_compare_to_i1(
                elem_ty,
                &ValueRef::new(lhs_elem_tmp, elem_ty),
                &ValueRef::new(rhs_elem_tmp, elem_ty),
            );
            accum = if let Some(prev) = accum {
                let and_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {and_tmp} = and i1 {}, {}",
                    prev.repr(),
                    cmp_val.repr()
                )
                .ok();
                Some(ValueRef::new(and_tmp, "i1"))
            } else {
                Some(cmp_val)
            };
        }
        accum.unwrap_or_else(|| ValueRef::new_literal("true".into(), "i1"))
    }

    fn emit_type_compare_to_i1(&mut self, ty: &str, lhs: &ValueRef, rhs: &ValueRef) -> ValueRef {
        let trimmed = ty.trim();
        if let Some(fields) = struct_field_types(trimmed) {
            return self.emit_struct_compare_i1(trimmed, &fields, lhs, rhs);
        }
        if let Some((len, elem_ty)) = parse_array_type(trimmed) {
            return self.emit_array_compare_i1(len, &elem_ty, lhs, rhs, trimmed);
        }
        let cmp_tmp = self.new_temp();
        if is_float_ty(trimmed) {
            writeln!(
                &mut self.builder,
                "  {cmp_tmp} = fcmp oeq {trimmed} {}, {}",
                lhs.repr(),
                rhs.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  {cmp_tmp} = icmp eq {trimmed} {}, {}",
                lhs.repr(),
                rhs.repr()
            )
            .ok();
        }
        ValueRef::new(cmp_tmp, "i1")
    }

    fn try_emit_pointer_arithmetic(
        &mut self,
        op: BinOp,
        left: &Operand,
        right: &Operand,
        expected_ty: &str,
        lhs_ty: Option<&str>,
        rhs_ty: Option<&str>,
    ) -> Result<Option<ValueRef>, Error> {
        let is_ptr = |ty: Option<&str>| ty == Some("ptr");
        let is_int = |ty: Option<&str>| {
            ty.map(|t| t.starts_with('i') && t[1..].chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        };

        if matches!(op, BinOp::Add | BinOp::Sub) && is_ptr(lhs_ty) && is_int(rhs_ty) {
            let base = self.emit_operand(left, Some("ptr"))?;
            let offset_ty = rhs_ty.unwrap_or("i64");
            let offset = self.emit_operand(right, Some(offset_ty))?;
            return Ok(Some(self.emit_gep_offset(op, base, offset, offset_ty)?));
        }

        if matches!(op, BinOp::Add) && is_int(lhs_ty) && is_ptr(rhs_ty) {
            let base = self.emit_operand(right, Some("ptr"))?;
            let offset_ty = lhs_ty.unwrap_or("i64");
            let offset = self.emit_operand(left, Some(offset_ty))?;
            return Ok(Some(self.emit_gep_offset(op, base, offset, offset_ty)?));
        }

        if matches!(op, BinOp::Sub) && is_ptr(lhs_ty) && is_ptr(rhs_ty) {
            let lhs_val = self.emit_operand(left, Some("ptr"))?;
            let rhs_val = self.emit_operand(right, Some("ptr"))?;
            let lhs_int = self.ptr_to_int(&lhs_val)?;
            let rhs_int = self.ptr_to_int(&rhs_val)?;
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = sub i64 {}, {}",
                lhs_int.repr(),
                rhs_int.repr()
            )
            .ok();
            return Ok(Some(ValueRef::new(tmp, expected_ty)));
        }

        Ok(None)
    }

    fn ptr_to_int(&mut self, ptr: &ValueRef) -> Result<ValueRef, Error> {
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = ptrtoint ptr {} to i64",
            ptr.repr()
        )
        .ok();
        Ok(ValueRef::new(tmp, "i64"))
    }

    fn emit_gep_offset(
        &mut self,
        op: BinOp,
        base: ValueRef,
        offset: ValueRef,
        offset_ty: &str,
    ) -> Result<ValueRef, Error> {
        let offset_i64 = if offset_ty == "i64" {
            offset
        } else {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = sext {offset_ty} {} to i64",
                offset.repr()
            )
            .ok();
            ValueRef::new(tmp, "i64")
        };
        let adjusted = if matches!(op, BinOp::Sub) {
            let neg = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {neg} = sub i64 0, {}",
                offset_i64.repr()
            )
            .ok();
            ValueRef::new(neg, "i64")
        } else {
            offset_i64
        };
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = getelementptr i8, ptr {}, i64 {}",
            base.repr(),
            adjusted.repr()
        )
        .ok();
        Ok(ValueRef::new(tmp, "ptr"))
    }

    fn comparison_operand_ty(&self, operand: &Operand) -> Result<Option<String>, Error> {
        match operand {
            Operand::Const(constant) => {
                infer_const_type(&constant.value, constant.literal.as_ref())
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if let Some(field_name) =
                    place.projection.iter().rev().find_map(|elem| match elem {
                        ProjectionElem::FieldNamed(name) => Some(name.as_str()),
                        _ => None,
                    })
                {
                    match field_name {
                        "Value" => return Ok(Some("i128".into())),
                        "Status" => {
                            if let Some(ty) = map_type_owned(
                                &Ty::named("Std::Numeric::Decimal::DecimalStatus"),
                                Some(self.type_layouts),
                            )? {
                                return Ok(Some(ty));
                            }
                            return Ok(Some("i32".into()));
                        }
                        "Variant" => {
                            if let Some(ty) = map_type_owned(
                                &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicVariant"),
                                Some(self.type_layouts),
                            )? {
                                return Ok(Some(ty));
                            }
                        }
                        _ => {}
                    }
                } else if place
                    .projection
                    .iter()
                    .any(|elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "Value"))
                {
                    // Nested decimal value accesses (e.g. runtime call structs)
                    return Ok(Some("i128".into()));
                }
                let ty = self.mir_ty_of_place(place)?;
                if matches!(ty, Ty::Unknown) && std::env::var("CHIC_DEBUG_TYPES").is_ok() {
                    let (local_name, local_ty) = self
                        .function
                        .body
                        .local(place.local)
                        .map(|decl| {
                            (
                                decl.name
                                    .clone()
                                    .unwrap_or_else(|| format!("{:?}", place.local)),
                                decl.ty.canonical_name(),
                            )
                        })
                        .unwrap_or_else(|| (format!("{:?}", place.local), "<missing>".into()));
                    eprintln!(
                        "[chic-debug] comparison operand has unknown type in {} local `{local_name}` (ty={local_ty}): {place:?}",
                        self.function.name
                    );
                }
                if matches!(ty, Ty::Unknown) {
                    // Stubbed stdlib modules sometimes leave locals untyped; treat them as `int`
                    // so comparisons can still lower to LLVM.
                    return Ok(Some("i32".into()));
                }
                if ty.canonical_name().eq_ignore_ascii_case("decimal") {
                    return Ok(Some("i128".into()));
                }
                if let Some(mapped) = map_type_owned(&ty, Some(self.type_layouts))? {
                    return Ok(Some(mapped));
                }
                Ok(None)
            }
            Operand::Borrow(borrow) => {
                self.comparison_operand_ty(&Operand::Copy(borrow.place.clone()))
            }
            _ => Ok(None),
        }
    }
}

pub(crate) struct BinaryContext<'a> {
    ty: &'a str,
    lhs: &'a ValueRef,
    rhs: &'a ValueRef,
    is_float: bool,
}

impl<'a> BinaryContext<'a> {
    pub(super) fn new(ty: &'a str, lhs: &'a ValueRef, rhs: &'a ValueRef) -> Self {
        Self {
            ty,
            lhs,
            rhs,
            is_float: is_float_ty(ty),
        }
    }
}

fn single_field_struct_element_ty(ty: &str) -> Option<String> {
    let trimmed = ty.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() || inner.contains(',') {
        return None;
    }
    Some(inner.to_string())
}

fn parse_array_type(ty: &str) -> Option<(usize, String)> {
    let trimmed = ty.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    let mut parts = inner.splitn(2, 'x');
    let len = parts.next()?.trim().parse::<usize>().ok()?;
    let elem_ty = parts.next()?.trim().to_string();
    Some((len, elem_ty))
}

fn struct_field_types(ty: &str) -> Option<Vec<String>> {
    let trimmed = ty.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut fields = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                fields.push(inner[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = inner[start..].trim();
    if !tail.is_empty() {
        fields.push(tail.to_string());
    }
    Some(fields)
}
