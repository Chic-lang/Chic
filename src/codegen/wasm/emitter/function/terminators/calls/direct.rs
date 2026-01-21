use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_direct_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
    ) -> Result<(), Error> {
        let callee = self.resolve_callee(call.func, call.args)?;
        let mut callee_name = match call.func {
            Operand::Const(constant) => match constant.value() {
                ConstValue::Symbol(name) => Some(canonical_symbol_name(name)),
                _ => None,
            },
            Operand::Pending(pending) => {
                let mut resolved = None;
                if let Some(info) = &pending.info {
                    let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                    for candidate in candidates {
                        if let Some(idx) = self.lookup_function_index(&candidate.qualified) {
                            if idx == callee {
                                resolved = Some(canonical_symbol_name(&candidate.qualified));
                                break;
                            }
                        }
                    }
                }
                if resolved.is_some() {
                    resolved
                } else {
                    let repr = pending.repr.replace('.', "::");
                    if self.lookup_function_index(&repr) == Some(callee) {
                        Some(repr)
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };
        let callee_name_by_index = self
            .functions
            .iter()
            .find_map(|(name, idx)| (*idx == callee).then_some(name.clone()));
        if callee_name.is_none() {
            callee_name = callee_name_by_index.clone();
        }
        let mut expected_params = callee_name.as_ref().and_then(|name| {
            self.function_signatures
                .get(name)
                .map(|sig| sig.params.clone())
        });
        let mut expected_results = callee_name.as_ref().and_then(|name| {
            self.function_signatures
                .get(name)
                .map(|sig| sig.results.clone())
        });
        let mut expected_mir_params = callee_name
            .as_deref()
            .and_then(|name| self.function_param_tys.get(name));
        if expected_params.is_none() && expected_results.is_none() {
            if let Some(name) = callee_name_by_index.clone() {
                if let Some(sig) = self.function_signatures.get(&name) {
                    expected_params = Some(sig.params.clone());
                    expected_results = Some(sig.results.clone());
                    callee_name = Some(name);
                    expected_mir_params = callee_name
                        .as_deref()
                        .and_then(|n| self.function_param_tys.get(n));
                }
            } else if let Some(name) = callee_name
                .as_deref()
                .and_then(|name| name.split('<').next())
            {
                if let Some(sig) = self.function_signatures.get(name) {
                    expected_params = Some(sig.params.clone());
                    expected_results = Some(sig.results.clone());
                    callee_name = Some(name.to_string());
                    expected_mir_params = callee_name
                        .as_deref()
                        .and_then(|n| self.function_param_tys.get(n));
                }
            }
        }
        if let Some(name) = callee_name.as_deref() {
            let tail = name.rsplit("::").next().unwrap_or(name);
            if name == "chic_rt::string_as_slice"
                || tail == "chic_rt_string_as_slice"
                || tail == "string_as_slice"
            {
                let signature = RuntimeHook::StringAsSlice.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if name == "chic_rt::string_as_chars"
                || tail == "chic_rt_string_as_chars"
                || tail == "string_as_chars"
            {
                let signature = RuntimeHook::StringAsChars.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if name == "chic_rt::str_as_chars"
                || tail == "chic_rt_str_as_chars"
                || tail == "str_as_chars"
            {
                let signature = RuntimeHook::StrAsChars.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if expected_results.is_none()
                && (name.contains("AsUtf8Span")
                    || name.contains("AsUtf8")
                    || name.contains("AsSpan"))
            {
                let signature = RuntimeHook::StringAsSlice.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            }
        }
        let callee_return_ty =
            self.resolve_callee_return_ty(call.func, callee_name.as_deref(), callee);
        let mut ret_is_sret = if let Some(ty) = callee_return_ty.as_ref() {
            self.ty_requires_sret(ty)
        } else {
            self.call_destination_requires_sret(call.destination)?
        };
        if std::env::var_os("CHIC_DEBUG_WASM_CALL_SIG").is_some() {
            eprintln!(
                "[wasm-call-sig] caller={} callee={:?} callee_idx={} args={} expected_params={} expected_results={} ret_is_sret={}",
                self.function.name,
                callee_name.as_deref(),
                callee,
                call.args.len(),
                expected_params.as_ref().map(|p| p.len()).unwrap_or(0),
                expected_results.as_ref().map(|r| r.len()).unwrap_or(0),
                ret_is_sret
            );
        }
        let multi_result = expected_results
            .as_ref()
            .map(|results| results.len() > 1)
            .unwrap_or(false);
        if multi_result {
            ret_is_sret = false;
        }
        if let Some(name) = callee_name.as_deref() {
            ensure_std_runtime_intrinsic_owner(name)?;
        }
        if let Some(name) = callee_name.as_deref() {
            if matches!(
                name,
                "Std::Numeric::PointerIntrinsics::AsByteMut"
                    | "Std::Numeric::PointerIntrinsics::AsByteConst"
                    | "Std::Numeric::PointerIntrinsics::AsByteConstFromMut"
            ) {
                if call.args.len() != 1 {
                    return Err(Error::Codegen(format!(
                        "pointer intrinsic `{name}` expects a single argument"
                    )));
                }
                self.emit_operand(buf, &call.args[0])?;
                if let Some(dest) = call.destination {
                    self.store_call_result(buf, dest)?;
                } else {
                    emit_instruction(buf, Op::Drop);
                }
                self.release_call_borrows(buf, call.args, call.modes)?;
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        let is_thread_fn_init = callee == 1466
            || callee_name
                .as_deref()
                .is_some_and(|name| name.contains("ThreadFunctionStartAdapter::init"));
        if callee == 1466 && call.args.len() >= 2 {
            let self_arg = &call.args[0];
            let fn_arg = &call.args[1];
            if let Some(fn_ty) = self.call_operand_fn_ty(fn_arg) {
                self.emit_operand(buf, self_arg)?;
                self.emit_fn_argument(buf, fn_arg, &fn_ty)?;
                emit_instruction(buf, Op::Call(callee));
                self.release_call_borrows(buf, call.args, call.modes)?;
                if let Some(place) = call.destination {
                    self.store_call_result(buf, place)?;
                }
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        if is_thread_fn_init && call.args.len() == 2 {
            // Force a well-formed init call: push the adapter `self` first, then a freshly
            // built fn struct derived from the invoke value of the second argument.
            self.emit_operand(buf, &call.args[0])?;
            let arg = &call.args[1];
            let arg_ty = self.operand_ty(arg).or_else(|| match arg {
                Operand::Copy(place) | Operand::Move(place) => {
                    self.local_tys.get(place.local.0).cloned()
                }
                _ => None,
            });
            if let Some(fn_ty) = arg_ty {
                if let Some(layout) = self
                    .lookup_struct_layout(&fn_ty)
                    .cloned()
                    .filter(Self::is_fn_pointer_layout)
                {
                    let allocation = compute_aggregate_allocation(&fn_ty, self.layouts)
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "function pointer layout missing allocation metadata for `{}` in WASM backend",
                                fn_ty.canonical_name()
                            ))
                        })?;
                    self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                    emit_instruction(buf, Op::LocalSet(self.block_local));
                    self.initialise_fn_from_invoke(buf, &layout, &fn_ty, self.block_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::Call(callee));
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
            }
        }
        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?
                .or_else(|| callee_return_ty.clone());
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        const NULLABLE_UNWRAP_PANIC_CODE: i32 = 0x2010;

        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            if !matches!(mode, ParamMode::Value) {
                self.emit_call_argument_for_mode(buf, arg, mode)?;
                continue;
            }
            let arg_ty = self.operand_ty(arg).or_else(|| match arg {
                Operand::Copy(place) | Operand::Move(place) => {
                    self.local_tys.get(place.local.0).cloned()
                }
                _ => None,
            });
            let expected_param = expected_params
                .as_ref()
                .and_then(|params| params.get(index + if ret_is_sret { 1 } else { 0 }))
                .copied();

            if let Some(expected_param) = expected_param {
                let is_str_operand = matches!(arg_ty.as_ref(), Some(Ty::Str))
                    || matches!(
                        arg,
                        Operand::Const(constant)
                            if matches!(constant.value(), ConstValue::Str { .. })
                    );

                if is_str_operand && expected_param == ValueType::I32 {
                    // Some call sites (notably runtime shims) expect `str` to be passed by pointer to
                    // a `{ ptr: u32, len: u32 }` pair in linear memory. Materialize the packed `i64`
                    // str value into a stack slot and pass the slot pointer.
                    self.allocate_stack_block(buf, 8, 8)?;
                    let value_ty = self.emit_operand(buf, arg)?;
                    Self::ensure_operand_type(value_ty, ValueType::I64, "str argument")?;
                    emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Store(0));

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    continue;
                }

                if expected_param == ValueType::I32 {
                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if map_type(&arg_ty) == ValueType::I64
                            && !local_requires_memory(&arg_ty, self.layouts)
                            && !matches!(arg_ty, Ty::Str)
                        {
                            let value_ty = self.emit_operand(buf, arg)?;
                            Self::ensure_operand_type(value_ty, ValueType::I64, "i64->i32 arg")?;
                            emit_instruction(buf, Op::I32WrapI64);
                            continue;
                        }
                    }
                }

                if let Some(Ty::Nullable(inner)) = arg_ty.as_ref() {
                    let inner_value_ty = map_type(inner.as_ref());
                    let expected_mir_param =
                        expected_mir_params.and_then(|params| params.get(index));
                    let should_unwrap = if let Some(expected_mir_param) = expected_mir_param {
                        let expected_mir_param = self.resolve_self_ty(expected_mir_param);
                        let expected_inner = self.resolve_self_ty(inner.as_ref());
                        inner_value_ty == expected_param
                            && matches!(inner_value_ty, ValueType::I32 | ValueType::I64)
                            && !matches!(expected_mir_param, Ty::Nullable(_))
                            && expected_mir_param == expected_inner
                    } else {
                        // Without MIR parameter types, unwrapping `Nullable(T)` based solely on wasm
                        // value types is ambiguous for `i32` (both the nullable wrapper pointer and
                        // many inner values are `i32`). Only unwrap by value type for `i64`.
                        inner_value_ty == expected_param && inner_value_ty == ValueType::I64
                    };
                    if should_unwrap {
                        if std::env::var_os("CHIC_DEBUG_WASM_NULLABLE_UNWRAP").is_some() {
                            eprintln!(
                                "[wasm-nullable-unwrap] caller={} callee={:?} arg_index={} arg_ty={:?} inner_ty={:?} expected_mir_param={:?} expected_wasm={:?}",
                                self.function.name,
                                callee_name.as_deref(),
                                index,
                                arg_ty,
                                inner.as_ref(),
                                expected_mir_param
                                    .map(|ty| self.resolve_self_ty(ty))
                                    .unwrap_or_else(|| Ty::Unknown),
                                expected_param,
                            );
                        }
                        // Flow typing can treat a nullable value as non-null after a null-check.
                        // The MIR still passes the nullable pointer, so load the inner payload.
                        let ptr_ty = self.emit_operand(buf, arg)?;
                        Self::ensure_operand_type(ptr_ty, ValueType::I32, "nullable unwrap")?;
                        emit_instruction(buf, Op::LocalSet(self.temp_local));

                        let nullable_ty = Ty::Nullable(inner.clone());
                        let (_, has_value_offset) =
                            self.resolve_field_by_name(&nullable_ty, None, "HasValue")?;
                        let (_, value_offset) =
                            self.resolve_field_by_name(&nullable_ty, None, "Value")?;

                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        let has_value_offset = ensure_u32(
                            has_value_offset,
                            "nullable HasValue offset exceeds wasm32 addressable range",
                        )?;
                        if has_value_offset != 0 {
                            emit_instruction(buf, Op::I32Const(has_value_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load8U(0));
                        emit_instruction(buf, Op::I32Eqz);
                        emit_instruction(buf, Op::If);
                        self.emit_runtime_panic_with_code(buf, NULLABLE_UNWRAP_PANIC_CODE)?;
                        emit_instruction(buf, Op::End);

                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        let value_offset = ensure_u32(
                            value_offset,
                            "nullable payload offset exceeds wasm32 addressable range",
                        )?;
                        if value_offset != 0 {
                            emit_instruction(buf, Op::I32Const(value_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        match expected_param {
                            ValueType::I32 => emit_instruction(buf, Op::I32Load(0)),
                            ValueType::I64 => emit_instruction(buf, Op::I64Load(0)),
                            _ => {}
                        }
                        continue;
                    }
                }

                if expected_param == ValueType::I64 {
                    if matches!(arg_ty.as_ref(), Some(Ty::String)) {
                        // Some call sites end up passing a `string` where a `str` is expected
                        // (notably base-ctor forwarding). Convert via `string_as_slice` and pack.
                        let string_ptr_ty = self.emit_operand(buf, arg)?;
                        Self::ensure_operand_type(
                            string_ptr_ty,
                            ValueType::I32,
                            "string->str conversion",
                        )?;
                        let hook = self.runtime_hook_index(RuntimeHook::StringAsSlice)?;
                        emit_instruction(buf, Op::Call(hook));
                        // Results are `(ptr, len)` with `len` on top of the stack.
                        emit_instruction(buf, Op::LocalSet(self.block_local)); // len
                        emit_instruction(buf, Op::LocalSet(self.temp_local)); // ptr

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        emit_instruction(buf, Op::I64ExtendI32U);
                        emit_instruction(buf, Op::I64Const(32));
                        emit_instruction(buf, Op::I64Shl);
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::I64ExtendI32U);
                        emit_instruction(buf, Op::I64Or);
                        continue;
                    }

                    if arg_ty.is_none() {
                        let value_ty = self.emit_operand(buf, arg)?;
                        if value_ty == ValueType::I32 {
                            let signed = matches!(
                                arg,
                                Operand::Const(constant)
                                    if matches!(constant.value(), ConstValue::Int(_))
                            );
                            emit_instruction(
                                buf,
                                if signed {
                                    Op::I64ExtendI32S
                                } else {
                                    Op::I64ExtendI32U
                                },
                            );
                        }
                        continue;
                    }

                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if local_requires_memory(&arg_ty, self.layouts) {
                            if let Some(allocation) =
                                compute_aggregate_allocation(&arg_ty, self.layouts)
                            {
                                if allocation.size == 8 {
                                    match arg {
                                        Operand::Copy(place) | Operand::Move(place) => {
                                            let access = self.resolve_memory_access(place)?;
                                            self.emit_pointer_expression(buf, &access)?;
                                            emit_instruction(buf, Op::I64Load(0));
                                            continue;
                                        }
                                        Operand::Borrow(borrow) => {
                                            let access =
                                                self.resolve_memory_access(&borrow.place)?;
                                            self.emit_pointer_expression(buf, &access)?;
                                            emit_instruction(buf, Op::I64Load(0));
                                            continue;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }

                if expected_param == ValueType::I64 {
                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if map_type(&arg_ty) == ValueType::I32
                            && !local_requires_memory(&arg_ty, self.layouts)
                            && !matches!(arg_ty, Ty::Str | Ty::Nullable(_))
                        {
                            let value_ty = self.emit_operand(buf, arg)?;
                            Self::ensure_operand_type(value_ty, ValueType::I32, "i32->i64 arg")?;
                            let signed = match &arg_ty {
                                Ty::Pointer(_)
                                | Ty::Ref(_)
                                | Ty::Rc(_)
                                | Ty::Arc(_)
                                | Ty::Fn(_) => false,
                                _ => crate::mir::casts::int_info(
                                    &self.layouts.primitive_registry,
                                    &arg_ty.canonical_name(),
                                    self.pointer_width_bits() / 8,
                                )
                                .map(|info| info.signed)
                                .unwrap_or(true),
                            };
                            emit_instruction(
                                buf,
                                if signed {
                                    Op::I64ExtendI32S
                                } else {
                                    Op::I64ExtendI32U
                                },
                            );
                            continue;
                        }
                    }
                }
            }

            if is_thread_fn_init && index == 1 {
                let fn_ty = arg_ty.as_ref().map(|ty| match ty {
                    Ty::Fn(_) => ty.clone(),
                    Ty::Pointer(inner) => inner.element.clone(),
                    Ty::Ref(inner) => inner.element.clone(),
                    Ty::Nullable(inner) => match inner.as_ref() {
                        Ty::Pointer(ptr) => ptr.element.clone(),
                        Ty::Ref(r) => r.element.clone(),
                        other => other.clone(),
                    },
                    other => other.clone(),
                });
                if let Some(fn_ty) = fn_ty {
                    if let Some(layout) = self
                        .lookup_struct_layout(&fn_ty)
                        .cloned()
                        .filter(Self::is_fn_pointer_layout)
                    {
                        let allocation = compute_aggregate_allocation(&fn_ty, self.layouts)
                            .ok_or_else(|| {
                                Error::Codegen(format!(
                                    "function pointer layout missing allocation metadata for `{}` in WASM backend",
                                    fn_ty.canonical_name()
                                ))
                            })?;
                        self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalSet(self.temp_local));
                        self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, &layout, &fn_ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        continue;
                    }
                }
            }
            if let Operand::Copy(place) | Operand::Move(place) = arg {
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::Scalar)
                ) {
                    let _layout_name = arg_ty
                        .as_ref()
                        .and_then(|ty| self.lookup_struct_layout(ty))
                        .map(|layout| layout.name.clone());
                    let is_fn_like = arg_ty
                        .as_ref()
                        .and_then(|ty| {
                            self.lookup_struct_layout(ty)
                                .filter(|layout| Self::is_fn_pointer_layout(layout))
                        })
                        .is_some()
                        || matches!(
                            arg_ty.as_ref(),
                            Some(Ty::Fn(fn_ty))
                                if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        );
                    if is_fn_like {
                        if let Some(arg_ty) = arg_ty {
                            let layout =
                                self.lookup_struct_layout(&arg_ty).cloned().ok_or_else(|| {
                                    Error::Codegen(format!(
                                        "missing function pointer layout for `{}` in WASM backend",
                                        arg_ty.canonical_name()
                                    ))
                                })?;
                            self.emit_fn_invoke_argument(buf, arg, &arg_ty, &layout)?;
                            continue;
                        }
                    }
                    if !is_fn_like {
                        self.emit_operand(buf, arg)?;
                        continue;
                    }
                }
            }
            if let Some(ty) = self.operand_ty(arg) {
                if let Some(layout) = self.lookup_struct_layout(&ty).cloned().filter(
                    crate::codegen::wasm::emitter::function::FunctionEmitter::is_fn_pointer_layout,
                ) {
                    let arg_repr = match arg {
                        Operand::Copy(place) | Operand::Move(place) => {
                            self.representations.get(place.local.0)
                        }
                        Operand::Borrow(borrow) => self.representations.get(borrow.place.local.0),
                        _ => None,
                    };
                    if matches!(
                        arg_repr,
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        match arg {
                            Operand::Copy(place) | Operand::Move(place) => {
                                if let Ok(access) = self.resolve_memory_access(place) {
                                    self.emit_pointer_expression(buf, &access)?;
                                } else {
                                    self.emit_operand(buf, arg)?;
                                }
                            }
                            Operand::Borrow(borrow) => {
                                let access = self.resolve_memory_access(&borrow.place)?;
                                self.emit_pointer_expression(buf, &access)?;
                            }
                            _ => {
                                self.emit_operand(buf, arg)?;
                            }
                        }
                        continue;
                    }
                    self.emit_fn_invoke_argument(buf, arg, &ty, &layout)?;
                    continue;
                }
                if let Ty::Fn(fn_ty) = &ty {
                    if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                        // Extern fn pointers are thin; treat them as scalars.
                    } else {
                        let layout = self.lookup_struct_layout(&ty).cloned().ok_or_else(|| {
                            Error::Codegen(format!(
                                "missing function pointer layout for `{}` in WASM backend",
                                ty.canonical_name()
                            ))
                        })?;
                        self.emit_fn_invoke_argument(buf, arg, &ty, &layout)?;
                        continue;
                    }
                }
                let pointer_to_fn = match &ty {
                    Ty::Pointer(inner) => {
                        matches!(
                            &inner.element,
                            Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        ) || self
                            .lookup_struct_layout(&inner.element)
                            .is_some_and(Self::is_fn_pointer_layout)
                    }
                    Ty::Ref(inner) => {
                        matches!(
                            &inner.element,
                            Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        ) || self
                            .lookup_struct_layout(&inner.element)
                            .is_some_and(Self::is_fn_pointer_layout)
                    }
                    _ => false,
                };
                if pointer_to_fn {
                    let fn_ty = match &ty {
                        Ty::Pointer(inner) => inner.element.clone(),
                        Ty::Ref(inner) => inner.element.clone(),
                        Ty::Nullable(inner) => match inner.as_ref() {
                            Ty::Pointer(ptr) => ptr.element.clone(),
                            Ty::Ref(r) => r.element.clone(),
                            other => other.clone(),
                        },
                        other => other.clone(),
                    };
                    let layout = self.lookup_struct_layout(&fn_ty).cloned().ok_or_else(|| {
                        Error::Codegen(format!(
                            "missing function pointer layout for `{}` in WASM backend",
                            fn_ty.canonical_name()
                        ))
                    })?;
                    self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                    continue;
                }

                // WASM internal calling convention passes non-scalar (frame-allocated) values
                // by pointer to their storage. Keep this consistent even when the MIR argument
                // is passed by value (Copy/Move), otherwise callers can accidentally pass the
                // first field value instead of the aggregate address.
                if !matches!(ty, Ty::Fn(_))
                    && !pointer_to_fn
                    && !self.ty_is_reference(&ty)
                    && local_requires_memory(&ty, self.layouts)
                {
                    match arg {
                        Operand::Copy(place) | Operand::Move(place) => {
                            let access = self.resolve_memory_access(place)?;
                            self.emit_pointer_expression(buf, &access)?;
                            continue;
                        }
                        Operand::Borrow(borrow) => {
                            let access = self.resolve_memory_access(&borrow.place)?;
                            self.emit_pointer_expression(buf, &access)?;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            self.emit_operand(buf, arg)?;
        }
        emit_instruction(buf, Op::Call(callee));
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            if multi_result {
                self.store_multi_call_result(
                    buf,
                    place,
                    expected_results
                        .as_ref()
                        .expect("multi_result implies expected results"),
                )?;
            } else {
                self.store_call_result(buf, place)?;
            }
        } else if expected_results
            .as_ref()
            .is_some_and(|results| results.len() == 1)
        {
            emit_instruction(buf, Op::Drop);
        } else if let Some(results) = expected_results.as_ref().filter(|r| r.len() > 1) {
            for _ in results {
                emit_instruction(buf, Op::Drop);
            }
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }
}
