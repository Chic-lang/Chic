use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_indirect_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
        fn_ty: FnTy,
    ) -> Result<(), Error> {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            return self.emit_extern_indirect_call(buf, call, fn_ty);
        }
        let signature = FunctionSignature::from_fn_ty(&fn_ty, self.layouts);
        let type_index = *self.signature_indices.get(&signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer signature `{}` is not registered in WASM type table",
                fn_ty.canonical_name()
            ))
        })?;
        let ret_is_sret = self.ty_requires_sret(fn_ty.ret.as_ref());
        if call.args.len() != fn_ty.params.len() {
            return Err(Error::Codegen(format!(
                "function pointer call expected {} argument(s) but found {}",
                fn_ty.params.len(),
                call.args.len()
            )));
        }

        if let Some(operand_ty) = self.operand_ty(&call.func) {
            let (trait_ty, via_pointer) = match &operand_ty {
                Ty::Pointer(inner) => (inner.element.clone(), true),
                Ty::Ref(inner) => (inner.element.clone(), true),
                Ty::Nullable(inner) => match inner.as_ref() {
                    Ty::Pointer(ptr) => (ptr.element.clone(), true),
                    Ty::Ref(r) => (r.element.clone(), true),
                    other => (other.clone(), false),
                },
                other => (other.clone(), false),
            };
            if self.ty_is_trait_object_like(&trait_ty) {
                let layout = self.lookup_struct_layout(&trait_ty).cloned();
                let (context_offset, vtable_offset) = layout
                    .as_ref()
                    .and_then(|layout| {
                        let context_offset = layout
                            .fields
                            .iter()
                            .find_map(|field| field.offset.filter(|off| *off == 0));
                        let vtable_offset = layout
                            .fields
                            .iter()
                            .find_map(|field| {
                                if field.name.contains("vtable") {
                                    field.offset
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off != 0))
                            });
                        Some((
                            context_offset.unwrap_or(0) as u32,
                            vtable_offset.unwrap_or(4) as u32,
                        ))
                    })
                    .unwrap_or((0, 4));
                let context_offset = ensure_u32(
                    context_offset as usize,
                    "trait object context offset exceeds wasm32 range",
                )?;
                let vtable_offset = ensure_u32(
                    vtable_offset as usize,
                    "trait object vtable offset exceeds wasm32 range",
                )?;

                let base_ptr = match &call.func {
                    Operand::Copy(place) | Operand::Move(place) => {
                        if via_pointer {
                            let ptr_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                            Self::ensure_operand_type(
                                ptr_ty,
                                ValueType::I32,
                                "trait object pointer",
                            )?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else if let Ok(access) = self.resolve_memory_access(place) {
                            self.emit_pointer_expression(buf, &access)?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else if matches!(
                            self.representations.get(place.local.0),
                            Some(
                                LocalRepresentation::PointerParam
                                    | LocalRepresentation::FrameAllocated
                                    | LocalRepresentation::Scalar
                            )
                        ) {
                            let ptr_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                            Self::ensure_operand_type(
                                ptr_ty,
                                ValueType::I32,
                                "trait object pointer",
                            )?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else {
                            None
                        }
                    }
                    Operand::Borrow(borrow) => {
                        if let Ok(access) = self.resolve_memory_access(&borrow.place) {
                            self.emit_pointer_expression(buf, &access)?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(base_local) = base_ptr {
                    emit_instruction(buf, Op::LocalGet(base_local));
                    if context_offset != 0 {
                        emit_instruction(buf, Op::I32Const(context_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

                    emit_instruction(buf, Op::LocalGet(base_local));
                    if vtable_offset != 0 {
                        emit_instruction(buf, Op::I32Const(vtable_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Eqz);
                    emit_instruction(buf, Op::If);
                    Self::emit_trap(buf);
                    emit_instruction(buf, Op::End);

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    if ret_is_sret {
                        self.emit_sret_out_pointer(
                            buf,
                            call.destination,
                            Some(fn_ty.ret.as_ref()),
                        )?;
                    }
                    for (index, arg) in call.args.iter().enumerate() {
                        let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                        self.emit_call_argument_for_mode(buf, arg, mode)?;
                    }
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(
                        buf,
                        Op::CallIndirect {
                            type_index,
                            table_index: 0,
                        },
                    );
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if ret_is_sret {
                        emit_instruction(buf, Op::Drop);
                    } else if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
            }
        }

        if let Operand::Copy(place) | Operand::Move(place) = call.func {
            if matches!(
                self.representations.get(place.local.0),
                Some(LocalRepresentation::Scalar)
            ) {
                let invoke_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                Self::ensure_operand_type(invoke_ty, ValueType::I32, "fn invoke value")?;
                emit_instruction(buf, Op::LocalTee(self.temp_local));
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::If);
                Self::emit_trap(buf);
                emit_instruction(buf, Op::End);
                emit_instruction(buf, Op::I32Const(0));
                if ret_is_sret {
                    self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
                }
                for (index, arg) in call.args.iter().enumerate() {
                    let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                    self.emit_call_argument_for_mode(buf, arg, mode)?;
                }
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(
                    buf,
                    Op::CallIndirect {
                        type_index,
                        table_index: 0,
                    },
                );
                self.release_call_borrows(buf, call.args, call.modes)?;
                if ret_is_sret {
                    emit_instruction(buf, Op::Drop);
                } else if let Some(place) = call.destination {
                    self.store_call_result(buf, place)?;
                }
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        if let Some(layout) = self.lookup_struct_layout(&Ty::Fn(fn_ty.clone())).cloned() {
            let invoke_offset = layout
                .fields
                .iter()
                .find_map(|field| {
                    Self::fn_field_key(&field.name)
                        .filter(|key| *key == "invoke")
                        .and_then(|_| field.offset)
                })
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "function pointer layout `{}` missing invoke field for WASM lowering",
                        layout.name
                    ))
                })?;
            let context_offset = layout
                .fields
                .iter()
                .find_map(|field| {
                    Self::fn_field_key(&field.name)
                        .filter(|key| *key == "context")
                        .and_then(|_| field.offset)
                })
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "function pointer layout `{}` missing context field for WASM lowering",
                        layout.name
                    ))
                })?;
            let invoke_offset = ensure_u32(
                invoke_offset,
                "function pointer invoke offset exceeds wasm32 range",
            )?;
            let context_offset = ensure_u32(
                context_offset,
                "function pointer context offset exceeds wasm32 range",
            )?;

            match &call.func {
                Operand::Copy(place) | Operand::Move(place) => {
                    if !matches!(
                        self.representations.get(place.local.0),
                        Some(LocalRepresentation::Scalar)
                    ) {
                        let access = self.resolve_memory_access(place)?;
                        self.emit_pointer_expression(buf, &access)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalTee(self.temp_local));
                        emit_instruction(buf, Op::I32Eqz);
                        emit_instruction(buf, Op::If);
                        Self::emit_trap(buf);
                        emit_instruction(buf, Op::End);

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if context_offset != 0 {
                            emit_instruction(buf, Op::I32Const(context_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        if ret_is_sret {
                            self.emit_sret_out_pointer(
                                buf,
                                call.destination,
                                Some(fn_ty.ret.as_ref()),
                            )?;
                        }
                        for (index, arg) in call.args.iter().enumerate() {
                            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                            self.emit_call_argument_for_mode(buf, arg, mode)?;
                        }
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(
                            buf,
                            Op::CallIndirect {
                                type_index,
                                table_index: 0,
                            },
                        );
                        self.release_call_borrows(buf, call.args, call.modes)?;
                        if ret_is_sret {
                            emit_instruction(buf, Op::Drop);
                        } else if let Some(place) = call.destination {
                            self.store_call_result(buf, place)?;
                        }
                        self.emit_goto(buf, call.target);
                        return Ok(());
                    }
                }
                Operand::Borrow(borrow) => {
                    let access = self.resolve_memory_access(&borrow.place)?;
                    self.emit_pointer_expression(buf, &access)?;
                    emit_instruction(buf, Op::LocalSet(self.block_local));

                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    if invoke_offset != 0 {
                        emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Eqz);
                    emit_instruction(buf, Op::If);
                    Self::emit_trap(buf);
                    emit_instruction(buf, Op::End);

                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    if context_offset != 0 {
                        emit_instruction(buf, Op::I32Const(context_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    if ret_is_sret {
                        self.emit_sret_out_pointer(
                            buf,
                            call.destination,
                            Some(fn_ty.ret.as_ref()),
                        )?;
                    }
                    for (index, arg) in call.args.iter().enumerate() {
                        let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                        self.emit_call_argument_for_mode(buf, arg, mode)?;
                    }
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(
                        buf,
                        Op::CallIndirect {
                            type_index,
                            table_index: 0,
                        },
                    );
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if ret_is_sret {
                        emit_instruction(buf, Op::Drop);
                    } else if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
                _ => {}
            }
        }
        let mut base_place = match call.func {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => {
                return Err(Error::Codegen(
                    "function pointer operand must be place-backed in WASM backend".into(),
                ));
            }
        };

        let mut invoke_place = base_place.clone();
        invoke_place
            .projection
            .push(crate::mir::ProjectionElem::FieldNamed("invoke".into()));
        let invoke_operand = Operand::Copy(invoke_place);

        base_place
            .projection
            .push(crate::mir::ProjectionElem::FieldNamed("context".into()));
        let context_operand = Operand::Copy(base_place);

        self.emit_operand(buf, &context_operand)?;
        if ret_is_sret {
            self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let invoke_ty = self.emit_operand(buf, &invoke_operand)?;
        if invoke_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "function pointer `invoke` field must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalTee(self.temp_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        Self::emit_trap(buf);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_extern_indirect_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
        fn_ty: FnTy,
    ) -> Result<(), Error> {
        let signature = FunctionSignature::from_fn_ty(&fn_ty, self.layouts);
        let type_index = *self.signature_indices.get(&signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer signature `{}` is not registered in WASM type table",
                fn_ty.canonical_name()
            ))
        })?;
        let ret_is_sret = self.ty_requires_sret(fn_ty.ret.as_ref());
        if call.args.len() != fn_ty.params.len() {
            return Err(Error::Codegen(format!(
                "function pointer call expected {} argument(s) but found {}",
                fn_ty.params.len(),
                call.args.len()
            )));
        }

        if ret_is_sret {
            self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let fn_ptr_ty = self.emit_extern_fn_ptr_value(buf, call.func)?;
        Self::ensure_operand_type(fn_ptr_ty, ValueType::I32, "extern fn pointer value")?;
        emit_instruction(buf, Op::LocalTee(self.temp_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        Self::emit_trap(buf);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_extern_fn_ptr_value(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    return self.emit_load_from_place(buf, place);
                }
                self.emit_operand(buf, operand)
            }
            Operand::Borrow(borrow) => {
                let access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::I32Load(0));
                Ok(ValueType::I32)
            }
            _ => self.emit_operand(buf, operand),
        }
    }

    pub(super) fn emit_fn_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        fn_ty: &FnTy,
    ) -> Result<(), Error> {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            let ptr_ty = self.emit_extern_fn_ptr_value(buf, operand)?;
            Self::ensure_operand_type(ptr_ty, ValueType::I32, "extern fn pointer argument")?;
            return Ok(());
        }
        let fn_ty_wrapped = Ty::Fn(fn_ty.clone());
        let layout = self
            .lookup_struct_layout(&fn_ty_wrapped)
            .cloned()
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing function pointer layout for `{}` in WASM backend",
                    fn_ty_wrapped.canonical_name()
                ))
            })?;
        let allocation =
            compute_aggregate_allocation(&fn_ty_wrapped, self.layouts).ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer layout missing allocation metadata for `{}` in WASM backend",
                    fn_ty_wrapped.canonical_name()
                ))
            })?;
        self.allocate_stack_block(buf, allocation.size, allocation.align)?;

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        match operand {
            Operand::Const(constant) => {
                self.initialise_fn_from_const(buf, &layout, &fn_ty_wrapped, constant)?;
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                    eprintln!(
                        "[wasm-fn-arg] func={} operand_local={} repr={:?}",
                        self.function.name,
                        place.local.0,
                        self.representations.get(place.local.0),
                    );
                }
                if let Ok(src_access) = self.resolve_memory_access(place) {
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(
                            buf,
                            &layout,
                            &fn_ty_wrapped,
                            self.block_local,
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(
                            buf,
                            &layout,
                            &fn_ty_wrapped,
                            self.block_local,
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                }
                let ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(ty, ValueType::I32, "fn invoke argument")?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.initialise_fn_from_invoke(buf, &layout, &fn_ty_wrapped, self.block_local)?;
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported function argument operand for WASM lowering".into(),
                ));
            }
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        Ok(())
    }

    #[allow(dead_code)]
    fn emit_fn_like_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        ty: &Ty,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        match ty {
            Ty::Fn(fn_ty) => return self.emit_fn_argument(buf, operand, fn_ty),
            _ => {}
        }

        // Handle pointer/ref/nullable-to-fn layouts by copying from the pointed-to memory.
        let (base_ty, via_pointer) = match ty {
            Ty::Pointer(inner) => (inner.element.clone(), true),
            Ty::Ref(inner) => (inner.element.clone(), true),
            Ty::Nullable(inner) => match inner.as_ref() {
                Ty::Pointer(ptr) => (ptr.element.clone(), true),
                Ty::Ref(r) => (r.element.clone(), true),
                other => (other.clone(), false),
            },
            other => (other.clone(), false),
        };
        if via_pointer {
            if let Some(base_layout) = self
                .lookup_struct_layout(&base_ty)
                .cloned()
                .filter(|l| Self::is_fn_pointer_layout(l))
            {
                let allocation =
                    compute_aggregate_allocation(&base_ty, self.layouts).ok_or_else(|| {
                        Error::Codegen(format!(
                            "function pointer layout missing allocation metadata for `{}` in WASM backend",
                            base_ty.canonical_name()
                        ))
                    })?;
                self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                match operand {
                    Operand::Copy(_place) | Operand::Move(_place) => {
                        let pointer_ty = self.emit_operand(buf, operand)?;
                        Self::ensure_operand_type(
                            pointer_ty,
                            ValueType::I32,
                            "fn pointer argument",
                        )?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(
                            buf,
                            &base_layout,
                            self.stack_temp_local,
                            self.temp_local,
                        )?;
                    }
                    Operand::Borrow(borrow) => {
                        let src_access = self.resolve_memory_access(&borrow.place)?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(
                            buf,
                            &base_layout,
                            self.stack_temp_local,
                            self.temp_local,
                        )?;
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported function pointer argument operand for WASM lowering"
                                .into(),
                        ));
                    }
                }
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(());
            }
        }

        let allocation = compute_aggregate_allocation(ty, self.layouts).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer layout missing allocation metadata for `{}` in WASM backend",
                ty.canonical_name()
            ))
        })?;
        self.allocate_stack_block(buf, allocation.size, allocation.align)?;

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        match operand {
            Operand::Const(constant) => {
                self.initialise_fn_from_const(buf, layout, ty, constant)?;
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                    eprintln!(
                        "[wasm-fn-like-arg] func={} operand_local={} repr={:?}",
                        self.function.name,
                        place.local.0,
                        self.representations.get(place.local.0),
                    );
                }
                if let Ok(src_access) = self.resolve_memory_access(place) {
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                }
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "fn invoke argument")?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported function argument operand for WASM lowering".into(),
                ));
            }
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        Ok(())
    }

    pub(crate) fn emit_fn_invoke_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        ty: &Ty,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
            eprintln!(
                "[wasm-fn-invoke-arg] func={} operand={:?} ty={} layout={} repr={:?}",
                self.function.name,
                operand,
                ty.canonical_name(),
                layout.name,
                match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        self.representations.get(place.local.0)
                    }
                    _ => None,
                }
            );
        }
        let invoke_offset = layout
            .fields
            .iter()
            .find_map(|field| {
                Self::fn_field_key(&field.name)
                    .filter(|key| *key == "invoke")
                    .and_then(|_| field.offset)
            })
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing invoke field offset for `{}` (type `{}`) in WASM backend",
                    layout.name,
                    ty.canonical_name()
                ))
            })?;
        let invoke_offset = ensure_u32(
            invoke_offset,
            "function pointer invoke offset exceeds wasm32 range",
        )?;
        match operand {
            Operand::Const(constant) => {
                let value = match constant.value() {
                    ConstValue::Symbol(name) => {
                        let index = self.lookup_function_index(name).ok_or_else(|| {
                            Error::Codegen(format!(
                                "unable to resolve function `{name}` for function pointer literal"
                            ))
                        })?;
                        i32::try_from(index).map_err(|_| {
                            Error::Codegen(
                                "function index exceeds i32 range in WASM backend".into(),
                            )
                        })?
                    }
                    ConstValue::Null => 0,
                    other => {
                        return Err(Error::Codegen(format!(
                            "function pointer invoke argument does not support constant operand {other:?}"
                        )));
                    }
                };
                emit_instruction(buf, Op::I32Const(value));
                return Ok(());
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if let Ok(access) = self.resolve_memory_access(place) {
                    self.emit_pointer_expression(buf, &access)?;
                    if invoke_offset != 0 {
                        emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    return Ok(());
                }
                let pointer_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                if invoke_offset != 0 {
                    emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                    emit_instruction(buf, Op::I32Add);
                }
                emit_instruction(buf, Op::I32Load(0));
                return Ok(());
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                if invoke_offset != 0 {
                    emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                    emit_instruction(buf, Op::I32Add);
                }
                emit_instruction(buf, Op::I32Load(0));
                Ok(())
            }
            _ => Err(Error::Codegen(
                "unsupported function invoke argument for WASM lowering".into(),
            )),
        }
    }

    pub(crate) fn allocate_stack_block(
        &mut self,
        buf: &mut Vec<u8>,
        size: u32,
        align: u32,
    ) -> Result<(), Error> {
        let mask = if align > 1 { !((align as i32) - 1) } else { -1 };
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::LocalSet(self.temp_local)); // old SP
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Const(size as i32));
        emit_instruction(buf, Op::I32Sub);
        if align > 1 {
            emit_instruction(buf, Op::I32Const(mask));
            emit_instruction(buf, Op::I32And);
        }
        emit_instruction(buf, Op::LocalTee(self.stack_temp_local));
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));

        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
        Ok(())
    }

    pub(crate) fn initialise_fn_from_invoke(
        &self,
        buf: &mut Vec<u8>,
        layout: &StructLayout,
        fn_ty: &Ty,
        invoke_local: u32,
    ) -> Result<(), Error> {
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
                    emit_instruction(buf, Op::LocalGet(invoke_local));
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
}
