use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_fn_assignment(
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

    pub(super) fn emit_fn_assignment_to_access(
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

    pub(super) fn emit_trait_object_assignment_to_access(
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

    pub(super) fn copy_trait_object(
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
}
