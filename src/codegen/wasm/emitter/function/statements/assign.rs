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

    fn emit_fn_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        fn_ty: &FnTy,
    ) -> Result<bool, Error> {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            return Ok(false);
        }
        let fn_ty = Ty::Fn(fn_ty.clone());
        let layout = self
            .lookup_struct_layout(&fn_ty)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing function pointer layout for `{}` in WASM backend",
                    fn_ty.canonical_name()
                ))
            })?
            .clone();
        let dest_access = self.resolve_memory_access(place)?;
        self.emit_fn_assignment_to_access(buf, place, &dest_access, &layout, &fn_ty, value)
    }

    fn emit_fn_assignment_to_access(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        access: &MemoryAccess,
        layout: &StructLayout,
        fn_ty: &Ty,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        self.emit_pointer_expression(buf, access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        if let Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) = value {
            let operand_ty = self.operand_ty(&Operand::Copy(src.clone()));
            let src_layout = operand_ty
                .as_ref()
                .and_then(|ty| self.lookup_struct_layout(ty))
                .cloned();
            let src_is_fn_like = Self::is_fn_pointer_layout(layout)
                || src_layout.as_ref().is_some_and(Self::is_fn_pointer_layout);
            let force_pointer_copy = self
                .function
                .name
                .contains("ThreadFunctionStartAdapter::init");
            if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                eprintln!(
                    "[wasm-fn-debug] func={} operand_ty={:?} repr={:?} src_layout={}",
                    self.function.name,
                    operand_ty.as_ref().map(|ty| ty.canonical_name()),
                    self.representations.get(src.local.0),
                    src_layout
                        .as_ref()
                        .map(|layout| layout.name.as_str())
                        .unwrap_or("<none>")
                );
            }
            if src_is_fn_like || force_pointer_copy {
                let src_repr = self.representations.get(src.local.0);
                if matches!(
                    src_repr,
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    if let Ok(src_access) = self.resolve_memory_access(src) {
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                        return Ok(true);
                    }
                    let pointer_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer address")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                if force_pointer_copy {
                    if let Ok(src_access) = self.resolve_memory_access(src) {
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                        return Ok(true);
                    }
                    let pointer_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer address")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                let invoke_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                Self::ensure_operand_type(invoke_ty, ValueType::I32, "fn invoke value")?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.initialise_fn_from_invoke(buf, layout, fn_ty, self.block_local)?;
                if self
                    .function
                    .name
                    .contains("ThreadFunctionStartAdapter::init")
                {
                    let context_field = layout
                        .fields
                        .iter()
                        .find_map(|field| {
                            Self::fn_field_key(&field.name)
                                .filter(|key| *key == "context")
                                .and_then(|_| field.offset)
                        })
                        .and_then(|off| {
                            ensure_u32(off, "fn context offset exceeds wasm range").ok()
                        })
                        .unwrap_or(8);
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    if context_field != 0 {
                        emit_instruction(buf, Op::I32Const(context_field as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::LocalGet(access.pointer_local));
                    if access.load_pointer_from_slot {
                        emit_instruction(buf, Op::I32Load(0));
                    }
                    emit_instruction(buf, Op::I32Store(0));
                }
                return Ok(true);
            }
        }
        if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
            eprintln!(
                "[wasm-fn-assign] func={} place_local={} proj={:?} value={:?}",
                self.function.name, place.local.0, place.projection, value
            );
        }

        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                let src_repr = self.representations.get(src.local.0);
                if let Ok(src_access) = self.resolve_memory_access(src) {
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                if matches!(src_repr, Some(LocalRepresentation::Scalar)) {
                    let invoke_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                    Self::ensure_operand_type(invoke_ty, ValueType::I32, "fn invoke value")?;
                    emit_instruction(buf, Op::LocalSet(self.block_local));
                    self.initialise_fn_from_invoke(buf, layout, fn_ty, self.block_local)?;
                    if self
                        .function
                        .name
                        .contains("ThreadFunctionStartAdapter::init")
                    {
                        let context_field = layout
                            .fields
                            .iter()
                            .find_map(|field| {
                                Self::fn_field_key(&field.name)
                                    .filter(|key| *key == "context")
                                    .and_then(|_| field.offset)
                            })
                            .and_then(|off| {
                                ensure_u32(off, "fn context offset exceeds wasm range").ok()
                            })
                            .unwrap_or(8);
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        if context_field != 0 {
                            emit_instruction(buf, Op::I32Const(context_field as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::LocalGet(access.pointer_local));
                        if access.load_pointer_from_slot {
                            emit_instruction(buf, Op::I32Load(0));
                        }
                        emit_instruction(buf, Op::I32Store(0));
                    }
                    return Ok(true);
                }
                if let Ok(src_access) = self.resolve_memory_access(src) {
                    if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                        eprintln!(
                            "[wasm-fn-copy] func={} dest_proj={:?} src_local={} src_ty={} layout_fields={} repr={:?}",
                            self.function.name,
                            place.projection,
                            src.local.0,
                            src_access.value_ty.canonical_name(),
                            layout.name,
                            self.representations.get(src.local.0),
                        );
                    }
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                if matches!(
                    self.representations.get(src.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    if env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                        eprintln!(
                            "[wasm-fn-copy-indirect] func={} dest_proj={:?} src_local={} layout_fields={}",
                            self.function.name, place.projection, src.local.0, layout.name,
                        );
                    }
                    let pointer_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer address")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                let pointer_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer address")?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                Ok(true)
            }
            Rvalue::Use(Operand::Borrow(borrow)) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                Ok(true)
            }
            Rvalue::Use(Operand::Const(constant)) => {
                self.initialise_fn_from_const(buf, layout, fn_ty, constant)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn emit_trait_object_assignment_to_access(
        &mut self,
        buf: &mut Vec<u8>,
        access: &MemoryAccess,
        trait_ty: &Ty,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        let lookup_vtable_offset = |emitter: &Self, impl_ty: &Ty| {
            let trait_name = trait_ty.canonical_name();
            let impl_name = impl_ty.canonical_name();
            emitter
                .trait_vtables
                .iter()
                .find(|table| {
                    Self::names_equivalent(&table.trait_name, &trait_name)
                        && Self::names_equivalent(&table.impl_type, &impl_name)
                })
                .and_then(|table| emitter.trait_vtable_offsets.get(&table.symbol))
                .copied()
        };
        match value {
            Rvalue::Cast { operand, .. } => {
                let lowered = Rvalue::Use(operand.clone());
                return self
                    .emit_trait_object_assignment_to_access(buf, access, trait_ty, &lowered);
            }
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                let src_access = self.resolve_memory_access(src)?;
                let src_repr = self.representations.get(src.local.0).copied();
                let impl_ty = self.operand_ty(&Operand::Copy(src.clone()));
                let force_threadstart_adapter = trait_ty.canonical_name().contains("ThreadStart")
                    && (self.function.name.contains("ThreadFunctionRunner::init")
                        || src_access
                            .value_ty
                            .canonical_name()
                            .contains("ThreadFunctionStartAdapter"));
                if force_threadstart_adapter {
                    let (context_off, vtable_off) = self
                        .lookup_struct_layout(trait_ty)
                        .and_then(|layout| {
                            let ctx = layout
                                .fields
                                .iter()
                                .find_map(|field| field.offset.filter(|off| *off == 0));
                            let vt = layout
                                .fields
                                .iter()
                                .find_map(|field| field.offset.filter(|off| *off != 0));
                            Some((ctx.unwrap_or(0) as u32, vt.unwrap_or(4) as u32))
                        })
                        .unwrap_or((0, 4));
                    let vtable_offset = impl_ty
                        .as_ref()
                        .and_then(|ty| lookup_vtable_offset(self, ty))
                        .or_else(|| lookup_vtable_offset(self, &src_access.value_ty))
                        .or_else(|| {
                            let class_symbol =
                                class_vtable_symbol_name(&src_access.value_ty.canonical_name());
                            let class_off =
                                self.class_vtable_offsets.get(&class_symbol).copied()?;
                            self.trait_vtables
                                .iter()
                                .find(|table| {
                                    Self::names_equivalent(
                                        &table.trait_name,
                                        &trait_ty.canonical_name(),
                                    ) && self
                                        .class_vtable_offsets
                                        .get(&class_vtable_symbol_name(&table.impl_type))
                                        .is_some_and(|off| *off == class_off)
                                })
                                .and_then(|table| self.trait_vtable_offsets.get(&table.symbol))
                                .copied()
                        })
                        .or_else(|| {
                            self.trait_vtables
                                .iter()
                                .find(|table| {
                                    Self::names_equivalent(
                                        &table.trait_name,
                                        &trait_ty.canonical_name(),
                                    ) && table.impl_type.contains("ThreadFunctionStartAdapter")
                                })
                                .and_then(|table| self.trait_vtable_offsets.get(&table.symbol))
                                .copied()
                        });
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    let context_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                    Self::ensure_operand_type(
                        context_ty,
                        ValueType::I32,
                        "ThreadStart context pointer",
                    )?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    if context_off != 0 {
                        emit_instruction(buf, Op::I32Const(context_off as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::I32Store(0));
                    if let Some(offset) = vtable_offset {
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        if vtable_off != 0 {
                            emit_instruction(buf, Op::I32Const(vtable_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Const(offset as i32));
                        emit_instruction(buf, Op::I32Store(0));
                        if let Some(run_index) = self.lookup_function_index(
                            "Std::Platform::Thread::ThreadFunctionStartAdapter::Run",
                        ) {
                            emit_instruction(buf, Op::I32Const(i32::try_from(offset).unwrap_or(0)));
                            emit_instruction(
                                buf,
                                Op::I32Const(i32::try_from(run_index).unwrap_or(0)),
                            );
                            emit_instruction(buf, Op::I32Store(0));
                        }
                    }
                    return Ok(true);
                }
                let vtable_offset = impl_ty
                    .as_ref()
                    .and_then(|ty| lookup_vtable_offset(self, ty))
                    .or_else(|| lookup_vtable_offset(self, &src_access.value_ty));
                if vtable_offset.is_none() && trait_ty.canonical_name().contains("ThreadStart") {
                    let remap: Vec<(u32, u32)> = self
                        .trait_vtables
                        .iter()
                        .filter(|table| {
                            Self::names_equivalent(&table.trait_name, &trait_ty.canonical_name())
                        })
                        .filter_map(|table| {
                            let trait_off = *self.trait_vtable_offsets.get(&table.symbol)?;
                            let class_symbol = class_vtable_symbol_name(&table.impl_type);
                            let class_off =
                                self.class_vtable_offsets.get(&class_symbol).copied()?;
                            Some((class_off, trait_off))
                        })
                        .collect();
                    if !remap.is_empty() {
                        let (context_off, vtable_off) = self
                            .lookup_struct_layout(trait_ty)
                            .and_then(|layout| {
                                let ctx = layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off == 0));
                                let vt = layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off != 0));
                                Some((ctx.unwrap_or(0) as u32, vt.unwrap_or(4) as u32))
                            })
                            .unwrap_or((0, 4));
                        let mut src_is_trait_object =
                            self.ty_is_trait_object_like(&src_access.value_ty);
                        if matches!(src_repr, Some(LocalRepresentation::Scalar)) {
                            src_is_trait_object = false;
                        }
                        let src_class_vtable_offset = self
                            .lookup_struct_layout(&src_access.value_ty)
                            .and_then(|layout| {
                                layout.fields.iter().find_map(|field| {
                                    if field.name.contains("vtable") {
                                        field.offset
                                    } else if field.offset == Some(0) {
                                        Some(0)
                                    } else {
                                        None
                                    }
                                })
                            });
                        self.emit_pointer_expression(buf, access)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

                        let use_trait_layout = src_is_trait_object
                            && matches!(src_repr, Some(LocalRepresentation::PointerParam));
                        if use_trait_layout || src_class_vtable_offset.is_none() {
                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            if context_off != 0 {
                                emit_instruction(buf, Op::I32Const(context_off as i32));
                                emit_instruction(buf, Op::I32Add);
                            }
                            emit_instruction(buf, Op::I32Load(0));
                            emit_instruction(buf, Op::LocalSet(self.temp_local));

                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            if vtable_off != 0 {
                                emit_instruction(buf, Op::I32Const(vtable_off as i32));
                                emit_instruction(buf, Op::I32Add);
                            }
                            emit_instruction(buf, Op::I32Load(0));
                            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        } else {
                            // Source is a class/reference; use the object pointer as the context
                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            emit_instruction(buf, Op::LocalSet(self.temp_local));

                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            if let Some(off) = src_class_vtable_offset {
                                if off != 0 {
                                    emit_instruction(buf, Op::I32Const(off as i32));
                                    emit_instruction(buf, Op::I32Add);
                                }
                            }
                            emit_instruction(buf, Op::I32Load(0));
                            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        }

                        for (class_off, trait_off) in &remap {
                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            emit_instruction(buf, Op::I32Const(*class_off as i32));
                            emit_instruction(buf, Op::I32Eq);
                            emit_instruction(buf, Op::If);
                            emit_instruction(buf, Op::I32Const(*trait_off as i32));
                            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                            emit_instruction(buf, Op::End);
                        }

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if context_off != 0 {
                            emit_instruction(buf, Op::I32Const(context_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::I32Store(0));

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if vtable_off != 0 {
                            emit_instruction(buf, Op::I32Const(vtable_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::I32Store(0));
                        if trait_ty.canonical_name().contains("ThreadStart") {
                            if let Some(run_index) = self.lookup_function_index(
                                "Std::Platform::Thread::ThreadFunctionStartAdapter::Run",
                            ) {
                                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                                emit_instruction(buf, Op::I32Const(0));
                                emit_instruction(buf, Op::I32Add);
                                emit_instruction(
                                    buf,
                                    Op::I32Const(i32::try_from(run_index).unwrap_or(0)),
                                );
                                emit_instruction(buf, Op::I32Store(0));
                            }
                        }
                        return Ok(true);
                    }
                }
                if let Some(offset) = vtable_offset {
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    let context_from_src = self.ty_is_trait_object_like(&src_access.value_ty);
                    if context_from_src {
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::I32Load(0));
                    } else {
                        let context_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                        Self::ensure_operand_type(
                            context_ty,
                            ValueType::I32,
                            "trait object context pointer",
                        )?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    }
                    emit_instruction(buf, Op::I32Store(0));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::I32Const(4));
                    emit_instruction(buf, Op::I32Add);
                    emit_instruction(buf, Op::I32Const(offset as i32));
                    emit_instruction(buf, Op::I32Store(0));
                    if trait_ty.canonical_name().contains("ThreadStart") {
                        if let Some(run_index) = self.lookup_function_index(
                            "Std::Platform::Thread::ThreadFunctionStartAdapter::Run",
                        ) {
                            emit_instruction(buf, Op::I32Const(offset as i32));
                            emit_instruction(
                                buf,
                                Op::I32Const(i32::try_from(run_index).unwrap_or(0)),
                            );
                            emit_instruction(buf, Op::I32Store(0));
                        }
                    }
                    return Ok(true);
                }
                if self.ty_is_trait_object_like(&src_access.value_ty) {
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_trait_object(buf, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                self.emit_pointer_expression(buf, access)?;
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                let context_ty = self.emit_operand(buf, &Operand::Copy(src.clone()))?;
                Self::ensure_operand_type(
                    context_ty,
                    ValueType::I32,
                    "trait object context pointer",
                )?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::I32Store(0));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(4));
                emit_instruction(buf, Op::I32Add);
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::I32Load(0));
                emit_instruction(buf, Op::I32Store(0));
                Ok(true)
            }
            Rvalue::Use(Operand::Borrow(borrow)) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                let impl_ty = self.operand_ty(&Operand::Borrow(borrow.clone()));
                let vtable_offset = impl_ty
                    .as_ref()
                    .and_then(|ty| lookup_vtable_offset(self, ty))
                    .or_else(|| lookup_vtable_offset(self, &src_access.value_ty));
                if vtable_offset.is_none() && trait_ty.canonical_name().contains("ThreadStart") {
                    let remap: Vec<(u32, u32)> = self
                        .trait_vtables
                        .iter()
                        .filter(|table| {
                            Self::names_equivalent(&table.trait_name, &trait_ty.canonical_name())
                        })
                        .filter_map(|table| {
                            let trait_off = *self.trait_vtable_offsets.get(&table.symbol)?;
                            let class_symbol = class_vtable_symbol_name(&table.impl_type);
                            let class_off =
                                self.class_vtable_offsets.get(&class_symbol).copied()?;
                            Some((class_off, trait_off))
                        })
                        .collect();
                    if !remap.is_empty() {
                        let (context_off, vtable_off) = self
                            .lookup_struct_layout(trait_ty)
                            .and_then(|layout| {
                                let ctx = layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off == 0));
                                let vt = layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off != 0));
                                Some((ctx.unwrap_or(0) as u32, vt.unwrap_or(4) as u32))
                            })
                            .unwrap_or((0, 4));
                        self.emit_pointer_expression(buf, access)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if context_off != 0 {
                            emit_instruction(buf, Op::I32Const(context_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.temp_local));

                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if vtable_off != 0 {
                            emit_instruction(buf, Op::I32Const(vtable_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

                        if trait_ty.canonical_name().contains("ThreadStart") {
                            emit_instruction(buf, Op::LocalGet(self.block_local));
                            if context_off != 0 {
                                emit_instruction(buf, Op::I32Const(context_off as i32));
                                emit_instruction(buf, Op::I32Add);
                            }
                            emit_instruction(buf, Op::LocalGet(self.temp_local));
                            emit_instruction(buf, Op::I32Store(0));

                            emit_instruction(buf, Op::LocalGet(self.block_local));
                            if vtable_off != 0 {
                                emit_instruction(buf, Op::I32Const(vtable_off as i32));
                                emit_instruction(buf, Op::I32Add);
                            }
                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            emit_instruction(buf, Op::I32Store(0));
                            if let Some(run_index) = self.lookup_function_index(&format!(
                                "{}::Run",
                                src_access.value_ty.canonical_name()
                            )) {
                                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                                emit_instruction(buf, Op::I32Const(0));
                                emit_instruction(buf, Op::I32Add);
                                emit_instruction(
                                    buf,
                                    Op::I32Const(i32::try_from(run_index).unwrap_or(0)),
                                );
                                emit_instruction(buf, Op::I32Store(0));
                            }
                            return Ok(true);
                        }

                        for (class_off, trait_off) in &remap {
                            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                            emit_instruction(buf, Op::I32Const(*class_off as i32));
                            emit_instruction(buf, Op::I32Eq);
                            emit_instruction(buf, Op::If);
                            emit_instruction(buf, Op::I32Const(*trait_off as i32));
                            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                            emit_instruction(buf, Op::End);
                        }

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if context_off != 0 {
                            emit_instruction(buf, Op::I32Const(context_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::I32Store(0));

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if vtable_off != 0 {
                            emit_instruction(buf, Op::I32Const(vtable_off as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::I32Store(0));
                        return Ok(true);
                    }
                }
                if let Some(offset) = vtable_offset {
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    let context_from_src = self.ty_is_trait_object_like(&src_access.value_ty);
                    if context_from_src {
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::I32Load(0));
                    } else {
                        let context_ty =
                            self.emit_operand(buf, &Operand::Borrow(borrow.clone()))?;
                        Self::ensure_operand_type(
                            context_ty,
                            ValueType::I32,
                            "trait object context pointer",
                        )?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    }
                    emit_instruction(buf, Op::I32Store(0));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::I32Const(4));
                    emit_instruction(buf, Op::I32Add);
                    emit_instruction(buf, Op::I32Const(offset as i32));
                    emit_instruction(buf, Op::I32Store(0));
                    if trait_ty.canonical_name().contains("ThreadStart") {
                        if let Some(run_index) = self.lookup_function_index(
                            "Std::Platform::Thread::ThreadFunctionStartAdapter::Run",
                        ) {
                            emit_instruction(buf, Op::I32Const(offset as i32));
                            emit_instruction(
                                buf,
                                Op::I32Const(i32::try_from(run_index).unwrap_or(0)),
                            );
                            emit_instruction(buf, Op::I32Store(0));
                        }
                    }
                    return Ok(true);
                }
                if self.ty_is_trait_object_like(&src_access.value_ty) {
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_trait_object(buf, self.stack_temp_local, self.temp_local)?;
                    return Ok(true);
                }
                self.emit_pointer_expression(buf, access)?;
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                let context_ty = self.emit_operand(buf, &Operand::Borrow(borrow.clone()))?;
                Self::ensure_operand_type(
                    context_ty,
                    ValueType::I32,
                    "trait object context pointer",
                )?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::I32Store(0));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(4));
                emit_instruction(buf, Op::I32Add);
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::I32Load(0));
                emit_instruction(buf, Op::I32Store(0));
                Ok(true)
            }
            Rvalue::Use(Operand::Const(constant)) => {
                if matches!(constant.value(), ConstValue::Null) {
                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::I32Const(0));
                    emit_instruction(buf, Op::I32Store(0));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::I32Const(4));
                    emit_instruction(buf, Op::I32Add);
                    emit_instruction(buf, Op::I32Const(0));
                    emit_instruction(buf, Op::I32Store(0));
                    return Ok(true);
                }
                if let ConstValue::Str { id, .. } = constant.value() {
                    let mut vtable_offset = lookup_vtable_offset(self, &Ty::Str);
                    let mut use_string_impl = false;
                    if vtable_offset.is_none() {
                        vtable_offset = lookup_vtable_offset(self, &Ty::String);
                        use_string_impl = vtable_offset.is_some();
                    }
                    let offset = vtable_offset.ok_or_else(|| {
                        Error::Codegen(format!(
                            "trait object assignment missing vtable for string literal in `{}`",
                            trait_ty.canonical_name()
                        ))
                    })?;
                    let impl_ty = if use_string_impl { Ty::String } else { Ty::Str };
                    let (size, align) =
                        self.layouts
                            .size_and_align_for_ty(&impl_ty)
                            .ok_or_else(|| {
                                Error::Codegen(format!(
                                    "missing layout for `{}` in WASM backend",
                                    impl_ty.canonical_name()
                                ))
                            })?;
                    let size = ensure_u32(size.max(1), "string literal context size overflow")?;
                    let align = ensure_u32(align.max(1), "string literal context align overflow")?;
                    let literal = self.string_literals.get(id).ok_or_else(|| {
                        Error::Codegen(format!("missing interned string literal {}", id.index()))
                    })?;

                    self.allocate_stack_block(buf, size, align)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalSet(self.block_local));

                    if use_string_impl {
                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        emit_instruction(buf, Op::I32Const(literal.offset as i32));
                        emit_instruction(buf, Op::I32Const(literal.len as i32));
                        let hook = self.runtime_hook_index(RuntimeHook::StringCloneSlice)?;
                        emit_instruction(buf, Op::Call(hook));
                        emit_instruction(buf, Op::Drop);
                    } else {
                        let literal_ty = self.emit_str_literal(buf, *id)?;
                        Self::ensure_operand_type(literal_ty, ValueType::I64, "str literal value")?;
                        emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                        emit_instruction(buf, Op::I64Store(0));
                    }

                    self.emit_pointer_expression(buf, access)?;
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::I32Store(0));
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::I32Const(4));
                    emit_instruction(buf, Op::I32Add);
                    emit_instruction(buf, Op::I32Const(offset as i32));
                    emit_instruction(buf, Op::I32Store(0));
                    return Ok(true);
                }
                Err(Error::Codegen(format!(
                    "trait object assignment does not support constant operand {:?}",
                    constant.value()
                )))
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn copy_fn_fields(
        &self,
        buf: &mut Vec<u8>,
        layout: &StructLayout,
        src_local: u32,
        dest_local: u32,
    ) -> Result<(), Error> {
        for field in &layout.fields {
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer field `{}` missing offset for WASM lowering",
                    field.name
                ))
            })?;
            let offset = ensure_u32(
                offset,
                "function pointer field offset exceeds wasm32 addressable range",
            )?;
            emit_instruction(buf, Op::LocalGet(dest_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::LocalGet(src_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            match map_type(&field.ty) {
                ValueType::I32 => emit_instruction(buf, Op::I32Load(0)),
                ValueType::I64 => emit_instruction(buf, Op::I64Load(0)),
                ValueType::F32 => emit_instruction(buf, Op::F32Load(0)),
                ValueType::F64 => emit_instruction(buf, Op::F64Load(0)),
            }
            match map_type(&field.ty) {
                ValueType::I32 => emit_instruction(buf, Op::I32Store(0)),
                ValueType::I64 => emit_instruction(buf, Op::I64Store(0)),
                ValueType::F32 => emit_instruction(buf, Op::F32Store(0)),
                ValueType::F64 => emit_instruction(buf, Op::F64Store(0)),
            }
        }
        Ok(())
    }

    fn copy_trait_object(
        &self,
        buf: &mut Vec<u8>,
        src_local: u32,
        dest_local: u32,
    ) -> Result<(), Error> {
        for offset in [0, 4] {
            emit_instruction(buf, Op::LocalGet(dest_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::LocalGet(src_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::I32Load(0));
            emit_instruction(buf, Op::I32Store(0));
        }
        Ok(())
    }

    pub(crate) fn initialise_fn_from_const(
        &self,
        buf: &mut Vec<u8>,
        layout: &StructLayout,
        fn_ty: &Ty,
        constant: &ConstOperand,
    ) -> Result<(), Error> {
        let invoke = match constant.value() {
            ConstValue::Symbol(name) => {
                let index = self.lookup_function_index(name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unable to resolve function `{name}` for function pointer literal"
                    ))
                })?;
                if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                    eprintln!(
                        "[wasm-fn-const] func={} ty={} layout={} symbol={} index={}",
                        self.function.name,
                        fn_ty.canonical_name(),
                        layout.name,
                        name,
                        index
                    );
                }
                Some(i32::try_from(index).map_err(|_| {
                    Error::Codegen("function index exceeds i32 range in WASM backend".into())
                })?)
            }
            ConstValue::Null => None,
            other => {
                return Err(Error::Codegen(format!(
                    "function pointer assignment does not support constant operand {other:?}"
                )));
            }
        };
        let type_id = drop_type_identity(&fn_ty.canonical_name()) as i64;
        for field in &layout.fields {
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer field `{}` missing offset for WASM lowering",
                    field.name
                ))
            })?;
            let offset = ensure_u32(
                offset,
                "function pointer field offset exceeds wasm32 addressable range",
            )?;
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            let value_ty = map_type(&field.ty);
            let key = Self::fn_field_key(&field.name).ok_or_else(|| {
                Error::Codegen(format!(
                    "unknown function pointer field `{}` in WASM lowering",
                    field.name
                ))
            })?;
            match key {
                "invoke" => {
                    let value = invoke.unwrap_or(0);
                    emit_instruction(buf, Op::I32Const(value));
                }
                "context" | "drop_glue" | "env_size" | "env_align" => {
                    emit_instruction(buf, Op::I32Const(0));
                }
                "type_id" => {
                    emit_instruction(buf, Op::I64Const(type_id));
                }
                _ => unreachable!("unexpected function pointer field key"),
            }
            match value_ty {
                ValueType::I32 => emit_instruction(buf, Op::I32Store(0)),
                ValueType::I64 => emit_instruction(buf, Op::I64Store(0)),
                ValueType::F32 => emit_instruction(buf, Op::F32Store(0)),
                ValueType::F64 => emit_instruction(buf, Op::F64Store(0)),
            }
        }
        Ok(())
    }

    fn emit_tuple_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        tuple_ty: &TupleTy,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                self.copy_tuple_fields(buf, place, src, tuple_ty)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn fn_field_key(name: &str) -> Option<&'static str> {
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "invoke" => Some("invoke"),
            "context" => Some("context"),
            "drop_glue" | "dropglue" => Some("drop_glue"),
            "type_id" | "typeid" => Some("type_id"),
            "env_size" | "envsize" => Some("env_size"),
            "env_align" | "envalign" => Some("env_align"),
            _ => None,
        }
    }

    pub(crate) fn is_fn_pointer_layout(layout: &StructLayout) -> bool {
        let expected = [
            "invoke",
            "context",
            "drop_glue",
            "type_id",
            "env_size",
            "env_align",
        ];
        layout.fields.len() == expected.len()
            && layout
                .fields
                .iter()
                .zip(expected.iter())
                .all(|(field, name)| Self::fn_field_key(&field.name) == Some(*name))
    }

    fn emit_aggregate_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        kind: &AggregateKind,
        fields: &[Operand],
    ) -> Result<(), Error> {
        match kind {
            AggregateKind::Tuple | AggregateKind::Array => {
                for (index, field) in fields.iter().enumerate() {
                    let mut field_place = place.clone();
                    field_place
                        .projection
                        .push(ProjectionElem::Field(index as u32));
                    let field_rvalue = Rvalue::Use(field.clone());
                    self.emit_assign(buf, &field_place, &field_rvalue)?;
                }
                Ok(())
            }
            AggregateKind::Adt { name, variant } => {
                if variant.is_some() {
                    return Err(Error::Codegen(
                        "enum variant aggregate assignment is not yet supported in WASM backend"
                            .into(),
                    ));
                }
                let Some(layout) = self.layouts.types.get(name.as_str()) else {
                    return Err(Error::Codegen(format!(
                        "missing layout for aggregate `{}` in WASM backend",
                        name
                    )));
                };
                let struct_layout = match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data,
                    other => {
                        return Err(Error::Codegen(format!(
                            "aggregate assignment only supports struct/class layouts in WASM backend; found {other:?}"
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
                    let field_rvalue = Rvalue::Use(value.clone());
                    self.emit_assign(buf, &field_place, &field_rvalue)?;
                }
                Ok(())
            }
        }
    }

    fn emit_named_aggregate_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        ty: &Ty,
    ) -> Result<bool, Error> {
        let Ty::Named(named) = ty else {
            return Ok(false);
        };
        if is_builtin_primitive(&self.layouts.primitive_registry, named.as_str()) {
            return Ok(false);
        }
        // Named aggregates that require memory are always copied as raw bytes in the WASM backend.
        // Field-by-field copies are both slower and can miss padding/ABI details (e.g., ValuePtr
        // handles that must roundtrip through runtime shims).
        if local_requires_memory(ty, self.layouts) {
            return Ok(false);
        }
        if env::var_os("CHIC_DEBUG_WASM_AGG").is_some() {
            let repr = self
                .representations
                .get(place.local.0)
                .copied()
                .unwrap_or(LocalRepresentation::Scalar);
            eprintln!(
                "[wasm-agg-assign] func={} local={} repr={:?} ty={} proj={:?}",
                self.function.name,
                place.local.0,
                repr,
                ty.canonical_name(),
                place.projection
            );
        }
        if matches!(
            self.representations
                .get(place.local.0)
                .copied()
                .unwrap_or(LocalRepresentation::Scalar),
            LocalRepresentation::Scalar
        ) {
            // Scalar locals for single-field structs are handled as plain values.
            return Ok(false);
        }
        if self.ty_is_reference(ty) {
            return Ok(false);
        }
        let layout = match self.lookup_struct_layout(ty).cloned() {
            Some(layout) => layout,
            None => return Ok(false),
        };
        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                self.copy_named_fields(buf, place, src, &layout)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn copy_named_fields(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        for field in &layout.fields {
            let mut dest_field = dest.clone();
            dest_field
                .projection
                .push(ProjectionElem::FieldNamed(field.name.clone()));
            let mut src_field = src.clone();
            src_field
                .projection
                .push(ProjectionElem::FieldNamed(field.name.clone()));
            let operand = Operand::Copy(src_field);
            let field_rvalue = Rvalue::Use(operand);
            self.emit_assign(buf, &dest_field, &field_rvalue)?;
        }
        Ok(())
    }

    fn emit_decimal_constant_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Decimal128,
    ) -> Result<(), Error> {
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        let parts = value.to_bits();
        for (index, part) in parts.iter().enumerate() {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            let offset = (index * 4) as i32;
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::I32Const(*part as i32));
            emit_instruction(buf, Op::I32Store(0));
        }
        Ok(())
    }

    fn emit_decimal_intrinsic_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        decimal: &DecimalIntrinsic,
    ) -> Result<(), Error> {
        let result_access = self.resolve_memory_access(place)?;
        // stash vector hint for reuse
        let vector_ty = self.emit_decimal_enum_operand(buf, &decimal.vectorize)?;
        if !matches!(vector_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "decimal vectorize operand must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::Drop);

        self.emit_decimal_runtime_call(buf, &result_access, decimal, false)?;
        self.emit_decimal_runtime_call(buf, &result_access, decimal, true)?;

        self.store_decimal_intrinsic_variant(buf, &result_access)?;
        Ok(())
    }

    fn emit_decimal_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        result_access: &MemoryAccess,
        decimal: &DecimalIntrinsic,
        vectorized: bool,
    ) -> Result<(), Error> {
        let base_hook = match decimal.kind {
            DecimalIntrinsicKind::Add => RuntimeHook::DecimalAdd,
            DecimalIntrinsicKind::Sub => RuntimeHook::DecimalSub,
            DecimalIntrinsicKind::Mul => RuntimeHook::DecimalMul,
            DecimalIntrinsicKind::Div => RuntimeHook::DecimalDiv,
            DecimalIntrinsicKind::Rem => RuntimeHook::DecimalRem,
            DecimalIntrinsicKind::Fma => RuntimeHook::DecimalFma,
        };
        let hook = if vectorized {
            match base_hook {
                RuntimeHook::DecimalAdd => RuntimeHook::DecimalAddSimd,
                RuntimeHook::DecimalSub => RuntimeHook::DecimalSubSimd,
                RuntimeHook::DecimalMul => RuntimeHook::DecimalMulSimd,
                RuntimeHook::DecimalDiv => RuntimeHook::DecimalDivSimd,
                RuntimeHook::DecimalRem => RuntimeHook::DecimalRemSimd,
                RuntimeHook::DecimalFma => RuntimeHook::DecimalFmaSimd,
                other => other,
            }
        } else {
            base_hook
        };

        self.emit_pointer_expression(buf, result_access)?;
        self.emit_decimal_operand_pointer(buf, &decimal.lhs)?;
        self.emit_decimal_operand_pointer(buf, &decimal.rhs)?;
        if decimal.kind == DecimalIntrinsicKind::Fma {
            let addend = decimal.addend.as_ref().ok_or_else(|| {
                Error::Codegen("decimal intrinsic `Fma` missing addend operand".into())
            })?;
            self.emit_decimal_operand_pointer(buf, addend)?;
        }

        let rounding_ty = self.emit_decimal_enum_operand(buf, &decimal.rounding)?;
        if !matches!(rounding_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "decimal rounding operand must lower to i32 in WASM backend".into(),
            ));
        }
        let flags = if vectorized {
            DECIMAL_FLAG_VECTORIZE
        } else {
            0
        };
        emit_instruction(buf, Op::I32Const(flags as i32));
        let hook_index = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::Call(hook_index));
        Ok(())
    }

    fn emit_decimal_operand_pointer(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<(), Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(())
            }
            Operand::Const(constant) => {
                if let ConstValue::Decimal(value) = &constant.value {
                    self.allocate_decimal_temp(buf, value, self.stack_temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    Ok(())
                } else {
                    Err(Error::Codegen(
                        "decimal intrinsic operands must be addressable in the WASM backend".into(),
                    ))
                }
            }
            other => Err(Error::Codegen(format!(
                "decimal intrinsic operand {other:?} is not supported in WASM backend"
            ))),
        }
    }

    fn store_decimal_intrinsic_variant(
        &mut self,
        buf: &mut Vec<u8>,
        result_access: &MemoryAccess,
    ) -> Result<(), Error> {
        let ty = Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult");
        let layout = self.lookup_struct_layout(&ty).ok_or_else(|| {
            Error::Codegen("missing `DecimalIntrinsicResult` layout for WASM backend".into())
        })?;
        let field = layout
            .fields
            .iter()
            .find(|field| field.name == "Variant")
            .ok_or_else(|| {
                Error::Codegen("`DecimalIntrinsicResult` layout missing Variant field".into())
            })?;
        let offset = field.offset.ok_or_else(|| {
            Error::Codegen("Variant field missing offset for WASM lowering".into())
        })?;

        self.emit_pointer_expression(buf, result_access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        if offset != 0 {
            emit_instruction(buf, Op::I32Const(offset as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::I32Store(0));
        Ok(())
    }

    fn emit_decimal_enum_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        match operand {
            Operand::Const(constant) => {
                if let ConstValue::Enum { discriminant, .. } = &constant.value {
                    let value = i32::try_from(*discriminant).map_err(|_| {
                        Error::Codegen(
                            "enum discriminant exceeds 32-bit range in WASM backend".into(),
                        )
                    })?;
                    emit_instruction(buf, Op::I32Const(value));
                    Ok(ValueType::I32)
                } else {
                    Err(Error::Codegen(format!(
                        "unsupported constant operand {:?} for decimal enum",
                        constant.value
                    )))
                }
            }
            _ => self.emit_operand(buf, operand),
        }
    }

    pub(crate) fn int128_signed(&self, ty: &Ty) -> Option<bool> {
        let canonical = ty.canonical_name().to_ascii_lowercase();
        match canonical.as_str() {
            "i128" | "int128" | "std::int128" | "system::int128" => Some(true),
            "u128" | "uint128" | "std::uint128" | "system::uint128" => Some(false),
            _ => None,
        }
    }

    fn emit_int128_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        signed: bool,
    ) -> Result<(), Error> {
        let dest_access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &dest_access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let pointer_size = self.pointer_width_bits() / 8;
        let int_info_for = |emitter: &Self, name: &str| -> Option<IntInfo> {
            if let Some(info) = int_info(&emitter.layouts.primitive_registry, name, pointer_size) {
                return Some(info);
            }
            let layout = emitter.layouts.layout_for_name(name)?;
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

        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                let src_access = self.resolve_memory_access(src)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.copy_int128(buf, self.temp_local, self.block_local);
                return Ok(());
            }
            Rvalue::Use(Operand::Pending(_)) => {
                self.store_int128_parts(buf, self.temp_local, 0, 0);
                return Ok(());
            }
            Rvalue::Use(Operand::Const(constant)) => {
                let (lo, hi) = self.int128_const_parts(&constant.value, signed)?;
                self.store_int128_parts(buf, self.temp_local, lo, hi);
                return Ok(());
            }
            Rvalue::Cast {
                kind: CastKind::IntToInt,
                operand,
                source,
                target,
                ..
            } => {
                let source_name = source.canonical_name();
                let target_name = target.canonical_name();
                let source_info = int_info_for(self, &source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let target_info = int_info_for(self, &target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                if target_info.bits != 128 {
                    return Err(Error::Codegen(format!(
                        "unsupported int-to-int cast to {}-bit target in WASM backend",
                        target_info.bits
                    )));
                }
                if source_info.bits == 128 {
                    self.materialize_int128_operand(
                        buf,
                        operand,
                        source_info.signed,
                        self.block_local,
                    )?;
                    self.copy_int128(buf, self.temp_local, self.block_local);
                    return Ok(());
                }
                if source_info.bits == 0 || source_info.bits > 128 {
                    return Err(Error::Codegen(format!(
                        "source integer width {} is not supported in WASM backend",
                        source_info.bits
                    )));
                }
                let source_bits = u32::from(source_info.bits);
                self.emit_numeric_operand_as(
                    buf,
                    operand,
                    ValueType::I64,
                    source_bits,
                    source_info.signed,
                )?;
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Store(0));
                if source_info.signed {
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Const(63));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    emit_instruction(buf, Op::I64Const(0));
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::I64Store(8));
                return Ok(());
            }
            Rvalue::Cast {
                kind: CastKind::FloatToInt,
                operand,
                target,
                ..
            } => {
                let target_name = target.canonical_name();
                let target_info = int_info_for(self, &target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                if target_info.bits != 128 {
                    return Err(Error::Codegen(format!(
                        "unsupported float-to-int cast to {}-bit target in WASM backend",
                        target_info.bits
                    )));
                }
                let value_ty = self.emit_operand(buf, operand)?;
                let convert_op = match value_ty {
                    ValueType::F32 => {
                        if signed {
                            Op::I64TruncF32S
                        } else {
                            Op::I64TruncF32U
                        }
                    }
                    ValueType::F64 => {
                        if signed {
                            Op::I64TruncF64S
                        } else {
                            Op::I64TruncF64U
                        }
                    }
                    other => {
                        return Err(Error::Codegen(format!(
                            "expected floating-point operand for int128 cast, found {:?}",
                            other
                        )));
                    }
                };
                emit_instruction(buf, convert_op);
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Store(0));
                if signed {
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Const(63));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    emit_instruction(buf, Op::I64Const(0));
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::I64Store(8));
                return Ok(());
            }
            Rvalue::Unary { op, operand, .. } => match op {
                UnOp::UnaryPlus => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    self.copy_int128(buf, self.temp_local, self.block_local);
                    return Ok(());
                }
                UnOp::Neg if signed => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    let call = self.runtime_hook_index(RuntimeHook::I128Neg)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                UnOp::BitNot => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Not
                    } else {
                        RuntimeHook::U128Not
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                _ => {}
            },
            Rvalue::Binary { op, lhs, rhs, .. } => match op {
                crate::mir::BinOp::Add => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Add
                    } else {
                        RuntimeHook::U128Add
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Sub => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Sub
                    } else {
                        RuntimeHook::U128Sub
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Mul => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Mul
                    } else {
                        RuntimeHook::U128Mul
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Div => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Div
                    } else {
                        RuntimeHook::U128Div
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Rem => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Rem
                    } else {
                        RuntimeHook::U128Rem
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitAnd => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128And
                    } else {
                        RuntimeHook::U128And
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitOr => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Or
                    } else {
                        RuntimeHook::U128Or
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitXor => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Xor
                    } else {
                        RuntimeHook::U128Xor
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Shl => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    let amount_ty = self.emit_operand(buf, rhs)?;
                    Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    let hook = if signed {
                        RuntimeHook::I128Shl
                    } else {
                        RuntimeHook::U128Shl
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Shr => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    let amount_ty = self.emit_operand(buf, rhs)?;
                    Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    let hook = if signed {
                        RuntimeHook::I128Shr
                    } else {
                        RuntimeHook::U128Shr
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                _ => {}
            },
            _ => {}
        }

        Err(Error::Codegen(format!(
            "unsupported int128 assignment for rvalue {:?} in WASM backend",
            value
        )))
    }

    pub(crate) fn materialize_int128_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        signed: bool,
        scratch_local: u32,
    ) -> Result<(), Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::LocalSet(scratch_local));
                Ok(())
            }
            Operand::Const(constant) => {
                let (lo, hi) = self.int128_const_parts(&constant.value, signed)?;
                self.allocate_int128_temp(buf, lo, hi, scratch_local)
            }
            _ => Err(Error::Codegen(
                "unsupported operand for int128 lowering in WASM backend".into(),
            )),
        }
    }

    pub(crate) fn allocate_int128_temp(
        &mut self,
        buf: &mut Vec<u8>,
        lo: u64,
        hi: i64,
        target_local: u32,
    ) -> Result<(), Error> {
        let size = 16i32;
        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::LocalTee(target_local));
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::LocalGet(target_local));
        emit_instruction(buf, Op::I64Const(lo as i64));
        emit_instruction(buf, Op::I64Store(0));
        emit_instruction(buf, Op::LocalGet(target_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Const(hi));
        emit_instruction(buf, Op::I64Store(0));
        Ok(())
    }

    fn allocate_decimal_temp(
        &mut self,
        buf: &mut Vec<u8>,
        value: &Decimal128,
        target_local: u32,
    ) -> Result<(), Error> {
        let size = 16i32;
        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::LocalTee(target_local));
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));

        for (index, part) in value.to_bits().iter().enumerate() {
            emit_instruction(buf, Op::LocalGet(target_local));
            let offset = (index * 4) as i32;
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::I32Const(*part as i32));
            emit_instruction(buf, Op::I32Store(0));
        }

        Ok(())
    }

    fn store_int128_parts(&self, buf: &mut Vec<u8>, dest_local: u32, lo: u64, hi: i64) {
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I64Const(lo as i64));
        emit_instruction(buf, Op::I64Store(0));
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Const(hi));
        emit_instruction(buf, Op::I64Store(0));
    }

    pub(crate) fn int128_const_parts(
        &self,
        value: &ConstValue,
        signed: bool,
    ) -> Result<(u64, i64), Error> {
        if signed {
            let raw: i128 = match value {
                ConstValue::Int(v) | ConstValue::Int32(v) => *v,
                ConstValue::UInt(v) => *v as i128,
                ConstValue::Bool(b) => i128::from(*b),
                ConstValue::Decimal(decimal) => decimal.to_encoding() as i128,
                _ => {
                    return Err(Error::Codegen(format!(
                        "unsupported int128 constant kind for WASM backend: {:?}",
                        value
                    )));
                }
            };
            let lo = raw as u128 as u64;
            let hi = (raw >> 64) as i64;
            Ok((lo, hi))
        } else {
            let raw: u128 = match value {
                ConstValue::UInt(v) => *v,
                ConstValue::Int(v) | ConstValue::Int32(v) if *v >= 0 => *v as u128,
                ConstValue::Bool(b) => u128::from(*b),
                ConstValue::Decimal(decimal) => decimal.to_encoding(),
                _ => {
                    return Err(Error::Codegen(format!(
                        "unsupported uint128 constant kind for WASM backend: {:?}",
                        value
                    )));
                }
            };
            let lo = raw as u64;
            let hi = (raw >> 64) as u64;
            Ok((lo, hi as i64))
        }
    }

    fn copy_int128(&self, buf: &mut Vec<u8>, dest_local: u32, src_local: u32) {
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::LocalGet(src_local));
        emit_instruction(buf, Op::I64Load(0));
        emit_instruction(buf, Op::I64Store(0));

        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalGet(src_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Load(0));
        emit_instruction(buf, Op::I64Store(0));
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

    fn emit_span_stack_alloc(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        element: &Ty,
        length: &Operand,
        source: Option<&Operand>,
    ) -> Result<(), Error> {
        if !place.projection.is_empty() {
            return Err(Error::Codegen(
                "span stack allocation requires assigning to a local binding".into(),
            ));
        }

        let (elem_size, elem_align) =
            self.layouts.size_and_align_for_ty(element).ok_or_else(|| {
                Error::Codegen(format!(
                    "span stack allocation element `{}` is not sized",
                    element.canonical_name()
                ))
            })?;
        let elem_size_i32 = i32::try_from(elem_size).map_err(|_| {
            Error::Codegen(format!(
                "span element `{}` size `{elem_size}` exceeds wasm i32 range",
                element.canonical_name()
            ))
        })?;
        let elem_align_i32 = i32::try_from(elem_align).map_err(|_| {
            Error::Codegen(format!(
                "span element `{}` alignment `{elem_align}` exceeds wasm i32 range",
                element.canonical_name()
            ))
        })?;
        let offsets = self.span_ptr_offsets(false)?;

        let dest_ptr = self.pointer_local_index(place.local)?;
        let len_ty = self.emit_operand(buf, length)?;
        Self::ensure_operand_type(len_ty, ValueType::I32, "span stack allocation length")?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        if elem_size == 0 {
            emit_instruction(buf, Op::I32Const(0));
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        } else {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            if elem_size != 1 {
                emit_instruction(buf, Op::I32Const(elem_size_i32));
                emit_instruction(buf, Op::I32Mul);
            }
            if elem_align > 1 {
                emit_instruction(buf, Op::I32Const(elem_align_i32 - 1));
                emit_instruction(buf, Op::I32Add);
                emit_instruction(buf, Op::I32Const(elem_align_i32));
                emit_instruction(buf, Op::I32DivS);
                emit_instruction(buf, Op::I32Const(elem_align_i32));
                emit_instruction(buf, Op::I32Mul);
            }
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
            emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I32Add);
            emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
            emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I32Sub);
            emit_instruction(buf, Op::LocalTee(self.stack_temp_local));
            emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
        }

        if elem_size == 0 {
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        }

        // ValueMutPtr.Data.Pointer
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Store(0));

        // ValueMutPtr.Data.Size
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_size != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_size_i32));
        emit_instruction(buf, Op::I32Store(0));

        // ValueMutPtr.Data.Alignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_align != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_align_i32));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.Length
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.ElementSize
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_size_i32));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.ElementAlignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_align_i32));
        emit_instruction(buf, Op::I32Store(0));
        if let Some(source) = source {
            self.emit_span_copy_from_source(buf, place, source)?;
        }
        Ok(())
    }

    fn emit_span_copy_from_source(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        source: &Operand,
    ) -> Result<(), Error> {
        let source_place = match source {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => {
                return Err(Error::Codegen(
                    "span stack allocation source must be addressable in WASM backend".into(),
                ));
            }
        };
        if !source_place.projection.is_empty() {
            return Err(Error::Codegen(
                "projected span stack allocation sources are not yet supported in WASM backend"
                    .into(),
            ));
        }
        let source_ptr = self.pointer_local_index(source_place.local)?;
        let dest_ptr = self.pointer_local_index(dest.local)?;
        let source_ty = self
            .local_tys
            .get(source_place.local.0)
            .cloned()
            .map(|ty| self.resolve_self_ty(&ty))
            .unwrap_or_else(|| Ty::named("Std::Span::SpanPtr"));
        let source_readonly = matches!(source_ty, Ty::ReadOnlySpan(_));
        let source_offsets = self.span_ptr_offsets(source_readonly)?;
        let dest_offsets = self.span_ptr_offsets(false)?;

        // source pointer
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source length
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source element size
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source element alignment
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));

        // dest pointer
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest length
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest element size
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest element alignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));

        let hook = self.runtime_hook_index(RuntimeHook::SpanCopyTo)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn span_ptr_offsets(&self, readonly: bool) -> Result<SpanOffsets, Error> {
        let raw_ty = if readonly {
            Ty::named("Std::Span::ReadOnlySpanPtr")
        } else {
            Ty::named("Std::Span::SpanPtr")
        };
        let (data_field, data_offset) = self.resolve_field_by_name(&raw_ty, None, "Data")?;
        let (_ptr_field, ptr_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Pointer")?;
        let (_size_field, size_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Size")?;
        let (_align_field, align_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Alignment")?;
        let (_, len_offset) = self.resolve_field_by_name(&raw_ty, None, "Length")?;
        let (_, elem_size_offset) = self.resolve_field_by_name(&raw_ty, None, "ElementSize")?;
        let (_, elem_align_offset) =
            self.resolve_field_by_name(&raw_ty, None, "ElementAlignment")?;

        let data_ptr = data_offset
            .checked_add(ptr_offset)
            .ok_or_else(|| Error::Codegen("span data pointer offset overflowed".into()))?;
        let data_size = data_offset
            .checked_add(size_offset)
            .ok_or_else(|| Error::Codegen("span data size offset overflowed".into()))?;
        let data_align = data_offset
            .checked_add(align_offset)
            .ok_or_else(|| Error::Codegen("span data alignment offset overflowed".into()))?;

        Ok(SpanOffsets {
            data_ptr: ensure_u32(
                data_ptr,
                "span data pointer offset exceeds 32-bit range in WASM backend",
            )?,
            data_size: ensure_u32(
                data_size,
                "span data size offset exceeds 32-bit range in WASM backend",
            )?,
            data_align: ensure_u32(
                data_align,
                "span data alignment offset exceeds 32-bit range in WASM backend",
            )?,
            len: ensure_u32(
                len_offset,
                "span length offset exceeds 32-bit range in WASM backend",
            )?,
            elem_size: ensure_u32(
                elem_size_offset,
                "span element size offset exceeds 32-bit range in WASM backend",
            )?,
            elem_align: ensure_u32(
                elem_align_offset,
                "span element alignment offset exceeds 32-bit range in WASM backend",
            )?,
        })
    }

    fn emit_numeric_intrinsic_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
    ) -> Result<(), Error> {
        let bits = numeric_width_bits(intrinsic.width);
        let value_ty = numeric_value_ty(intrinsic.width);
        match intrinsic.kind {
            NumericIntrinsicKind::TryAdd
            | NumericIntrinsicKind::TrySub
            | NumericIntrinsicKind::TryMul
            | NumericIntrinsicKind::TryNeg => {
                self.emit_numeric_checked_arith(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::LeadingZeroCount
            | NumericIntrinsicKind::TrailingZeroCount
            | NumericIntrinsicKind::PopCount => {
                self.emit_numeric_count_intrinsic(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::RotateLeft | NumericIntrinsicKind::RotateRight => {
                self.emit_numeric_rotate_intrinsic(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::ReverseEndianness => {
                self.emit_numeric_reverse_endianness(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::IsPowerOfTwo => {
                self.emit_numeric_is_power_of_two(buf, place, intrinsic, bits, value_ty)?
            }
        }
        Ok(())
    }

    fn emit_numeric_checked_arith(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        let result_local = match value_ty {
            ValueType::I64 => self.wide_temp_local,
            _ => self.temp_local,
        };
        match intrinsic.kind {
            NumericIntrinsicKind::TryNeg => {
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[0],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                emit_instruction(buf, Op::LocalSet(result_local));
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I32Const(0)),
                    ValueType::I64 => emit_instruction(buf, Op::I64Const(0)),
                    _ => unreachable!("numeric intrinsic uses integer operands"),
                }
                emit_instruction(buf, Op::LocalGet(result_local));
                emit_instruction(buf, self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub));
                self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
                emit_instruction(buf, Op::LocalSet(result_local));
            }
            NumericIntrinsicKind::TryAdd
            | NumericIntrinsicKind::TrySub
            | NumericIntrinsicKind::TryMul => {
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[0],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[1],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                let op = match intrinsic.kind {
                    NumericIntrinsicKind::TryAdd => {
                        self.op_for_int(value_ty, Op::I32Add, Op::I64Add)
                    }
                    NumericIntrinsicKind::TrySub => {
                        self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub)
                    }
                    NumericIntrinsicKind::TryMul => {
                        self.op_for_int(value_ty, Op::I32Mul, Op::I64Mul)
                    }
                    _ => unreachable!(),
                };
                emit_instruction(buf, op);
                self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
                emit_instruction(buf, Op::LocalSet(result_local));
            }
            _ => unreachable!("checked arithmetic helper used for Try* intrinsics only"),
        }

        self.emit_numeric_overflow_flag(buf, intrinsic, bits, value_ty, result_local)?;
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        if let Some(out_place) = &intrinsic.out {
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::If);
            emit_instruction(buf, Op::LocalGet(result_local));
            self.store_value_into_place(buf, out_place, value_ty)?;
            emit_instruction(buf, Op::End);
        }

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    fn emit_numeric_overflow_flag(
        &mut self,
        buf: &mut Vec<u8>,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
        result_local: u32,
    ) -> Result<(), Error> {
        match intrinsic.kind {
            NumericIntrinsicKind::TryAdd => {
                if intrinsic.signed {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor));
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
                    emit_instruction(
                        buf,
                        self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
                    );
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtS, Op::I64LtS));
                } else {
                    emit_instruction(buf, Op::LocalGet(result_local));
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        false,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtU, Op::I64LtU));
                }
            }
            NumericIntrinsicKind::TrySub => {
                if intrinsic.signed {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor)); // lhs ^ rhs
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor)); // lhs ^ result
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
                    emit_instruction(
                        buf,
                        self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
                    );
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtS, Op::I64LtS));
                } else {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        false,
                    )?;
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        false,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtU, Op::I64LtU));
                }
            }
            NumericIntrinsicKind::TryNeg => {
                let min_value = min_int_value(bits);
                self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I32Const(min_value as i32)),
                    ValueType::I64 => emit_instruction(buf, Op::I64Const(min_value)),
                    _ => unreachable!(),
                }
                emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eq, Op::I64Eq));
            }
            NumericIntrinsicKind::TryMul => {
                self.emit_numeric_mul_overflow_flag(buf, intrinsic, bits, value_ty, result_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unexpected numeric intrinsic for overflow flag".into(),
                ));
            }
        }
        Ok(())
    }

    fn emit_numeric_mul_overflow_flag(
        &mut self,
        buf: &mut Vec<u8>,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
        result_local: u32,
    ) -> Result<(), Error> {
        if bits < 64 {
            self.emit_widened_operand(buf, &intrinsic.operands[0], bits, intrinsic.signed)?;
            self.emit_widened_operand(buf, &intrinsic.operands[1], bits, intrinsic.signed)?;
            emit_instruction(buf, Op::I64Mul);
            emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

            if intrinsic.signed {
                let min = min_int_value(bits);
                let max = max_int_value(bits);
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(min));
                emit_instruction(buf, Op::I64LtS);
                emit_instruction(buf, Op::LocalSet(self.block_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(max));
                emit_instruction(buf, Op::I64GtS);
                emit_instruction(buf, Op::LocalSet(self.temp_local));

                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Or);
            } else {
                let max = max_uint_value(bits);
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(max as i64));
                emit_instruction(buf, Op::I64GtU);
            }
            return Ok(());
        }

        // 64-bit path: guard divide-by-zero and MIN * -1 (signed).
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[0],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[1],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        emit_instruction(buf, Op::I32Or);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::Else);
        if intrinsic.signed {
            let min = min_int_value(bits);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Const(min));
            emit_instruction(buf, Op::I64Eq);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Const(-1));
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32And);
            emit_instruction(buf, Op::If);
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::Else);
            emit_instruction(buf, Op::LocalGet(result_local));
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64DivS);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32Eqz);
            emit_instruction(buf, Op::End);
            emit_instruction(buf, Op::End);
        } else {
            emit_instruction(buf, Op::LocalGet(result_local));
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, false)?;
            emit_instruction(buf, Op::I64DivU);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32Eqz);
            emit_instruction(buf, Op::End);
        }
        Ok(())
    }

    fn emit_numeric_count_intrinsic(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        let op = match (intrinsic.kind, value_ty) {
            (NumericIntrinsicKind::LeadingZeroCount, ValueType::I32) => Op::I32Clz,
            (NumericIntrinsicKind::LeadingZeroCount, ValueType::I64) => Op::I64Clz,
            (NumericIntrinsicKind::TrailingZeroCount, ValueType::I32) => Op::I32Ctz,
            (NumericIntrinsicKind::TrailingZeroCount, ValueType::I64) => Op::I64Ctz,
            (NumericIntrinsicKind::PopCount, ValueType::I32) => Op::I32Popcnt,
            (NumericIntrinsicKind::PopCount, ValueType::I64) => Op::I64Popcnt,
            _ => {
                return Err(Error::Codegen(format!(
                    "unsupported numeric intrinsic {:?} in WASM backend",
                    intrinsic.kind
                )));
            }
        };
        emit_instruction(buf, op);
        let type_bits = match value_ty {
            ValueType::I64 => 64,
            _ => 32,
        };
        if bits < type_bits
            && matches!(
                intrinsic.kind,
                NumericIntrinsicKind::LeadingZeroCount | NumericIntrinsicKind::TrailingZeroCount
            )
        {
            emit_instruction(buf, Op::I32Const((type_bits - bits) as i32));
            emit_instruction(buf, Op::I32Sub);
        }
        if matches!(value_ty, ValueType::I64) {
            emit_instruction(buf, Op::I32WrapI64);
        }
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    fn emit_numeric_rotate_intrinsic(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        if value_ty == ValueType::I64 && bits != 64 {
            return Err(Error::Codegen(
                "64-bit rotate with non-64 bit width is not supported in WASM backend".into(),
            ));
        }
        if matches!(value_ty, ValueType::I32) && bits < 32 {
            self.emit_small_rotate(buf, place, intrinsic, bits)?;
            return Ok(());
        }

        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        self.emit_numeric_operand_as(buf, &intrinsic.operands[1], ValueType::I32, bits, false)?;
        let mask = (bits - 1) as i32;
        emit_instruction(buf, Op::I32Const(mask));
        emit_instruction(buf, Op::I32And);
        let op = match (intrinsic.kind, value_ty) {
            (NumericIntrinsicKind::RotateLeft, ValueType::I32) => Op::I32Rotl,
            (NumericIntrinsicKind::RotateRight, ValueType::I32) => Op::I32Rotr,
            (NumericIntrinsicKind::RotateLeft, ValueType::I64) => Op::I64Rotl,
            (NumericIntrinsicKind::RotateRight, ValueType::I64) => Op::I64Rotr,
            _ => unreachable!(),
        };
        emit_instruction(buf, op);
        self.canonicalize_int_value(buf, value_ty, bits, false);
        self.store_value_into_place(buf, place, value_ty)?;
        Ok(())
    }

    fn emit_small_rotate(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], ValueType::I32, bits, false)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        self.emit_numeric_operand_as(buf, &intrinsic.operands[1], ValueType::I32, bits, false)?;
        emit_instruction(buf, Op::I32Const((bits - 1) as i32));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        match intrinsic.kind {
            NumericIntrinsicKind::RotateLeft => emit_instruction(buf, Op::I32Shl),
            NumericIntrinsicKind::RotateRight => emit_instruction(buf, Op::I32ShrU),
            _ => unreachable!(),
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Const(bits as i32));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Sub);
        match intrinsic.kind {
            NumericIntrinsicKind::RotateLeft => emit_instruction(buf, Op::I32ShrU),
            NumericIntrinsicKind::RotateRight => emit_instruction(buf, Op::I32Shl),
            _ => unreachable!(),
        }

        emit_instruction(buf, Op::I32Or);
        self.canonicalize_int_value(buf, ValueType::I32, bits, false);
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    fn emit_numeric_reverse_endianness(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        match bits {
            8 => {}
            16 => {
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Or);
                self.canonicalize_int_value(buf, ValueType::I32, bits, false);
            }
            32 => {
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(24));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Const(0xFF));
                emit_instruction(buf, Op::I32And);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Const(0xFF00));
                emit_instruction(buf, Op::I32And);
                emit_instruction(buf, Op::I32Or);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::I32Const(0x00FF0000));
                emit_instruction(buf, Op::I32And);
                emit_instruction(buf, Op::I32Or);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(24));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::I32Or);
            }
            64 => {
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                // byte 0 -> byte 7
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(56));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF));
                emit_instruction(buf, Op::I64And);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(40));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF00));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(24));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF0000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(8));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(8));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF00000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(24));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF0000000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(40));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF000000000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(56));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Or);
            }
            _ => {
                return Err(Error::Codegen(format!(
                    "unsupported reverse endianness width {}",
                    bits
                )));
            }
        }
        self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
        self.store_value_into_place(buf, place, value_ty)?;
        Ok(())
    }

    fn emit_numeric_is_power_of_two(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[0],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        let value_local = if value_ty == ValueType::I64 {
            self.wide_temp_local
        } else {
            self.temp_local
        };
        emit_instruction(buf, Op::LocalTee(value_local));

        // value != 0 (and > 0 for signed)
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        if intrinsic.signed {
            emit_instruction(buf, Op::LocalGet(value_local));
            emit_instruction(
                buf,
                self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
            );
            emit_instruction(buf, self.op_for_int(value_ty, Op::I32GtS, Op::I64GtS));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }

        emit_instruction(buf, Op::LocalGet(value_local));
        emit_instruction(
            buf,
            self.op_for_int(value_ty, Op::I32Const(1), Op::I64Const(1)),
        );
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub));
        self.canonicalize_int_value(buf, value_ty, bits, false);
        emit_instruction(buf, Op::LocalGet(value_local));
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32And);
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    fn emit_numeric_operand_as(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        value_ty: ValueType,
        bits: u32,
        signed: bool,
    ) -> Result<(), Error> {
        let operand_ty = self.emit_operand(buf, operand)?;
        match (operand_ty, value_ty) {
            (ValueType::I32, ValueType::I64) => emit_instruction(
                buf,
                if signed {
                    Op::I64ExtendI32S
                } else {
                    Op::I64ExtendI32U
                },
            ),
            (ValueType::I64, ValueType::I32) => emit_instruction(buf, Op::I32WrapI64),
            _ => {}
        }
        self.canonicalize_int_value(buf, value_ty, bits, signed);
        Ok(())
    }

    fn emit_widened_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        bits: u32,
        signed: bool,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, operand, ValueType::I64, bits, signed)
    }

    fn canonicalize_int_value(
        &self,
        buf: &mut Vec<u8>,
        value_ty: ValueType,
        bits: u32,
        signed: bool,
    ) {
        if bits == 0
            || bits
                >= match value_ty {
                    ValueType::I64 => 64,
                    _ => 32,
                }
        {
            return;
        }
        match value_ty {
            ValueType::I64 => {
                if signed {
                    let shift = 64 - bits;
                    emit_instruction(buf, Op::I64Const(shift as i64));
                    emit_instruction(buf, Op::I64Shl);
                    emit_instruction(buf, Op::I64Const(shift as i64));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    let mask = numeric_mask(bits);
                    emit_instruction(buf, Op::I64Const(mask as i64));
                    emit_instruction(buf, Op::I64And);
                }
            }
            _ => {
                if signed {
                    let shift = 32 - bits;
                    emit_instruction(buf, Op::I32Const(shift as i32));
                    emit_instruction(buf, Op::I32Shl);
                    emit_instruction(buf, Op::I32Const(shift as i32));
                    emit_instruction(buf, Op::I32ShrS);
                } else {
                    let mask = (numeric_mask(bits) & 0xFFFF_FFFF) as i32;
                    emit_instruction(buf, Op::I32Const(mask));
                    emit_instruction(buf, Op::I32And);
                }
            }
        }
    }

    fn op_for_int(&self, value_ty: ValueType, i32_op: Op, i64_op: Op) -> Op {
        match value_ty {
            ValueType::I64 => i64_op,
            _ => i32_op,
        }
    }
}

fn numeric_width_bits(width: NumericWidth) -> u32 {
    match width {
        NumericWidth::W8 => 8,
        NumericWidth::W16 => 16,
        NumericWidth::W32 => 32,
        NumericWidth::W64 => 64,
        NumericWidth::W128 => 128,
        NumericWidth::Pointer => 32,
    }
}

#[derive(Clone, Copy, Debug)]
struct SpanOffsets {
    data_ptr: u32,
    data_size: u32,
    data_align: u32,
    len: u32,
    elem_size: u32,
    elem_align: u32,
}

fn numeric_value_ty(width: NumericWidth) -> ValueType {
    match width {
        NumericWidth::W64 => ValueType::I64,
        NumericWidth::W128 => ValueType::I64,
        NumericWidth::Pointer => ValueType::I32,
        _ => ValueType::I32,
    }
}

fn numeric_mask(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u128 << bits) as u64 - 1
    }
}

fn min_int_value(bits: u32) -> i64 {
    if bits == 64 {
        i64::MIN
    } else {
        -(1i64 << (bits - 1))
    }
}

fn max_int_value(bits: u32) -> i64 {
    if bits == 64 {
        i64::MAX
    } else {
        (1i64 << (bits - 1)) - 1
    }
}

fn max_uint_value(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        numeric_mask(bits)
    }
}
