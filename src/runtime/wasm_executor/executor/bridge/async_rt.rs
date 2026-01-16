use super::*;

impl<'a> Executor<'a> {
    fn call_async_function(
        &mut self,
        func_index: u32,
        user_args: &[Value],
    ) -> Result<u32, WasmExecutionError> {
        let import_count = self.module.imports.len();
        let func_slot =
            func_index
                .checked_sub(import_count as u32)
                .ok_or_else(|| WasmExecutionError {
                    message: format!(
                        "function index {func_index} out of range (ctx={} stack={})",
                        self.current_wasm_context(),
                        self.format_call_stack()
                    ),
                })? as usize;
        let function = self
            .module
            .functions
            .get(func_slot)
            .ok_or_else(|| WasmExecutionError {
                message: format!(
                    "function index {func_index} out of range (ctx={} stack={})",
                    self.current_wasm_context(),
                    self.format_call_stack()
                ),
            })?;
        let sig = self
            .module
            .types
            .get(function.type_index as usize)
            .ok_or_else(|| WasmExecutionError {
                message: format!("type index {} out of range", function.type_index),
            })?;
        let ptr = if let Some(ValueType::I32) = sig.results.first() {
            let result = self.invoke(func_index, user_args)?;
            result
                .first()
                .and_then(|value| value.as_i32().ok())
                .unwrap_or(0)
                .max(0) as u32
        } else {
            if !sig.results.is_empty() {
                return Err(WasmExecutionError {
                    message: "async entry/testcase must return i32 task pointer in wasm backend"
                        .into(),
                });
            }
            if sig.params.is_empty() || sig.params[0] != ValueType::I32 {
                return Err(WasmExecutionError {
                    message:
                        "async entry/testcase must accept an i32 return slot parameter in wasm backend"
                            .into(),
                });
            }
            let out_ptr = self.allocate_heap_block(128, 4)?;
            let mut params = Vec::with_capacity(sig.params.len());
            params.push(Value::I32(out_ptr as i32));
            for _ in 1..sig.params.len() {
                params.push(Value::I32(0));
            }
            if !user_args.is_empty() {
                params.extend_from_slice(user_args);
            }
            let _ = self.invoke(func_index, &params)?;
            out_ptr
        };
        self.register_future_node(ptr)?;
        let layout = self.async_layout;
        let vtable_ptr = self
            .load_i32(ptr, layout.future_header_vtable_offset)
            .unwrap_or(0);
        if self.future_ready(ptr)? || vtable_ptr == 0 {
            self.mark_future_completed(ptr)?;
        } else {
            self.enqueue_future(ptr);
        }
        Ok(ptr)
    }

    fn register_future_node(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        if self.async_nodes.contains_key(&base) {
            return Ok(());
        }
        let flags = self.future_flags(base)?;
        let completed = flags & (FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED) != 0;
        let faulted = flags & FUTURE_FLAG_FAULTED != 0;
        let cancelled = flags & FUTURE_FLAG_CANCELLED != 0;
        self.async_nodes.insert(
            base,
            AsyncNode {
                waiters: Vec::new(),
                completed,
                faulted,
                cancelled,
                queued: false,
                result_offset: None,
                borrows: HashMap::new(),
            },
        );
        if completed {
            self.wake_waiters(base);
        }
        Ok(())
    }

    fn future_flags(&self, base: u32) -> Result<u32, WasmExecutionError> {
        let layout = self.async_layout;
        self.read_u32(base + layout.future_header_flags_offset)
    }

    fn enqueue_future(&mut self, base: u32) {
        let entry = self.async_nodes.entry(base).or_insert_with(|| AsyncNode {
            waiters: Vec::new(),
            completed: false,
            faulted: false,
            cancelled: false,
            queued: false,
            result_offset: None,
            borrows: HashMap::new(),
        });
        if entry.completed || entry.queued {
            return;
        }
        entry.queued = true;
        self.ready_queue.push_back(base);
    }

    fn poll_future(&mut self, base: u32) -> Result<AwaitStatus, WasmExecutionError> {
        self.register_future_node(base)?;
        if self.future_ready(base)? {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let layout = self.async_layout;
        let vtable_ptr = self.load_i32(base, layout.future_header_vtable_offset)? as u32;
        if vtable_ptr == 0 {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let poll_idx = self.load_i32(vtable_ptr, 0)? as u32;
        let state_ptr = self.load_i32(base, layout.future_header_state_offset)? as u32;
        let ctx_ptr = self
            .load_i32(base, layout.future_header_executor_context_offset)
            .unwrap_or(0) as u32;
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!(
                "[wasm-async] poll_future base={:#x} vtable={:#x} poll_idx={:#x} state_ptr={:#x} ctx_ptr={:#x}",
                base, vtable_ptr, poll_idx, state_ptr, ctx_ptr
            );
        }
        let prev = self.switch_borrow_context(Some(base));
        let status_raw = self
            .invoke(
                poll_idx,
                &[Value::I32(state_ptr as i32), Value::I32(ctx_ptr as i32)],
            )?
            .first()
            .and_then(|value| value.as_i32().ok())
            .unwrap_or(0);
        self.switch_borrow_context(prev);
        let status = if status_raw == AwaitStatus::Ready as i32 {
            AwaitStatus::Ready
        } else {
            AwaitStatus::Pending
        };
        if matches!(status, AwaitStatus::Ready) {
            self.mark_future_completed(base)?;
        }
        Ok(status)
    }

    fn mark_future_completed(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        self.register_future_node(base)?;
        let mut flags = self.future_flags(base)?;
        let ready_mask = FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED;
        if flags & ready_mask != ready_mask {
            flags |= ready_mask;
            let layout = self.async_layout;
            self.write_u32(base + layout.future_header_flags_offset, flags)?;
        }
        // If this is a Task<T>, mirror completion into the inner future.
        let layout = self.async_layout;
        let inner_base = self.resolve_inner_base(base);
        if inner_base != base {
            let inner_flags = self
                .read_u32(inner_base + layout.future_header_flags_offset)
                .unwrap_or_default();
            let cancel_fault = flags & (FUTURE_FLAG_CANCELLED | FUTURE_FLAG_FAULTED);
            let propagated_flags = inner_flags | cancel_fault | ready_mask;
            let _ = self.write_u32(
                inner_base + layout.future_header_flags_offset,
                propagated_flags,
            );
            self.store_completion_flag(inner_base);
        } else {
            self.store_completion_flag(base);
        }
        if let Some(node) = self.async_nodes.get_mut(&base) {
            node.completed = true;
            node.faulted = flags & FUTURE_FLAG_FAULTED != 0;
            node.cancelled = flags & FUTURE_FLAG_CANCELLED != 0;
            node.queued = false;
        }
        self.wake_waiters(base);
        Ok(())
    }

    fn store_completion_flag(&mut self, base: u32) {
        let offset = self.async_layout.future_completed_offset;
        if self.async_layout.bool_size <= 1 {
            let _ = self.store_i8(base, offset, 1);
        } else {
            let _ = self.store_i32(base, offset, 1);
        }
    }

    fn wake_waiters(&mut self, base: u32) {
        if let Some(node) = self.async_nodes.get_mut(&base) {
            let waiters = std::mem::take(&mut node.waiters);
            for waiter in waiters {
                self.enqueue_future(waiter);
            }
        }
    }

    pub(crate) fn await_future_once(
        &mut self,
        base: u32,
    ) -> Result<AwaitStatus, WasmExecutionError> {
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!("[wasm-async] await_future_once base={:#x}", base);
        }
        self.register_future_node(base)?;
        if self.future_ready(base)? {
            self.mark_future_completed(base)?;
            return Ok(AwaitStatus::Ready);
        }
        let status = self.poll_future(base)?;
        if matches!(status, AwaitStatus::Pending) {
            if let Some(current) = self.current_future {
                self.register_future_node(current)?;
                if let Some(node) = self.async_nodes.get_mut(&base) {
                    if !node.waiters.contains(&current) {
                        node.waiters.push(current);
                    }
                }
            }
            self.enqueue_future(base);
        }
        Ok(status)
    }

    pub(crate) fn await_future_blocking(
        &mut self,
        base: u32,
        result_len: Option<u32>,
    ) -> Result<i32, WasmExecutionError> {
        let in_task = self.current_future.is_some();
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        if debug_async {
            eprintln!("[wasm-async] await_future_blocking base={:#x}", base);
            // Snapshot a wider window so async state locals (including cancellation flags)
            // are visible during debugging.
            let remaining = self.memory_len().saturating_sub(base as usize);
            let window_len: u32 = remaining.min(320).try_into().unwrap_or(0);
            if window_len > 0 {
                if let Ok(window) = self.read_bytes(base, window_len) {
                    eprintln!(
                        "[wasm-async] await_future_blocking mem[{:#x}..{:#x}]={:?}",
                        base,
                        base + window_len,
                        window
                    );
                }
            }
        }
        self.register_future_node(base)?;
        self.enqueue_future(base);
        let mut polls = 0usize;
        loop {
            if self.future_ready(base)? {
                let flags = self.future_flags(base)?;
                if flags & FUTURE_FLAG_FAULTED != 0 {
                    return Err(WasmExecutionError {
                        message: format!("async future at 0x{base:08X} faulted"),
                    });
                }
                self.mark_future_completed(base)?;
                if flags & FUTURE_FLAG_CANCELLED != 0 {
                    return Ok(0);
                }
                if debug_async {
                    eprintln!("[wasm-async] future {:#x} completed, loading result", base);
                }
                let len = result_len
                    .or_else(|| {
                        if in_task {
                            None
                        } else {
                            self.options.async_result_len
                        }
                    })
                    .unwrap_or(self.async_layout.uint_size.max(4));
                let align = if in_task {
                    None
                } else {
                    self.options.async_result_align
                };
                return self.load_future_result(base, len, align);
            }
            let next = match self.ready_queue.pop_front() {
                Some(value) => value,
                None => {
                    return Err(WasmExecutionError {
                        message: format!(
                            "async scheduler stalled while waiting for future at 0x{base:08X}"
                        ),
                    });
                }
            };
            polls += 1;
            if polls > 4096 {
                return Err(WasmExecutionError {
                    message: format!(
                        "async scheduler exceeded poll budget while awaiting 0x{base:08X}"
                    ),
                });
            }
            if let Some(node) = self.async_nodes.get_mut(&next) {
                node.queued = false;
            }
            let status = self.poll_future(next)?;
            if debug_async {
                eprintln!(
                    "[wasm-async] poll #{polls} future={:#x} status={:?}",
                    next, status
                );
            }
            if matches!(status, AwaitStatus::Pending) && !self.future_ready(next)? {
                self.enqueue_future(next);
            }
        }
    }

    fn yield_current(&mut self) -> AwaitStatus {
        if let Some(current) = self.current_future {
            self.enqueue_future(current);
            AwaitStatus::Pending
        } else {
            AwaitStatus::Ready
        }
    }

    pub(crate) fn cancel_future(&mut self, base: u32) -> Result<(), WasmExecutionError> {
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        if debug_async {
            eprintln!("[wasm-async] cancel_future base={:#x}", base);
        }
        self.register_future_node(base)?;
        let mut flags = self.future_flags(base)?;
        flags |= FUTURE_FLAG_CANCELLED | FUTURE_FLAG_COMPLETED | FUTURE_FLAG_READY;
        let layout = self.async_layout;
        self.write_u32(base + layout.future_header_flags_offset, flags)?;
        let _ = self.write_u32(base + layout.task_flags_offset, flags);
        let inner_base = self.resolve_inner_base(base);
        if self
            .read_u32(inner_base + layout.future_header_flags_offset)
            .is_ok()
        {
            let result_offset =
                layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
            let _ = self.write_u32(inner_base + layout.future_header_flags_offset, flags);
            self.store_completion_flag(inner_base);
            let _ = self.store_i32(inner_base, result_offset, 0);
        }
        if let Some(node) = self.async_nodes.get_mut(&base) {
            node.completed = true;
            node.cancelled = true;
            node.queued = false;
        }
        self.wake_waiters(base);
        Ok(())
    }

    fn future_ready(&self, base: u32) -> Result<bool, WasmExecutionError> {
        let layout = self.async_layout;
        let flags = self.read_u32(base + layout.future_header_flags_offset)?;
        if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
            eprintln!(
                "[wasm-async] future_ready base={:#x} flags=0x{:x}",
                base, flags
            );
        }
        Ok(flags & FUTURE_FLAG_COMPLETED != 0
            || flags & FUTURE_FLAG_READY != 0
            || flags & FUTURE_FLAG_CANCELLED != 0
            || flags & FUTURE_FLAG_FAULTED != 0)
    }

    fn resolve_inner_base(&self, base: u32) -> u32 {
        let layout = self.async_layout;
        if layout.task_inner_future_offset == 0 {
            return base;
        }
        let candidate = base.saturating_add(layout.task_inner_future_offset);
        let base_flags = self
            .read_u32(base + layout.future_header_flags_offset)
            .unwrap_or_default();
        let candidate_flags = self
            .read_u32(candidate + layout.future_header_flags_offset)
            .unwrap_or_default();
        if base_flags != 0 && candidate_flags == 0 {
            base
        } else {
            candidate
        }
    }

    fn load_future_result(
        &mut self,
        base: u32,
        result_len: u32,
        result_align: Option<u32>,
    ) -> Result<i32, WasmExecutionError> {
        let layout = self.async_layout;
        let task_flags = self.read_u32(base + layout.task_flags_offset)?;
        let debug_async = std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok();
        let node_result_offset = self
            .async_nodes
            .get(&base)
            .and_then(|node| node.result_offset);
        let state_ptr = self
            .load_i32(base, layout.future_header_state_offset)
            .unwrap_or_default() as u32;
        if debug_async {
            eprintln!(
                "[wasm-async] load_future_result base={:#x} state_ptr={:#x} task_flags=0x{:x} len={} align={:?}",
                base, state_ptr, task_flags, result_len, result_align
            );
        }
        let header_flags = self.read_u32(base + layout.future_header_flags_offset)?;
        if header_flags & FUTURE_FLAG_CANCELLED != 0 || task_flags & FUTURE_FLAG_CANCELLED != 0 {
            return Ok(0);
        }
        let inner_base = self.resolve_inner_base(base);
        let inner_flags = self
            .read_u32(inner_base + layout.future_header_flags_offset)
            .unwrap_or_default();
        let inner_completed = self
            .load_i32(inner_base, layout.future_completed_offset)
            .unwrap_or_default();
        let result_offset =
            node_result_offset.unwrap_or_else(|| layout.result_offset(result_len, result_align));
        if header_flags & FUTURE_FLAG_CANCELLED != 0
            || task_flags & FUTURE_FLAG_CANCELLED != 0
            || inner_flags & FUTURE_FLAG_CANCELLED != 0
        {
            return Ok(0);
        }

        if result_len == self.async_layout.bool_size {
            let value = *self
                .load_bytes(inner_base, result_offset, 1)?
                .first()
                .unwrap_or(&0);
            let value = i32::from(value != 0);
            if debug_async {
                eprintln!(
                    "[wasm-async] load_future_result bool value={} addr={:#x}",
                    value,
                    inner_base + result_offset
                );
            }
            return Ok(value);
        }
        if debug_async {
            let probe_addr = inner_base.saturating_add(result_offset.saturating_sub(4));
            match self.read_bytes(probe_addr, result_len.max(16)) {
                Ok(window) => {
                    eprintln!(
                        "[wasm-async] load_future_result probe addr={:#x} len={} bytes={:?}",
                        probe_addr,
                        result_len.max(16),
                        window
                    );
                }
                Err(err) => {
                    eprintln!(
                        "[wasm-async] load_future_result probe failed addr={:#x} len={} err={}",
                        probe_addr,
                        result_len.max(16),
                        err.message
                    );
                }
            }
            eprintln!(
                "[wasm-async] load_future_result inner_base={:#x} inner_flags=0x{:x} inner_completed={} result_len={} result_offset={} header_flags=0x{:x}",
                inner_base, inner_flags, inner_completed, result_len, result_offset, header_flags
            );
        }
        let mut result_value = self.load_i32(inner_base, result_offset).ok();
        let mut exported: Option<i32> = None;
        let mut observed_offset = None;
        if result_len <= 4 {
            exported = self
                .module
                .exports
                .get("chic_rt_async_task_bool_result")
                .and_then(|idx| self.invoke(*idx, &[Value::I32(base as i32)]).ok())
                .and_then(|values| values.first().copied())
                .and_then(|value| value.as_i32().ok())
                .map(|v| if result_len == 1 { (v != 0) as i32 } else { v })
                .or_else(|| {
                    self.module
                        .exports
                        .get("chic_rt_async_task_int_result")
                        .and_then(|idx| self.invoke(*idx, &[Value::I32(base as i32)]).ok())
                        .and_then(|values| values.first().copied())
                        .and_then(|value| value.as_i32().ok())
                });
        }
        if debug_async {
            let window_start =
                inner_base.saturating_add(layout.future_completed_offset.saturating_sub(4));
            if let Ok(window) = self.read_bytes(window_start, 40) {
                eprintln!(
                    "[wasm-async] load_future_result window[{:#x}..{:#x}]={:?}",
                    window_start,
                    window_start + 40,
                    window
                );
            }
        }
        if node_result_offset.is_none() && result_len <= 4 && result_value.is_none() {
            for step in [0, 4, 8, 12, 16] {
                let candidate = result_offset.saturating_add(step);
                if let Ok(value) = self.load_i32(inner_base, candidate) {
                    if value != 0 {
                        observed_offset = Some(candidate);
                        result_value = Some(value);
                        if let Some(node) = self.async_nodes.get_mut(&base) {
                            node.result_offset = Some(candidate);
                        }
                        break;
                    }
                }
            }
        }
        if (result_value.is_none() || result_value == Some(0))
            && result_len == self.async_layout.bool_size
        {
            let alt_offset = self.async_layout.result_offset(
                self.async_layout.uint_size.max(4),
                Some(self.async_layout.uint_align),
            );
            if alt_offset != result_offset {
                if let Ok(value) = self.load_i32(inner_base, alt_offset) {
                    if value != 0 {
                        observed_offset = Some(alt_offset);
                        result_value = Some(value);
                        if let Some(node) = self.async_nodes.get_mut(&base) {
                            node.result_offset = Some(alt_offset);
                        }
                    }
                }
            }
            if result_value.is_none() || result_value == Some(0) {
                for delta in 1..=8 {
                    let candidate = result_offset.saturating_add(delta);
                    if let Ok(value) = self.load_i32(inner_base, candidate) {
                        if value != 0 {
                            observed_offset = Some(candidate);
                            result_value = Some(value);
                            if let Some(node) = self.async_nodes.get_mut(&base) {
                                node.result_offset = Some(candidate);
                            }
                            break;
                        }
                    }
                }
            }
        }
        let final_offset = observed_offset.unwrap_or(result_offset);
        let mut value = result_value
            .or_else(|| self.load_i32(inner_base, final_offset).ok())
            .or(exported)
            .unwrap_or(0);
        if result_len == self.async_layout.bool_size {
            value = (value != 0) as i32;
        }
        if debug_async {
            eprintln!(
                "[wasm-async] load_future_result value={} addr={:#x}",
                value,
                inner_base + final_offset
            );
        }
        Ok(value)
    }

}
