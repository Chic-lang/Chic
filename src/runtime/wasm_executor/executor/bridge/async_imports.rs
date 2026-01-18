use super::*;

impl<'a> Executor<'a> {
    pub(super) fn invoke_async_import(
        &mut self,
        name: &str,
        params: &[Value],
        tracer: &mut SchedulerTracer,
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "async_cancel" => {
                let [Value::I32(future)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_cancel expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*future).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_cancel received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_cancel ptr={:#x}", ptr);
                }
                self.cancel_future(ptr)?;
                Ok(Some(Value::I32(AwaitStatus::Ready as i32)))
            }
            "async_scope" => {
                let [Value::I32(ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_scope expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_scope received negative pointer".into(),
                })?;
                let status = self.await_future_once(base)?;
                if matches!(status, AwaitStatus::Ready) {
                    return Ok(Some(Value::I32(status as i32)));
                }
                let status = self.await_future_blocking(base, None)?;
                Ok(Some(Value::I32(status)))
            }
            "async_block_on" => {
                let [Value::I32(ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_block_on expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_block_on received negative pointer".into(),
                })?;
                let status = self.await_future_blocking(base, None)?;
                Ok(Some(Value::I32(status)))
            }
            "async_spawn_local" => {
                let [Value::I32(ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_spawn_local expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_spawn_local received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_spawn_local ptr={:#x}", base);
                }
                let status = self.await_future_once(base)?;
                Ok(Some(Value::I32(status as i32)))
            }
            "async_spawn" => {
                let [Value::I32(ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_spawn expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_spawn received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_spawn ptr={:#x}", base);
                }
                let status = self.await_future_once(base)?;
                Ok(Some(Value::I32(status as i32)))
            }
            "async_task_header" => {
                let [Value::I32(task_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_task_header expects a single i32 argument".into(),
                    });
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_task_header {:#x}", task_ptr);
                }
                Ok(Some(Value::I32(*task_ptr)))
            }
            "async_task_result" => {
                let [
                    Value::I32(src_ptr),
                    Value::I32(out_ptr),
                    Value::I32(out_len),
                ] = params
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_task_result expects (i32 src, i32 ptr, i32 len)"
                            .into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative src pointer".into(),
                })?;
                let out = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative out pointer".into(),
                })?;
                let len = u32::try_from(*out_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_task_result received negative length".into(),
                })?;
                let align = if self.options.async_result_len == Some(len) {
                    self.options.async_result_align
                } else {
                    None
                };
                let result_offset = self.async_layout.result_offset(len, align);
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!(
                        "[wasm-async] async_task_result src={:#x} dst={:#x} len={} result_offset={} current={:?}",
                        src, out, len, result_offset, self.current_future
                    );
                }
                let layout = self.async_layout;
                let mut observed_offset = None;
                let mut inferred_base = self
                    .async_nodes
                    .keys()
                    .copied()
                    .filter(|base| src >= *base)
                    .map(|base| (base, src - base))
                    .filter(|(_, diff)| *diff < 512)
                    .min_by_key(|(_, diff)| *diff)
                    .map(|(base, diff)| {
                        observed_offset = Some(diff);
                        base
                    });
                if inferred_base.is_none() {
                    inferred_base = src
                        .checked_sub(layout.task_inner_future_offset + result_offset)
                        .or_else(|| src.checked_sub(result_offset));
                    if inferred_base
                        .map(|base| !self.async_nodes.contains_key(&base))
                        .unwrap_or(true)
                    {
                        let mut best: Option<(u32, u32)> = None;
                        for base in self.async_nodes.keys().copied() {
                            if src > base {
                                let diff = src - base;
                                if diff < 512 {
                                    if best.map(|(_, d)| diff < d).unwrap_or(true) {
                                        best = Some((base, diff));
                                    }
                                }
                            }
                        }
                        if let Some((base, diff)) = best {
                            inferred_base = Some(base);
                            observed_offset = Some(diff);
                        }
                    }
                }
                if let Some(base) = inferred_base {
                    if observed_offset.is_none() {
                        observed_offset = src.checked_sub(base);
                    }
                }
                let invalid_src = src == 0 || inferred_base == Some(0);
                let data = if invalid_src {
                    vec![0u8; len as usize]
                } else {
                    match self.read_bytes(src, len) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            let export_value = inferred_base.and_then(|base| {
                                if len <= 4 {
                                    if let Some(idx) =
                                        self.module.exports.get("chic_rt_async_task_bool_result")
                                    {
                                        if let Ok(values) =
                                            self.invoke(*idx, &[Value::I32(base as i32)])
                                        {
                                            if let Some(Value::I32(value)) = values.first().copied()
                                            {
                                                return Some(if len == 1 && value != 0 {
                                                    vec![1u8]
                                                } else {
                                                    value.to_le_bytes().to_vec()
                                                });
                                            }
                                        }
                                    }
                                    if let Some(idx) =
                                        self.module.exports.get("chic_rt_async_task_int_result")
                                    {
                                        if let Ok(values) =
                                            self.invoke(*idx, &[Value::I32(base as i32)])
                                        {
                                            if let Some(Value::I32(value)) = values.first().copied()
                                            {
                                                return Some(value.to_le_bytes().to_vec());
                                            }
                                        }
                                    }
                                }
                                None
                            });
                            export_value.unwrap_or_else(|| vec![0u8; len as usize])
                        }
                    }
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    let window_start = src.saturating_sub(32);
                    if let Ok(window) = self.read_bytes(window_start, 96) {
                        eprintln!(
                            "[wasm-async] mem[{:#x}..{:#x}]={:?}",
                            window_start,
                            window_start + 96,
                            window
                        );
                    }
                }
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!(
                        "[wasm-async] async_task_result bytes={:?} inferred_base={:?}",
                        data, inferred_base
                    );
                }
                self.store_bytes(out, 0, &data)?;
                if invalid_src {
                    return Ok(Some(Value::I32(AwaitStatus::Ready as i32)));
                }
                if let Some(current) = self.current_future {
                    // Populate the result slot on the current future so awaiters can read it directly.
                    let offset = observed_offset.unwrap_or(result_offset);
                    let _ = self.store_bytes(current + offset, 0, &data);
                    let _ = self.store_bytes(
                        current + layout.task_inner_future_offset + offset,
                        0,
                        &data,
                    );
                    if let Some(node) = self.async_nodes.get_mut(&current) {
                        node.result_offset = Some(offset);
                    }
                } else {
                    let candidates = inferred_base.into_iter().chain(
                        [
                            src.checked_sub(result_offset),
                            src.checked_sub(layout.task_inner_future_offset + result_offset),
                        ]
                        .into_iter()
                        .flatten(),
                    );
                    for base in candidates {
                        if self.async_nodes.contains_key(&base)
                            || self
                                .read_u32(base + layout.future_header_flags_offset)
                                .is_ok()
                        {
                            let offset = observed_offset.unwrap_or(result_offset);
                            let _ = self.store_bytes(base + offset, 0, &data);
                            let _ = self.store_bytes(
                                base + layout.task_inner_future_offset + offset,
                                0,
                                &data,
                            );
                            if let Some(node) = self.async_nodes.get_mut(&base) {
                                node.result_offset = Some(offset);
                            }
                            if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                                eprintln!(
                                    "[wasm-async] async_task_result inferred base={:#x} from src={:#x} offset={}",
                                    base, src, offset
                                );
                            }
                            break;
                        }
                    }
                }
                Ok(Some(Value::I32(AwaitStatus::Ready as i32)))
            }
            "async_token_new" => {
                let ptr = self.allocate_heap_block(1, 1)?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_new -> {:#x}", ptr);
                }
                self.store_bytes(ptr, 0, &[0])?;
                Ok(Some(Value::I32(ptr as i32)))
            }
            "async_token_state" => {
                let [Value::I32(state_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_token_state expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*state_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_token_state received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_state ptr={:#x}", ptr);
                }
                let value = self.read_bytes(ptr, 1)?.first().copied().unwrap_or(0);
                Ok(Some(Value::I32(value as i32)))
            }
            "async_token_cancel" => {
                let [Value::I32(state_ptr)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_token_cancel expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*state_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.async_token_cancel received negative pointer".into(),
                })?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_cancel ptr={:#x}", ptr);
                }
                self.store_bytes(ptr, 0, &[1])?;
                Ok(Some(Value::I32(1)))
            }
            "throw" => {
                let [Value::I32(payload), Value::I64(type_id)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_throw expects (i32, i64) arguments".into(),
                    });
                };
                let payload_u32 = payload.saturating_abs() as u32;
                let type_id_bits = *type_id as u64;
                if std::env::var_os("CHIC_DEBUG_WASM_THROW").is_some() {
                    eprintln!(
                        "[wasm-throw] {} payload=0x{payload_u32:08x} type=0x{type_id_bits:016x}",
                        self.current_wasm_context()
                    );
                }
                let thrown = crate::runtime::error::RuntimeThrownException {
                    payload: payload_u32 as u64,
                    type_id: type_id_bits,
                };
                self.pending_exception = Some(thrown);
                if let Some(hook) = &self.options.error_hook {
                    hook(thrown);
                }
                if let Some(func_index) = self.module.exports.get("chic_rt_throw").copied() {
                    let _ = self.invoke(
                        func_index,
                        &[
                            Value::I64(payload_u32 as i64),
                            Value::I64(type_id_bits as i64),
                        ],
                    )?;
                }
                Ok(None)
            }
            "await" => {
                let start = Instant::now();
                let [Value::I32(_ctx), Value::I32(future)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.await expects (i32, i32) arguments".into(),
                    });
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] await hook future_ptr={:#x}", future);
                    if let Ok(base) = u32::try_from(*future) {
                        let window_start = base.saturating_sub(16);
                        if let Ok(window) = self.read_bytes(window_start, 96) {
                            eprintln!(
                                "[wasm-async] await hook mem[{:#x}..{:#x}]={:?}",
                                window_start,
                                window_start + 96,
                                window
                            );
                        }
                    }
                }
                let prev = self.switch_borrow_context(Some(*future as u32));
                let status = self.await_future_once(*future as u32)?;
                self.switch_borrow_context(prev);
                tracer.record_await("chic_rt.await", start.elapsed())?;
                Ok(Some(Value::I32(status as i32)))
            }
            "yield" => {
                let [Value::I32(_ctx)] = params else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.yield expects a single i32 argument".into(),
                    });
                };
                let status = self.yield_current();
                Ok(Some(Value::I32(status as i32)))
            }
            _ => Err(WasmExecutionError {
                message: format!("unsupported import chic_rt::{name} encountered during execution"),
            }),
        }
    }
}
