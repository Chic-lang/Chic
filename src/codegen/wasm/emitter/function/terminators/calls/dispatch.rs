use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_trait_object_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
        dispatch: &TraitObjectDispatch,
    ) -> Result<(), Error> {
        let receiver = call.args.get(dispatch.receiver_index).ok_or_else(|| {
            Error::Codegen("trait object call is missing its receiver argument".into())
        })?;
        let place = match receiver {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(
                    "trait object receiver must be addressable in WASM backend".into(),
                ));
            }
        };
        let trait_label = dispatch
            .trait_name
            .rsplit("::")
            .next()
            .unwrap_or(dispatch.trait_name.as_str());
        let lookup_slot = |impl_type: &str| {
            self.trait_vtables
                .iter()
                .find(|table| {
                    Self::names_equivalent(&table.trait_name, &dispatch.trait_name)
                        && Self::names_equivalent(&table.impl_type, impl_type)
                })
                .and_then(|table| table.slots.get(dispatch.slot_index as usize))
                .map(|slot| slot.symbol.clone())
        };
        let lookup_method_symbol = |impl_type: &str| {
            let candidates = [
                format!("{impl_type}::{trait_label}::{}", dispatch.method),
                format!("{impl_type}::{}", dispatch.method),
            ];
            for candidate in candidates {
                let canonical = canonical_symbol_name(&candidate);
                if self.function_signatures.contains_key(&canonical)
                    || self.functions.contains_key(&canonical)
                {
                    return Some(candidate);
                }
            }
            None
        };
        let resolve_direct_symbol =
            |impl_type: &str| lookup_slot(impl_type).or_else(|| lookup_method_symbol(impl_type));
        let mut direct_symbol = dispatch
            .impl_type
            .as_deref()
            .and_then(|impl_type| resolve_direct_symbol(impl_type));
        let mut receiver_ty = None;
        if direct_symbol.is_none() && dispatch.impl_type.is_some() {
            if let Ok(place_ty) = self.mir_place_ty(place) {
                let resolved = self.resolve_self_ty(&place_ty);
                receiver_ty = Some(resolved.clone());
                let mut core_ty = &resolved;
                loop {
                    match core_ty {
                        Ty::Pointer(ptr) => core_ty = &ptr.element,
                        Ty::Ref(reference) => core_ty = &reference.element,
                        Ty::Nullable(inner) => core_ty = inner.as_ref(),
                        _ => break,
                    }
                }
                if !matches!(core_ty, Ty::TraitObject(_)) {
                    direct_symbol = resolve_direct_symbol(&core_ty.canonical_name());
                }
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_TRAIT_CALL").is_some() {
            eprintln!(
                "[wasm-trait-call] func={} trait={} method={} impl_hint={:?} receiver_ty={:?} direct_symbol={:?}",
                self.function.name,
                dispatch.trait_name,
                dispatch.method,
                dispatch.impl_type,
                receiver_ty.as_ref().map(|ty: &Ty| ty.canonical_name()),
                direct_symbol
            );
        }
        if let Some(symbol) = direct_symbol {
            let direct = Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)));
            let direct_call = CallLowering {
                func: &direct,
                args: call.args,
                modes: call.modes,
                destination: call.destination,
                target: call.target,
                unwind: call.unwind,
                dispatch: None,
            };
            return self.emit_direct_call(buf, direct_call);
        }
        if self.function.name.contains("ThreadFunctionRunner::Run") {
            eprintln!(
                "[wasm-trait-call] func={} trait={} method={} slot_index={} impl_hint={:?}",
                self.function.name,
                dispatch.trait_name,
                dispatch.method,
                dispatch.slot_index,
                dispatch.impl_type
            );
        }
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalTee(self.block_local));
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let mut vtable_offset: Option<u32> = None;
        if let Some(impl_type) = dispatch.impl_type.as_deref()
            && impl_type != dispatch.trait_name
        {
            let tables_for_trait = self
                .trait_vtables
                .iter()
                .filter(|table| Self::names_equivalent(&table.trait_name, &dispatch.trait_name))
                .collect::<Vec<_>>();
            let selected = tables_for_trait
                .iter()
                .copied()
                .find(|table| Self::names_equivalent(&table.impl_type, impl_type))
                .or_else(|| (tables_for_trait.len() == 1).then(|| tables_for_trait[0]));
            if let Some(table) = selected {
                vtable_offset = self.trait_vtable_offsets.get(&table.symbol).copied();
            }
        }
        if let Some(offset) = vtable_offset {
            emit_instruction(
                buf,
                Op::I32Const(i32::try_from(offset).map_err(|_| {
                    Error::Codegen("trait vtable offset exceeds i32 range in WASM backend".into())
                })?),
            );
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            // Fallback to dyn trait object layout: load the vtable pointer from the second
            // word of the `{ data_ptr, vtable_ptr }` pair.
            emit_instruction(buf, Op::LocalGet(self.block_local));
            emit_instruction(buf, Op::I32Const(4));
            emit_instruction(buf, Op::I32Add);
            emit_instruction(buf, Op::I32Load(0));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }
        let slot_offset = dispatch
            .slot_index
            .checked_mul(4)
            .ok_or_else(|| Error::Codegen("trait vtable slot offset overflow".into()))?;
        let slot_offset = i32::try_from(slot_offset).map_err(|_| {
            Error::Codegen("trait vtable slot offset exceeds addressable range".into())
        })?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32Const(slot_offset));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.block_local));

        if dispatch.trait_name.contains("ThreadStart") && dispatch.method == "Run" {
            if let Some(run_index) =
                self.lookup_function_index("Std::Platform::Thread::ThreadFunctionStartAdapter::Run")
            {
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::If);
                emit_instruction(
                    buf,
                    Op::I32Const(i32::try_from(run_index).unwrap_or_default()),
                );
                emit_instruction(buf, Op::LocalSet(self.block_local));
                emit_instruction(buf, Op::End);
            }
        }

        let symbol = self.trait_vtable_slot_symbol(dispatch)?;
        let signature = self.function_signatures.get(symbol).ok_or_else(|| {
            Error::Codegen(format!(
                "missing WebAssembly signature metadata for trait method `{symbol}`"
            ))
        })?;
        let type_index = *self.signature_indices.get(signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function signature for `{symbol}` is not registered in the WASM type table"
            ))
        })?;

        let signature_requires_sret = signature.params.len() == call.args.len() + 1
            && matches!(signature.params.first(), Some(ValueType::I32))
            && signature.results.len() == 1
            && signature.results[0] == ValueType::I32;
        let ret_is_sret =
            signature_requires_sret || self.call_destination_requires_sret(call.destination)?;

        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?
                .or_else(|| self.function_return_tys.get(symbol).cloned());
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            if index == dispatch.receiver_index {
                emit_instruction(buf, Op::LocalGet(self.temp_local));
            } else {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
        }

        emit_instruction(buf, Op::LocalGet(self.block_local));
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

    fn trait_vtable_slot_symbol(&self, dispatch: &TraitObjectDispatch) -> Result<&str, Error> {
        let table = self
            .trait_vtables
            .iter()
            .find(|table| table.trait_name == dispatch.trait_name)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "trait `{}` does not have vtable metadata in this module",
                    dispatch.trait_name
                ))
            })?;
        let slot = table
            .slots
            .get(dispatch.slot_index as usize)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "trait `{}` vtable is missing slot {}",
                    dispatch.trait_name, dispatch.slot_index
                ))
            })?;
        Ok(slot.symbol.as_str())
    }

    pub(super) fn emit_virtual_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
        dispatch: &VirtualDispatch,
    ) -> Result<(), Error> {
        let receiver = call.args.get(dispatch.receiver_index).ok_or_else(|| {
            Error::Codegen("virtual dispatch is missing its receiver argument".into())
        })?;
        match receiver {
            Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_) => {}
            _ => {
                return Err(Error::Codegen(
                    "virtual dispatch receiver must be addressable in WASM backend".into(),
                ));
            }
        };

        let receiver_ty = self.emit_operand(buf, receiver)?;
        if receiver_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "virtual dispatch receiver must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        if let Some(owner) = dispatch.base_owner.as_deref() {
            let table = self
                .class_vtables
                .iter()
                .find(|table| table.type_name == owner)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "class `{owner}` is not defined in this module; base dispatch cannot resolve its vtable"
                    ))
                })?;
            let offset = self
                .class_vtable_offsets
                .get(&table.symbol)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "class `{owner}` does not have vtable data in this module"
                    ))
                })?;
            emit_instruction(
                buf,
                Op::I32Const(i32::try_from(*offset).map_err(|_| {
                    Error::Codegen("class vtable offset exceeds i32 range in WASM backend".into())
                })?),
            );
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            emit_instruction(buf, Op::I32Load(0));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }

        let slot_offset_bytes = dispatch
            .slot_index
            .checked_mul(4)
            .ok_or_else(|| Error::Codegen("virtual dispatch slot offset overflow".into()))?;
        let slot_offset = i32::try_from(slot_offset_bytes)
            .map_err(|_| Error::Codegen("virtual dispatch slot offset exceeds i32 range".into()))?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32Const(slot_offset));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.block_local));

        let ret_is_sret = self.call_destination_requires_sret(call.destination)?;
        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?;
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            if index == dispatch.receiver_index {
                emit_instruction(buf, Op::LocalGet(self.temp_local));
            } else {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
        }

        let callee = match call.func {
            Operand::Const(constant) => match constant.value() {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => {
                    return Err(Error::Codegen(
                        "virtual dispatch requires a symbol operand for its callee".into(),
                    ));
                }
            },
            _ => {
                return Err(Error::Codegen(
                    "virtual dispatch requires a constant callee operand in WASM backend".into(),
                ));
            }
        };

        let signature = self.function_signatures.get(&callee).ok_or_else(|| {
            Error::Codegen(format!(
                "missing WebAssembly signature metadata for virtual call `{callee}`"
            ))
        })?;
        let type_index = *self.signature_indices.get(signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function signature `{callee}` is not registered in the WASM type table"
            ))
        })?;

        emit_instruction(buf, Op::LocalGet(self.block_local));
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
}
