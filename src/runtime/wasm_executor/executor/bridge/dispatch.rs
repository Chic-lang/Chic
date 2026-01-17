use super::*;

impl<'a> Executor<'a> {
    pub(crate) fn invoke_import(
        &mut self,
        import: &Import,
        params: Vec<Value>,
        tracer: &mut SchedulerTracer,
    ) -> Result<Vec<Value>, WasmExecutionError> {
        struct ImportGuard<'a> {
            exec: *mut Executor<'a>,
            prev: Option<(String, String)>,
        }

        impl<'a> Drop for ImportGuard<'a> {
            fn drop(&mut self) {
                unsafe {
                    (*self.exec).current_import = self.prev.take();
                }
            }
        }

        let prev_import = self.current_import.take();
        self.current_import = Some((import.module.clone(), import.name.clone()));
        let _import_guard = ImportGuard {
            exec: self as *mut Executor<'a>,
            prev: prev_import,
        };

        if import.module == "chic_rt" && import.name == "string_as_slice" {
            let [Value::I32(ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_as_slice expects a single i32 argument".into(),
                });
            };
            let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_slice received negative pointer".into(),
            })?;
            if base == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let repr = self.read_string_repr(base)?;
            let len = repr.len;
            let inline = self.is_inline_string(&repr);
            let slice_ptr = if len == 0 {
                0
            } else {
                self.resolve_string_data_ptr(base, &repr)?
            };
            if len > 0 {
                let mem_len = u32::try_from(self.memory_len()).map_err(|_| WasmExecutionError {
                    message: "wasm memory length exceeds addressable range for wasm32".into(),
                })?;
                let end = slice_ptr
                    .checked_add(len)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!(
                            "string_as_slice pointer overflow: base=0x{base:08X} slice_ptr=0x{slice_ptr:08X} len={len}"
                        ),
                    })?;
                if end > mem_len {
                    return Err(WasmExecutionError {
                        message: format!(
                            "string_as_slice out of bounds: base=0x{base:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} slice_ptr=0x{slice_ptr:08X} end=0x{end:08X} mem_len=0x{mem_len:08X}",
                            ptr = repr.ptr,
                            cap = repr.cap
                        ),
                    });
                }
            }
            if std::env::var_os("CHIC_DEBUG_WASM_STRING_AS_SLICE").is_some() {
                let stride = self.ptr_stride();
                eprintln!(
                    "[wasm-string] as_slice base=0x{base:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} slice_ptr=0x{slice_ptr:08X} mem_len={} stride={stride}",
                    self.memory_len(),
                    ptr = repr.ptr,
                    cap = repr.cap
                );
            }
            return Ok(vec![Value::I32(slice_ptr as i32), Value::I32(len as i32)]);
        }

        if import.module == "chic_rt" && import.name == "string_try_copy_utf8" {
            let [
                Value::I32(string_ptr),
                Value::I32(span_ptr),
                Value::I32(out_written_ptr),
            ] = params.as_slice()
            else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 expects (i32, i32, i32) arguments"
                        .into(),
                });
            };
            let string_ptr = u32::try_from(*string_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_try_copy_utf8 received negative string pointer".into(),
            })?;
            let span_ptr = u32::try_from(*span_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_try_copy_utf8 received negative span pointer".into(),
            })?;
            let out_written_ptr =
                u32::try_from(*out_written_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 received negative out pointer".into(),
                })?;
            if out_written_ptr == 0 {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_try_copy_utf8 out pointer must be non-null".into(),
                });
            }

            let (dest_ptr, dest_len, elem_size, elem_align) = if span_ptr == 0 {
                (0, 0, 0, 0)
            } else {
                self.read_span_ptr(span_ptr)?
            };

            let (slice_ptr, len) = if string_ptr == 0 {
                (0, 0)
            } else {
                let repr = self.read_string_repr(string_ptr)?;
                if repr.len == 0 {
                    (0, 0)
                } else {
                    let slice_ptr = self.resolve_string_data_ptr(string_ptr, &repr)?;
                    (slice_ptr, repr.len)
                }
            };

            let mut written = 0u32;
            let ok = if len == 0 {
                true
            } else if span_ptr == 0 {
                false
            } else if elem_size != 1 || elem_align != 1 {
                false
            } else if dest_len < len {
                false
            } else if dest_ptr == 0 {
                false
            } else {
                let data = self.read_bytes(slice_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                written = len;
                true
            };

            self.write_u32(out_written_ptr, written)?;
            return Ok(vec![Value::I32(if ok { 1 } else { 0 })]);
        }

        if import.module == "chic_rt" && import.name == "string_as_chars" {
            let [Value::I32(ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.string_as_chars expects a single i32 argument".into(),
                });
            };
            let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_chars received negative pointer".into(),
            })?;
            if base == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let repr = self.read_string_repr(base)?;
            if repr.len == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let slice_ptr = self.resolve_string_data_ptr(base, &repr)?;
            let data = self.read_bytes(slice_ptr, repr.len)?;
            let text = std::str::from_utf8(&data).map_err(|err| WasmExecutionError {
                message: format!("chic_rt.string_as_chars encountered invalid UTF-8: {err}"),
            })?;
            let mut utf16_units: Vec<u16> = Vec::new();
            for ch in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                utf16_units.extend_from_slice(encoded);
            }
            if utf16_units.is_empty() {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let mut encoded = Vec::with_capacity(utf16_units.len() * 2);
            for unit in utf16_units {
                encoded.extend_from_slice(&unit.to_le_bytes());
            }
            let byte_len = u32::try_from(encoded.len()).map_err(|_| WasmExecutionError {
                message: "chic_rt.string_as_chars output exceeds wasm32 addressable range".into(),
            })?;
            let out_ptr = self.allocate_heap_block(byte_len, 2)?;
            self.store_bytes(out_ptr, 0, &encoded)?;
            let char_len = i32::try_from(encoded.len() / 2).unwrap_or(i32::MAX);
            return Ok(vec![Value::I32(out_ptr as i32), Value::I32(char_len)]);
        }

        if import.module == "chic_rt" && import.name == "str_as_chars" {
            let [Value::I32(slice_ptr)] = params.as_slice() else {
                return Err(WasmExecutionError {
                    message: "chic_rt.str_as_chars expects a single i32 argument".into(),
                });
            };
            let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                message: "chic_rt.str_as_chars received negative pointer".into(),
            })?;
            if slice_ptr == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let (ptr, len) = self.read_str_ptr(slice_ptr)?;
            if len == 0 || ptr == 0 {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let data = self.read_bytes(ptr, len)?;
            let text = std::str::from_utf8(&data).map_err(|err| WasmExecutionError {
                message: format!("chic_rt.str_as_chars encountered invalid UTF-8: {err}"),
            })?;
            let mut utf16_units: Vec<u16> = Vec::new();
            for ch in text.chars() {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                utf16_units.extend_from_slice(encoded);
            }
            if utf16_units.is_empty() {
                return Ok(vec![Value::I32(0), Value::I32(0)]);
            }
            let mut encoded = Vec::with_capacity(utf16_units.len() * 2);
            for unit in utf16_units {
                encoded.extend_from_slice(&unit.to_le_bytes());
            }
            let byte_len = u32::try_from(encoded.len()).map_err(|_| WasmExecutionError {
                message: "chic_rt.str_as_chars output exceeds wasm32 addressable range".into(),
            })?;
            let out_ptr = self.allocate_heap_block(byte_len, 2)?;
            self.store_bytes(out_ptr, 0, &encoded)?;
            let char_len = i32::try_from(encoded.len() / 2).unwrap_or(i32::MAX);
            return Ok(vec![Value::I32(out_ptr as i32), Value::I32(char_len)]);
        }

        let result = (|| match (import.module.as_str(), import.name.as_str()) {
            ("chic_rt", "rounding_mode") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "chic_rt.rounding_mode expects ()".into(),
                    });
                }
                let mode = rounding_mode() as i32;
                return Ok(Some(Value::I32(mode)));
            }
            ("chic_rt", "f32_rem") | ("chic_rt", "chic_rt_f32_rem") => {
                let [Value::F32(lhs), Value::F32(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.f32_rem expects (f32 lhs, f32 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F32(lhs % rhs)));
            }
            ("chic_rt", "f64_rem") | ("chic_rt", "chic_rt_f64_rem") => {
                let [Value::F64(lhs), Value::F64(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.f64_rem expects (f64 lhs, f64 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F64(lhs % rhs)));
            }
            ("chic_rt", name) if name.starts_with("math_") => {
                return Err(WasmExecutionError {
                    message: format!(
                        "math shim import `{name}` is unsupported; Std.Math lowers directly"
                    ),
                });
            }
            ("chic_rt", name)
                if matches!(
                    name,
                    "alloc"
                        | "alloc_zeroed"
                        | "realloc"
                        | "free"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "chic_rt_alloc"
                        | "chic_rt_alloc_zeroed"
                        | "chic_rt_realloc"
                        | "chic_rt_free"
                        | "chic_rt_memcpy"
                        | "chic_rt_memmove"
                        | "chic_rt_memset"
                ) =>
            {
                self.invoke_memory_import(name, params.as_slice())
            }
            ("env", name) if !name.starts_with("chic_rt_thread_") => {
                self.invoke_env_import(name, params.as_slice(), tracer)
            }
            ("env", "chic_rt_thread_sleep_ms") | ("chic_rt", "chic_rt_thread_sleep_ms") => {
                let [Value::I64(ms)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_sleep_ms expects (i64 millis)".into(),
                    });
                };
                let millis = u64::try_from(*ms).unwrap_or(0);
                let _ = self.host_sleep_millis(millis.min(u64::from(u32::MAX)) as u32)?;
                return Ok(None);
            }
            ("env", "chic_rt_thread_yield") | ("chic_rt", "chic_rt_thread_yield") => {
                let _ = self.host_sleep_millis(0)?;
                return Ok(None);
            }
            ("env", "chic_rt_thread_spin_wait") | ("chic_rt", "chic_rt_thread_spin_wait") => {
                let [Value::I32(iters)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_spin_wait expects (i32 iterations)".into(),
                    });
                };
                let iterations = u32::try_from(*iters).unwrap_or(0);
                for _ in 0..iterations {
                    std::hint::spin_loop();
                }
                return Ok(None);
            }
            ("env", "chic_rt_thread_spawn") | ("chic_rt", "chic_rt_thread_spawn") => {
                let [Value::I32(_start), Value::I32(handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_spawn expects (i32 start, i32 handle_ptr)".into(),
                    });
                };
                if *handle_ptr != 0 {
                    let slot = u32::try_from(*handle_ptr).map_err(|_| WasmExecutionError {
                        message: "chic_rt_thread_spawn received negative handle pointer".into(),
                    })?;
                    let _ = self.store_i32(slot, 0, 0);
                }
                return Ok(Some(Value::I32(1)));
            }
            ("env", "chic_rt_thread_join") | ("chic_rt", "chic_rt_thread_join") => {
                let [Value::I32(_handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_join expects (i32 handle_ptr)".into(),
                    });
                };
                return Ok(Some(Value::I32(1)));
            }
            ("env", "chic_rt_thread_detach") | ("chic_rt", "chic_rt_thread_detach") => {
                let [Value::I32(_handle_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_thread_detach expects (i32 handle_ptr)".into(),
                    });
                };
                return Ok(Some(Value::I32(1)));
            }
            ("chic_rt", name)
                if name.starts_with("arc_")
                    || name.starts_with("weak_")
                    || name.starts_with("chic_rt_arc_")
                    || name.starts_with("chic_rt_weak_")
                    || matches!(name, "object_new" | "panic" | "abort") =>
            {
                self.invoke_arc_import(name, params.as_slice())
            }
            ("chic_rt", "has_pending_exception") => {
                if let Some(func_index) = self
                    .module
                    .exports
                    .get("chic_rt_has_pending_exception")
                    .copied()
                {
                    let mut values = self.invoke(func_index, &[])?;
                    if values.len() != 1 {
                        return Err(WasmExecutionError {
                            message:
                                "chic_rt_has_pending_exception returned unexpected value count"
                                    .into(),
                        });
                    }
                    let value = values.pop().unwrap_or(Value::I32(0));
                    let result = match value {
                        Value::I32(v) => v,
                        Value::I64(v) => v as i32,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "chic_rt_has_pending_exception returned non-integer"
                                    .into(),
                            });
                        }
                    };
                    Ok(Some(Value::I32(result)))
                } else {
                    Ok(Some(Value::I32(i32::from(
                        self.pending_exception.is_some(),
                    ))))
                }
            }
            ("chic_rt", "take_pending_exception") => {
                let [Value::I32(payload_ptr), Value::I32(type_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.take_pending_exception expects two i32 out-pointers"
                            .into(),
                    });
                };
                if let Some(func_index) = self
                    .module
                    .exports
                    .get("chic_rt_take_pending_exception")
                    .copied()
                {
                    let mut values = self.invoke(
                        func_index,
                        &[Value::I32(*payload_ptr), Value::I32(*type_ptr)],
                    )?;
                    self.pending_exception = None;
                    if values.len() != 1 {
                        return Err(WasmExecutionError {
                            message:
                                "chic_rt_take_pending_exception returned unexpected value count"
                                    .into(),
                        });
                    }
                    let value = values.pop().unwrap_or(Value::I32(0));
                    match value {
                        Value::I32(v) => Ok(Some(Value::I32(v))),
                        Value::I64(v) => Ok(Some(Value::I32(v as i32))),
                        _ => Err(WasmExecutionError {
                            message: "chic_rt_take_pending_exception returned non-integer".into(),
                        }),
                    }
                } else if let Some(exc) = self.pending_exception.take() {
                    if *payload_ptr != 0 {
                        self.write_u64(*payload_ptr as u32, exc.payload)?;
                    }
                    if *type_ptr != 0 {
                        self.write_u64(*type_ptr as u32, exc.type_id)?;
                    }
                    Ok(Some(Value::I32(1)))
                } else {
                    Ok(Some(Value::I32(0)))
                }
            }
            ("chic_rt", "startup_call_entry_async") => {
                let [
                    Value::I64(function_ptr),
                    Value::I32(_flags),
                    Value::I32(_argc),
                    Value::I64(_argv),
                    Value::I64(_envp),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.startup_call_entry_async expects (i64, i32, i32, i64, i64) arguments".into(),
                    });
                };
                let func_index = *function_ptr as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "startup_call_entry_async received negative function pointer"
                            .into(),
                    });
                }
                let task_ptr = self.call_async_function(func_index as u32, &[])? as i64;
                Ok(Some(Value::I64(task_ptr)))
            }
            ("chic_rt", "startup_complete_entry_async") => {
                let [Value::I64(task_ptr), Value::I32(_flags)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_complete_entry_async expects (i64, i32) arguments"
                                .into(),
                    });
                };
                let ptr = *task_ptr as i32;
                if ptr == 0 {
                    return Ok(Some(Value::I32(1)));
                }
                let value = self.await_future_blocking(ptr as u32, None)?;
                Ok(Some(Value::I32(value)))
            }
            ("chic_rt", "startup_call_testcase_async") => {
                let [Value::I64(function_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_call_testcase_async expects a single i64 argument"
                                .into(),
                    });
                };
                let func_index = *function_ptr as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "startup_call_testcase_async received negative function pointer"
                            .into(),
                    });
                }
                let task_ptr = self.call_async_function(func_index as u32, &[])? as i64;
                Ok(Some(Value::I64(task_ptr)))
            }
            ("chic_rt", "startup_complete_testcase_async") => {
                let [Value::I64(task_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.startup_complete_testcase_async expects a single i64 argument"
                                .into(),
                    });
                };
                let ptr = *task_ptr as i32;
                if ptr == 0 {
                    return Ok(Some(Value::I32(1)));
                }
                let value =
                    self.await_future_blocking(ptr as u32, Some(self.async_layout.bool_size))?;
                Ok(Some(Value::I32(value)))
            }
            ("chic_rt", name)
                if name == "await"
                    || name == "yield"
                    || name == "throw"
                    || name.starts_with("async_") =>
            {
                self.invoke_async_import(name, params.as_slice(), tracer)
            }
            ("chic_rt", name)
                if name.starts_with("borrow_")
                    || name == "drop_resource"
                    || name.ends_with("_invoke") =>
            {
                self.invoke_runtime_helpers_import(name, params.as_slice())
            }
            ("chic_rt", name) if name.starts_with("type_") || name.starts_with("chic_rt_type_") => {
                self.invoke_type_import(name, params.as_slice())
            }
            ("chic_rt", "trace_enter") => {
                let [
                    Value::I64(trace_id),
                    Value::I32(label_ptr),
                    Value::I64(len),
                    Value::I64(_cpu_budget),
                    Value::I64(_mem_budget),
                    Value::I64(_gpu_budget),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.trace_enter expects (i64, i32, i64, i64, i64, i64) arguments (budgets currently ignored)"
                                .into(),
                    });
                };
                let ptr = u32::try_from(*label_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.trace_enter received negative label pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.trace_enter received negative label length".into(),
                })?;
                let label = self.read_bytes(ptr, len)?;
                unsafe {
                    chic_rt_trace_enter(*trace_id as u64, label.as_ptr(), label.len() as u64);
                }
                Ok(None)
            }
            ("chic_rt", "trace_exit") => {
                let [Value::I64(trace_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.trace_exit expects a single i64 argument".into(),
                    });
                };
                unsafe {
                    chic_rt_trace_exit(*trace_id as u64);
                }
                Ok(None)
            }
            ("chic_rt", "trace_flush") => {
                let [Value::I32(path_ptr), Value::I64(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.trace_flush expects (i32, i64) arguments".into(),
                    });
                };
                let path = if *path_ptr == 0 || *len == 0 {
                    None
                } else {
                    let ptr = *path_ptr as u32;
                    let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                        message: "chic_rt.trace_flush received negative path length".into(),
                    })?;
                    Some(self.read_bytes(ptr, len)?)
                };
                let status = if let Some(bytes) = path {
                    unsafe { chic_rt_trace_flush(bytes.as_ptr(), bytes.len() as u64) }
                } else {
                    unsafe { chic_rt_trace_flush(std::ptr::null(), 0) }
                };
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "span_from_raw_mut") | ("chic_rt", "chic_rt_span_from_raw_mut") => {
                let [Value::I32(out_ptr), data_ptr, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_from_raw_mut expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let data_ptr = value_as_ptr_u32(data_ptr, "chic_rt.span_from_raw_mut data")?;
                let len = value_as_u32(len, "chic_rt.span_from_raw_mut length")?;
                let (data_ptr, elem_size, elem_align) = if data_ptr == 0 {
                    (0, 0, 1)
                } else {
                    let (ptr, size, align) = self.read_value_ptr(data_ptr)?;
                    if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                        eprintln!(
                            "[wasm-span] from_raw_mut handle=0x{data_ptr:08x} ptr=0x{ptr:08x} size={} align={} len={}",
                            size, align, len
                        );
                    }
                    (ptr, size, align)
                };
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_from_raw_const") | ("chic_rt", "chic_rt_span_from_raw_const") => {
                let [Value::I32(out_ptr), data_ptr, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_from_raw_const expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let data_ptr = value_as_ptr_u32(data_ptr, "chic_rt.span_from_raw_const data")?;
                let len = value_as_u32(len, "chic_rt.span_from_raw_const length")?;
                let (data_ptr, elem_size, elem_align) = if data_ptr == 0 {
                    (0, 0, 1)
                } else {
                    let (ptr, size, align) = self.read_value_ptr(data_ptr)?;
                    if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                        eprintln!(
                            "[wasm-span] from_raw_const handle=0x{data_ptr:08x} ptr=0x{ptr:08x} size={} align={} len={}",
                            size, align, len
                        );
                    }
                    (ptr, size, align)
                };
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_slice_mut") | ("chic_rt", "chic_rt_span_slice_mut") => {
                let [Value::I32(source_ptr), start, length, Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_slice_mut expects (i32, i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_mut received negative source pointer".into(),
                })?;
                let start = value_as_u32(start, "chic_rt.span_slice_mut start")?;
                let length = value_as_u32(length, "chic_rt.span_slice_mut length")?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_mut received negative destination pointer".into(),
                })?;
                if source_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let (data_ptr, span_len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if start > span_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if length > span_len - start {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                let validation = span_validate_stride(elem_size, elem_align);
                if validation != SpanError::Success {
                    return Ok(Some(Value::I32(validation as i32)));
                }
                let mut new_ptr = data_ptr;
                if elem_size != 0 {
                    let offset = match start.checked_mul(elem_size) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                    new_ptr = match data_ptr.checked_add(offset) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                }
                if length == 0 || elem_size == 0 {
                    new_ptr = SPAN_DANGLING_PTR;
                } else if new_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                } else if elem_align > 1 && new_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                self.write_span_ptr(dest_ptr, new_ptr, length, elem_size, elem_align)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", "span_slice_readonly") | ("chic_rt", "chic_rt_span_slice_readonly") => {
                let [Value::I32(source_ptr), start, length, Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.span_slice_readonly expects (i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_readonly received negative source pointer".into(),
                })?;
                let start = value_as_u32(start, "chic_rt.span_slice_readonly start")?;
                let length = value_as_u32(length, "chic_rt.span_slice_readonly length")?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_slice_readonly received negative destination pointer"
                        .into(),
                })?;
                if source_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let (data_ptr, span_len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if start > span_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if length > span_len - start {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                let validation = span_validate_stride(elem_size, elem_align);
                if validation != SpanError::Success {
                    return Ok(Some(Value::I32(validation as i32)));
                }
                let mut new_ptr = data_ptr;
                if elem_size != 0 {
                    let offset = match start.checked_mul(elem_size) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                    new_ptr = match data_ptr.checked_add(offset) {
                        Some(value) => value,
                        None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                    };
                }
                if length == 0 || elem_size == 0 {
                    new_ptr = SPAN_DANGLING_PTR;
                } else if new_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                } else if elem_align > 1 && new_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                self.write_span_ptr(dest_ptr, new_ptr, length, elem_size, elem_align)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", "span_to_readonly") | ("chic_rt", "chic_rt_span_to_readonly") => {
                let [Value::I32(out_ptr), Value::I32(source_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_to_readonly expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_to_readonly received negative return pointer".into(),
                })?;
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_to_readonly received negative source pointer".into(),
                })?;
                if source_ptr == 0 {
                    self.write_span_ptr(out_ptr, 0, 0, 0, 0)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                self.write_span_ptr(out_ptr, data_ptr, len, elem_size, elem_align)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "span_ptr_at_mut") | ("chic_rt", "chic_rt_span_ptr_at_mut") => {
                let [Value::I32(source_ptr), index] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_ptr_at_mut expects (i32, i32) arguments".into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_ptr_at_mut received negative source pointer".into(),
                })?;
                let index = value_as_u32(index, "chic_rt.span_ptr_at_mut index")?;
                if source_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                    eprintln!(
                        "[wasm-span] ptr_at_mut span=0x{source_ptr:08x} index={} data=0x{data_ptr:08x} len={} elem_size={} elem_align={}",
                        index, len, elem_size, elem_align
                    );
                }
                if index >= len {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_size == 0 {
                    return Ok(Some(Value::I32(SPAN_DANGLING_PTR as i32)));
                }
                if data_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_align > 1 && data_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = match index.checked_mul(elem_size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = match data_ptr.checked_add(offset) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "span_ptr_at_readonly") | ("chic_rt", "chic_rt_span_ptr_at_readonly") => {
                let [Value::I32(source_ptr), index] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_ptr_at_readonly expects (i32, i32) arguments".into(),
                    });
                };
                let source_ptr = u32::try_from(*source_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_ptr_at_readonly received negative source pointer".into(),
                })?;
                let index = value_as_u32(index, "chic_rt.span_ptr_at_readonly index")?;
                if source_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (data_ptr, len, elem_size, elem_align) = self.read_span_ptr(source_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_SPAN").is_some() {
                    eprintln!(
                        "[wasm-span] ptr_at_readonly span=0x{source_ptr:08x} index={} data=0x{data_ptr:08x} len={} elem_size={} elem_align={}",
                        index, len, elem_size, elem_align
                    );
                }
                if index >= len {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_size == 0 {
                    return Ok(Some(Value::I32(SPAN_DANGLING_PTR as i32)));
                }
                if data_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if elem_align > 1 && data_ptr % elem_align != 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = match index.checked_mul(elem_size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = match data_ptr.checked_add(offset) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "span_copy_to") => {
                let [
                    Value::I32(src_ptr),
                    Value::I32(src_len),
                    Value::I32(src_size),
                    Value::I32(src_align),
                    Value::I32(dest_ptr),
                    Value::I32(dest_len),
                    Value::I32(dest_size),
                    Value::I32(dest_align),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.span_copy_to expects (i32, i32, i32, i32, i32, i32, i32, i32) arguments".into(),
                    });
                };
                let src_ptr = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source pointer".into(),
                })?;
                let src_len = u32::try_from(*src_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source length".into(),
                })?;
                let src_size = u32::try_from(*src_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source element size".into(),
                })?;
                let _src_align = u32::try_from(*src_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative source alignment".into(),
                })?;
                let dest_ptr = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination pointer".into(),
                })?;
                let dest_len = u32::try_from(*dest_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination length".into(),
                })?;
                let dest_size = u32::try_from(*dest_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination element size"
                        .into(),
                })?;
                let _dest_align = u32::try_from(*dest_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.span_copy_to received negative destination alignment".into(),
                })?;

                if src_len > dest_len {
                    return Ok(Some(Value::I32(SpanError::OutOfBounds as i32)));
                }
                if src_size != dest_size {
                    return Ok(Some(Value::I32(SpanError::InvalidStride as i32)));
                }
                if src_len == 0 || src_size == 0 {
                    return Ok(Some(Value::I32(SpanError::Success as i32)));
                }
                if src_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(SpanError::NullPointer as i32)));
                }
                let byte_len = match src_len.checked_mul(src_size) {
                    Some(len) => len,
                    None => return Ok(Some(Value::I32(SpanError::InvalidStride as i32))),
                };
                let data = self.read_bytes(src_ptr, byte_len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                Ok(Some(Value::I32(SpanError::Success as i32)))
            }
            ("chic_rt", name) if name.starts_with("string_") => {
                self.invoke_string_import(name, params.as_slice())
            }
            ("chic_rt", "closure_env_alloc") => {
                let [Value::I32(size), Value::I32(align)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_alloc expects (i32, i32) arguments".into(),
                    });
                };
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_alloc received negative size".into(),
                })?;
                let align = u32::try_from(*align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_alloc received negative alignment".into(),
                })?;
                let ptr = self.allocate_heap_block(size, align.max(1))?;
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "closure_env_clone") => {
                let [Value::I32(src), Value::I32(size), Value::I32(align)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_clone expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let src = u32::try_from(*src).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative source pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative size".into(),
                })?;
                let align = u32::try_from(*align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.closure_env_clone received negative alignment".into(),
                })?;
                let dest = self.allocate_heap_block(size, align.max(1))?;
                if size != 0 {
                    let data = self.read_bytes(src, size)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                Ok(Some(Value::I32(dest as i32)))
            }
            ("chic_rt", "closure_env_free") => {
                let [Value::I32(_ptr), Value::I32(_size), Value::I32(_align)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.closure_env_free expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                Ok(None)
            }
            ("chic_rt", "drop_missing") => {
                let [Value::I32(_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_missing expects a single i32 argument".into(),
                    });
                };
                Ok(None)
            }
            ("chic_rt", name) if name.starts_with("vec_") || name.starts_with("array_") => {
                self.invoke_vec_import(name, params.as_slice())
            }
            ("chic_rt", name) if name.starts_with("hashset_") => {
                self.invoke_hashset_import(name, params.as_slice())
            }
            ("chic_rt", name) if name.starts_with("hashmap_") => {
                self.invoke_hashmap_import(name, params.as_slice())
            }
            ("chic_rt", "mmio_read") => {
                let [Value::I64(address), Value::I32(width), Value::I32(flags)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.mmio_read expects (i64, i32, i32) arguments".into(),
                    });
                };
                let value = self.read_mmio(*address as u64, *width as u32, *flags)?;
                Ok(Some(Value::I64(value as i64)))
            }
            ("chic_rt", "mmio_write") => {
                let [
                    Value::I64(address),
                    Value::I64(value),
                    Value::I32(width),
                    Value::I32(flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.mmio_write expects (i64, i64, i32, i32) arguments".into(),
                    });
                };
                self.write_mmio(*address as u64, *value as u64, *width as u32, *flags)?;
                Ok(None)
            }
            ("chic_rt", name) if name.starts_with("i128_") || name.starts_with("u128_") => {
                self.invoke_int128_import(name, params.as_slice())
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import {}::{} encountered during execution",
                    import.module, import.name
                ),
            }),
        })()?;

        Ok(result.into_iter().collect())
    }
}
