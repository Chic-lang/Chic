use crate::codegen::wasm::{
    RuntimeHook, STACK_POINTER_GLOBAL_INDEX, ValueType, compute_aggregate_allocation, ensure_u32,
    local_requires_memory, map_type,
};
use crate::decimal::{DECIMAL_FLAG_VECTORIZE, Decimal128};
use crate::drop_glue::drop_type_identity;
use crate::error::Error;
use crate::mir::casts::{IntInfo, int_info, is_builtin_primitive};
use crate::mir::{
    AggregateKind, BorrowKind, CastKind, ConstOperand, ConstValue, DecimalIntrinsic,
    DecimalIntrinsicKind, FnTy, NumericIntrinsic, NumericIntrinsicKind, NumericWidth, Operand,
    Place, ProjectionElem, Rvalue, StructLayout, TupleTy, Ty, TypeLayout, UnOp,
    class_vtable_symbol_name,
};
use std::{convert::TryFrom, env};

use super::super::ops::{Op, emit_instruction};
use super::super::values::MemoryAccess;
use super::super::{FunctionEmitter, LocalRepresentation};
mod aggregate;
mod decimal;
mod fn_assignment;
mod int128;
mod numeric;
mod span;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
    ) -> Result<(), Error> {
        wasm_debug!("        emit_assign: place {} <- {:?}", place.local, value);
        match value {
            Rvalue::Use(Operand::Const(constant)) => {
                if let ConstValue::Decimal(decimal) = &constant.value {
                    if place.projection.is_empty() {
                        let local_index = place.local.0;
                        if let Some(meta) = self.borrow_destinations.get(&local_index)
                            && meta.kind != BorrowKind::Raw
                            && self.initialised_borrow_locals.contains(&local_index)
                        {
                            self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                            self.initialised_borrow_locals.remove(&local_index);
                        }
                    }
                    self.emit_decimal_constant_assign(buf, place, decimal)?;
                    return Ok(());
                }
            }
            Rvalue::DecimalIntrinsic(decimal) => {
                if place.projection.is_empty() {
                    let local_index = place.local.0;
                    if let Some(meta) = self.borrow_destinations.get(&local_index)
                        && meta.kind != BorrowKind::Raw
                        && self.initialised_borrow_locals.contains(&local_index)
                    {
                        self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                        self.initialised_borrow_locals.remove(&local_index);
                    }
                }
                self.emit_decimal_intrinsic_assign(buf, place, decimal)?;
                return Ok(());
            }
            Rvalue::NumericIntrinsic(numeric) => {
                self.emit_numeric_intrinsic_assign(buf, place, numeric)?;
                return Ok(());
            }
            _ => {}
        }
        if let Rvalue::SpanStackAlloc {
            element,
            length,
            source,
        } = value
        {
            self.emit_span_stack_alloc(buf, place, element, length, source.as_ref())?;
            return Ok(());
        }
        let dest_ty = self
            .local_tys
            .get(place.local.0)
            .cloned()
            .map(|ty| self.resolve_self_ty(&ty))
            .and_then(|ty| self.compute_projection_offset(&ty, &place.projection).ok())
            .map(|plan| plan.value_ty);
        if let Some(ref ty) = dest_ty {
            if let Some(signed) = self.int128_signed(ty) {
                self.emit_int128_assign(buf, place, value, signed)?;
                return Ok(());
            }
        }

        if matches!(
            self.representations[place.local.0],
            LocalRepresentation::Scalar
        ) {
            let base_ty = self.resolve_self_ty(&self.local_tys[place.local.0]);
            if matches!(base_ty, Ty::Str) && place.projection.len() == 1 {
                let field = &place.projection[0];
                let key = match field {
                    ProjectionElem::Field(0) => Some("ptr"),
                    ProjectionElem::Field(1) => Some("len"),
                    ProjectionElem::FieldNamed(name) => {
                        let lowered = name.to_ascii_lowercase();
                        if lowered == "ptr" || lowered == "pointer" {
                            Some("ptr")
                        } else if lowered == "len" || lowered == "length" {
                            Some("len")
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let (Some(key), Some(local_index)) = (key, self.local_index(place.local)) {
                    let new_value_ty = self.emit_rvalue(buf, value)?;
                    match new_value_ty {
                        ValueType::I32 => emit_instruction(buf, Op::LocalSet(self.temp_local)),
                        ValueType::I64 => {
                            emit_instruction(buf, Op::I32WrapI64);
                            emit_instruction(buf, Op::LocalSet(self.temp_local));
                        }
                        other => {
                            return Err(Error::Codegen(format!(
                                "assigning {other:?} into str.{key} is not supported by WASM lowering"
                            )));
                        }
                    }

                    emit_instruction(buf, Op::LocalGet(local_index));
                    emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                    match key {
                        "ptr" => {
                            emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                            emit_instruction(buf, Op::I64Const(32));
                            emit_instruction(buf, Op::I64ShrU);
                            emit_instruction(buf, Op::I32WrapI64);
                            emit_instruction(buf, Op::LocalSet(self.block_local)); // len

                            emit_instruction(buf, Op::LocalGet(self.block_local));
                            emit_instruction(buf, Op::I64ExtendI32U);
                            emit_instruction(buf, Op::I64Const(32));
                            emit_instruction(buf, Op::I64Shl);
                            emit_instruction(buf, Op::LocalGet(self.temp_local)); // ptr
                            emit_instruction(buf, Op::I64ExtendI32U);
                            emit_instruction(buf, Op::I64Or);
                            emit_instruction(buf, Op::LocalSet(local_index));
                        }
                        "len" => {
                            emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                            emit_instruction(buf, Op::I32WrapI64);
                            emit_instruction(buf, Op::LocalSet(self.block_local)); // ptr

                            emit_instruction(buf, Op::LocalGet(self.temp_local)); // len
                            emit_instruction(buf, Op::I64ExtendI32U);
                            emit_instruction(buf, Op::I64Const(32));
                            emit_instruction(buf, Op::I64Shl);
                            emit_instruction(buf, Op::LocalGet(self.block_local)); // ptr
                            emit_instruction(buf, Op::I64ExtendI32U);
                            emit_instruction(buf, Op::I64Or);
                            emit_instruction(buf, Op::LocalSet(local_index));
                        }
                        _ => {}
                    }
                    return Ok(());
                }
            }
        }

        if place
            .projection
            .iter()
            .any(|elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "_fn"))
        {
            wasm_debug!(
                "fn projection assignment: func={} dest_ty={:?} value={:?}",
                self.function.name,
                dest_ty.as_ref().map(|ty| ty.canonical_name()),
                value
            );
        }
        if let Some(ref ty) = dest_ty {
            if let Ty::Named(name) = ty {
                if name.as_str() == "Std::Platform::Thread::ThreadStart"
                    || name.as_str().ends_with("ThreadStart")
                {
                    let access = self.resolve_memory_access(place)?;
                    if self.emit_trait_object_assignment_to_access(buf, &access, ty, value)? {
                        return Ok(());
                    }
                }
            }
        }
        if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
            let access = self.resolve_memory_access(place);
            eprintln!(
                "[wasm-fn-assign-debug] func={} place_local={} proj={:?} dest_ty={} access={:?}",
                self.function.name,
                place.local.0,
                place.projection,
                dest_ty
                    .as_ref()
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into()),
                access.as_ref().map(|a| a.value_ty.canonical_name())
            );
        }
        if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some()
            && dest_ty
                .as_ref()
                .map_or(false, |ty| matches!(ty, Ty::Fn(_) | Ty::Named(_)))
        {
            if let Ok(access) = self.resolve_memory_access(place) {
                eprintln!(
                    "[wasm-fn-assign-debug] func={} place_local={} proj={:?} dest_ty={} access_ty={} offset={} load_slot={} repr={:?}",
                    self.function.name,
                    place.local.0,
                    place.projection,
                    dest_ty
                        .as_ref()
                        .map(|ty| ty.canonical_name())
                        .unwrap_or_else(|| "<unknown>".into()),
                    access.value_ty.canonical_name(),
                    access.offset,
                    access.load_pointer_from_slot,
                    self.representations[place.local.0],
                );
            }
        }
        let src_fn_layout = match value {
            Rvalue::Use(Operand::Copy(place) | Operand::Move(place)) => self
                .operand_ty(&Operand::Copy(place.clone()))
                .and_then(|ty| {
                    self.lookup_struct_layout(&ty)
                        .cloned()
                        .map(|layout| (ty, layout))
                }),
            Rvalue::Use(Operand::Borrow(borrow)) => self
                .operand_ty(&Operand::Borrow(borrow.clone()))
                .and_then(|ty| {
                    self.lookup_struct_layout(&ty)
                        .cloned()
                        .map(|layout| (ty, layout))
                }),
            _ => None,
        };
        if let Some((src_ty, src_layout)) = src_fn_layout {
            if Self::is_fn_pointer_layout(&src_layout) {
                let dest_access = self.resolve_memory_access(place)?;
                if self.emit_fn_assignment_to_access(
                    buf,
                    place,
                    &dest_access,
                    &src_layout,
                    &src_ty,
                    value,
                )? {
                    return Ok(());
                }
            }
        }
        if let Some(ref ty) = dest_ty {
            if let Some(layout) = self.lookup_struct_layout(ty).cloned() {
                if Self::is_fn_pointer_layout(&layout) {
                    let dest_access = self.resolve_memory_access(place)?;
                    if self.emit_fn_assignment_to_access(
                        buf,
                        place,
                        &dest_access,
                        &layout,
                        ty,
                        value,
                    )? {
                        return Ok(());
                    }
                }
            }
        }
        if let Some(dest_ty) = dest_ty.as_ref()
            && let Ty::Fn(fn_ty) = dest_ty
            && !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
        {
            if place.projection.is_empty() {
                let local_index = place.local.0;
                if let Some(meta) = self.borrow_destinations.get(&local_index)
                    && meta.kind != BorrowKind::Raw
                    && self.initialised_borrow_locals.contains(&local_index)
                {
                    self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                    self.initialised_borrow_locals.remove(&local_index);
                }
            }
            if self.emit_fn_assignment(buf, place, value, fn_ty)? {
                return Ok(());
            }
        }
        if let Some(trait_ty) = dest_ty
            .as_ref()
            .filter(|ty| self.ty_is_trait_object_like(ty))
        {
            let access = self.resolve_memory_access(place)?;
            if self.emit_trait_object_assignment_to_access(buf, &access, trait_ty, value)? {
                return Ok(());
            }
        }
        if let Rvalue::Aggregate { kind, fields } = value {
            self.emit_aggregate_assignment(buf, place, kind, fields)?;
            return Ok(());
        }
        if place.projection.is_empty() {
            let local_index = place.local.0;
            if let Some(meta) = self.borrow_destinations.get(&local_index)
                && meta.kind != BorrowKind::Raw
                && self.initialised_borrow_locals.contains(&local_index)
            {
                self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                self.initialised_borrow_locals.remove(&local_index);
            }
            let representation = self.representations.get(local_index).copied();
            let local_ty = self.local_tys.get(local_index).cloned();
            if env::var_os("CHIC_DEBUG_WASM_RETURN_ASSIGN").is_some()
                && local_index == 0
                && self.function.name.contains("MemoryTestHelpers::Alloc")
            {
                eprintln!(
                    "[wasm-return-assign] func={} place_local={} repr={:?} return_local={:?} wasm_slot={:?} value={:?} ty={}",
                    self.function.name,
                    local_index,
                    representation,
                    self.return_local,
                    self.local_index(place.local),
                    value,
                    local_ty
                        .as_ref()
                        .map(|ty| ty.canonical_name())
                        .unwrap_or_else(|| "<unknown>".into())
                );
            }
            if matches!(representation, Some(LocalRepresentation::PointerParam))
                && self.return_local == self.local_index(place.local)
            {
                if let Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) = value {
                    let dest_access = self.resolve_memory_access(place)?;
                    if local_requires_memory(&dest_access.value_ty, self.layouts) {
                        if env::var_os("CHIC_DEBUG_WASM_RETURN_ASSIGN").is_some()
                            && local_index == 0
                            && self.function.name.contains("MemoryTestHelpers::Alloc")
                        {
                            eprintln!(
                                "[wasm-return-assign] func={} emitting memmove to sret for {}",
                                self.function.name,
                                dest_access.value_ty.canonical_name()
                            );
                        }
                        let allocation = compute_aggregate_allocation(
                            &dest_access.value_ty,
                            self.layouts,
                        )
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "missing layout metadata for aggregate return `{}` in WASM backend",
                                dest_access.value_ty.canonical_name()
                            ))
                        })?;
                        let size_i32 = i32::try_from(allocation.size).map_err(|_| {
                            Error::Codegen(format!(
                                "aggregate return size {} exceeds wasm i32 range",
                                allocation.size
                            ))
                        })?;
                        let src_access = self.resolve_memory_access(src)?;
                        self.emit_pointer_expression(buf, &dest_access)?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::I32Const(size_i32));
                        let hook = self.runtime_hook_index(RuntimeHook::Memmove)?;
                        emit_instruction(buf, Op::Call(hook));
                        return Ok(());
                    }
                }
                if let Some(ty) = local_ty.as_ref() {
                    if let Ty::Named(named) = ty {
                        if !is_builtin_primitive(&self.layouts.primitive_registry, named.as_str())
                            && !self.ty_is_reference(ty)
                        {
                            if let Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) = value {
                                if let Some(layout) = self.lookup_struct_layout(ty).cloned() {
                                    self.copy_named_fields(buf, place, src, &layout)?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            if let Some(ty) = local_ty.as_ref() {
                if self.emit_named_aggregate_assignment(buf, place, value, ty)? {
                    return Ok(());
                }
            }
            let special_assignment = matches!(local_ty.as_ref(), Some(Ty::String))
                || matches!(local_ty.as_ref(), Some(Ty::Vec(_)))
                || matches!(local_ty.as_ref(), Some(Ty::Span(_)))
                || matches!(local_ty.as_ref(), Some(Ty::Rc(_)))
                || matches!(local_ty.as_ref(), Some(Ty::Arc(_)));
            let mut skip_pointer_store =
                matches!(local_ty.as_ref(), Some(Ty::Tuple(_))) || special_assignment;
            if let Some(ty) = local_ty.as_ref() {
                if matches!(ty, Ty::Named(_))
                    && !self.ty_is_reference(ty)
                    && !matches!(representation, Some(LocalRepresentation::PointerParam))
                {
                    skip_pointer_store = true;
                }
            }
            if matches!(representation, Some(LocalRepresentation::FrameAllocated))
                && !special_assignment
            {
                // Frame allocated locals (including async state slots) must be written through
                // their frame address instead of clobbering the pointer slot.
                skip_pointer_store = false;
            }
            if matches!(
                representation,
                Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
            ) && !skip_pointer_store
            {
                let value_ty = self.emit_rvalue(buf, value)?;
                self.store_value_into_place(buf, place, value_ty)?;
                if let Some(meta) = self.borrow_destinations.get(&local_index) {
                    if meta.kind != BorrowKind::Raw
                        && matches!(value, Rvalue::Use(Operand::Borrow(_)))
                    {
                        self.initialised_borrow_locals.insert(local_index);
                    }
                }
                return Ok(());
            }
            if let Some(tuple_ty) = local_ty.as_ref().and_then(|ty| match ty {
                Ty::Tuple(tuple) => Some(tuple.clone()),
                _ => None,
            }) {
                if self.emit_tuple_assignment(buf, place, value, &tuple_ty)? {
                    return Ok(());
                }
            }
            if let Some(local_ty) = local_ty.clone() {
                match &local_ty {
                    Ty::String => {
                        if self.emit_string_assignment(buf, place, value)? {
                            if let Some(meta) = self.borrow_destinations.get(&local_index) {
                                if meta.kind != BorrowKind::Raw
                                    && matches!(value, Rvalue::Use(Operand::Borrow(_)))
                                {
                                    self.initialised_borrow_locals.insert(local_index);
                                }
                            }
                            return Ok(());
                        }
                    }
                    Ty::Vec(_) | Ty::Span(_) => {
                        if self.emit_vec_assignment(buf, place, value)? {
                            return Ok(());
                        }
                    }
                    Ty::Rc(_) => {
                        if self.emit_rc_assignment(buf, place, value)? {
                            return Ok(());
                        }
                    }
                    Ty::Arc(_) => {
                        if self.emit_arc_assignment(buf, place, value)? {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
            let value_ty = self.emit_rvalue(buf, value)?;
            if let Some(index) = self.local_index(place.local) {
                if let Some(expected) = self.local_value_types.get(place.local.0).and_then(|ty| *ty)
                {
                    if expected != value_ty {
                        match (value_ty, expected) {
                            (ValueType::F64, ValueType::F32) => {
                                emit_instruction(buf, Op::F32DemoteF64);
                            }
                            (ValueType::F32, ValueType::F64) => {
                                emit_instruction(buf, Op::F64PromoteF32);
                            }
                            (ValueType::I64, ValueType::I32) => {
                                emit_instruction(buf, Op::I32WrapI64);
                            }
                            (ValueType::I32, ValueType::I64) => {
                                emit_instruction(buf, Op::I64ExtendI32S);
                            }
                            (ValueType::I32, ValueType::F32) => {
                                emit_instruction(buf, Op::F32ConvertI32S);
                            }
                            (ValueType::I32, ValueType::F64) => {
                                emit_instruction(buf, Op::F64ConvertI32S);
                            }
                            _ => {
                                let mir_ty = self
                                    .local_tys
                                    .get(place.local.0)
                                    .map(|ty| ty.canonical_name())
                                    .unwrap_or_else(|| "<unknown>".into());
                                if env::var_os("CHIC_DEBUG_WASM_ASSIGN").is_some() {
                                    eprintln!(
                                        "[wasm assign debug] func={} local={} mir_ty={} expected={:?} value_ty={:?} value={:?} locals={:?}",
                                        self.function.name,
                                        place.local.0,
                                        mir_ty,
                                        expected,
                                        value_ty,
                                        value,
                                        self.local_value_types
                                    );
                                }
                                return Err(Error::Codegen(format!(
                                    "assigning value of type {:?} to local {} expected {:?} in {} (MIR type `{mir_ty}`)",
                                    value_ty, place.local.0, expected, self.function.name
                                )));
                            }
                        }
                    }
                }
                emit_instruction(buf, Op::LocalSet(index));
            } else {
                emit_instruction(buf, Op::Drop);
            }
            if let Some(meta) = self.borrow_destinations.get(&local_index) {
                if meta.kind != BorrowKind::Raw && matches!(value, Rvalue::Use(Operand::Borrow(_)))
                {
                    self.initialised_borrow_locals.insert(local_index);
                }
            }
            return Ok(());
        }

        let access = self.resolve_memory_access(place)?;
        if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some()
            && access.value_ty.canonical_name().contains("fn(")
        {
            eprintln!(
                "[wasm-fn-assign-debug] func={} place_local={} proj={:?} access_ty={} layout={}",
                self.function.name,
                place.local.0,
                place.projection,
                access.value_ty.canonical_name(),
                self.lookup_struct_layout(&access.value_ty)
                    .map(|layout| layout.name.clone())
                    .unwrap_or_else(|| "<none>".into())
            );
        }
        if let Ty::Fn(fn_ty) = &access.value_ty {
            if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                // Extern function pointers are thin; treat them as scalars in memory.
            } else {
                let layout = self
                    .lookup_struct_layout(&access.value_ty)
                    .cloned()
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "missing function pointer layout for `{}` in WASM lowering",
                            access.value_ty.canonical_name()
                        ))
                    })?;
                if self.emit_fn_assignment_to_access(
                    buf,
                    place,
                    &access,
                    &layout,
                    &access.value_ty,
                    value,
                )? {
                    return Ok(());
                }
            }
        }
        if self.ty_is_trait_object_like(&access.value_ty) {
            let trait_ty = access.value_ty.clone();
            if self.emit_trait_object_assignment_to_access(buf, &access, &trait_ty, value)? {
                return Ok(());
            }
        }
        if let Some(layout) = self.lookup_struct_layout(&access.value_ty).cloned() {
            if Self::is_fn_pointer_layout(&layout)
                && self.emit_fn_assignment_to_access(
                    buf,
                    place,
                    &access,
                    &layout,
                    &access.value_ty,
                    value,
                )?
            {
                return Ok(());
            }
        }
        if local_requires_memory(&access.value_ty, self.layouts) {
            let value_ty = self.emit_rvalue(buf, value)?;
            self.store_value_into_place(buf, place, value_ty)?;
            return Ok(());
        }
        if access.load_pointer_from_slot && place.projection.is_empty() {
            emit_instruction(buf, Op::LocalGet(access.pointer_local));
            if access.offset != 0 {
                emit_instruction(buf, Op::I32Const(access.offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            let value_ty = self.emit_rvalue(buf, value)?;
            self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
            return Ok(());
        }
        self.emit_pointer_expression(buf, &access)?;
        let value_ty = self.emit_rvalue(buf, value)?;
        self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
        Ok(())
    }

    pub(super) fn emit_len_rvalue(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<ValueType, Error> {
        if env::var_os("CHIC_DEBUG_WASM_LEN").is_some() {
            eprintln!(
                "[wasm-len] func={} local={} proj={:?} ty={}",
                self.function.name,
                place.local.0,
                place.projection,
                self.local_tys
                    .get(place.local.0)
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into())
            );
        }
        let base_ty = self
            .local_tys
            .get(place.local.0)
            .cloned()
            .ok_or_else(|| Error::Codegen("length operand references unknown local".into()))?;
        let mut seq_ty = self
            .compute_projection_offset(&base_ty, &place.projection)?
            .value_ty;
        while let Ty::Nullable(inner) = seq_ty {
            seq_ty = *inner;
        }

        if matches!(seq_ty, Ty::Unknown) {
            // Missing layout information; return zero length to allow codegen to continue.
            emit_instruction(buf, Op::I32Const(0));
            return Ok(ValueType::I32);
        }

        let field_name = match seq_ty {
            Ty::Vec(_)
            | Ty::Array(_)
            | Ty::Span(_)
            | Ty::ReadOnlySpan(_)
            | Ty::String
            | Ty::Str => "len",
            _ => {
                return Err(Error::Codegen(
                    "length operator is only supported on sequence types in the WASM backend"
                        .into(),
                ));
            }
        };

        let mut len_place = place.clone();
        len_place
            .projection
            .push(ProjectionElem::FieldNamed(field_name.to_string()));
        let access = self.resolve_memory_access(&len_place)?;
        self.emit_pointer_expression(buf, &access)?;
        let value_ty = map_type(&access.value_ty);
        let op = match value_ty {
            ValueType::I32 => Op::I32Load(0),
            ValueType::I64 => Op::I64Load(0),
            other => {
                return Err(Error::Codegen(format!(
                    "sequence length expects an integer representation, found {other:?}"
                )));
            }
        };
        emit_instruction(buf, op);
        Ok(value_ty)
    }

    fn copy_tuple_fields(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
        tuple_ty: &TupleTy,
    ) -> Result<(), Error> {
        for index in 0..tuple_ty.elements.len() {
            let mut dest_field = dest.clone();
            dest_field
                .projection
                .push(ProjectionElem::Field(index as u32));
            let mut src_field = src.clone();
            src_field
                .projection
                .push(ProjectionElem::Field(index as u32));
            let operand = Operand::Copy(src_field);
            let rvalue = Rvalue::Use(operand);
            self.emit_assign(buf, &dest_field, &rvalue)?;
        }
        Ok(())
    }
}
