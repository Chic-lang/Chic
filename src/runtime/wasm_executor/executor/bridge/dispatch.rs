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
            ("env", "write") => {
                let [Value::I32(fd), Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.write expects (i32 fd, i32 ptr, i32 len)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.write received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.write received negative length".into(),
                })?;
                let written = self.host_write(*fd, ptr, len)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "read") => {
                let [Value::I32(fd), Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.read expects (i32 fd, i32 ptr, i32 len)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.read received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.read received negative length".into(),
                })?;
                let read = self.host_read(*fd, ptr, len)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "isatty") => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.isatty expects (i32 fd)".into(),
                    });
                };
                let result = self.host_isatty(*fd);
                return Ok(Some(Value::I32(result)));
            }
            ("env", "monotonic_nanos") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.monotonic_nanos expects ()".into(),
                    });
                }
                let value = self.host_monotonic_nanos()?;
                return Ok(Some(Value::I64(value)));
            }
            ("env", "sleep_millis") => {
                let [Value::I32(ms)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.sleep_millis expects (i32 millis)".into(),
                    });
                };
                let millis = u32::try_from(*ms).map_err(|_| WasmExecutionError {
                    message: "env.sleep_millis received negative millis".into(),
                })?;
                let code = self.host_sleep_millis(millis)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "malloc") => {
                let [size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.malloc expects (i32 size)".into(),
                    });
                };
                let size = value_as_u32(size, "env.malloc size")?;
                let ptr = self.allocate_heap_block(size, 8)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                }
                return Ok(Some(Value::I32(ptr as i32)));
            }
            ("env", "calloc") => {
                let [count, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.calloc expects (i32 count, i32 size)".into(),
                    });
                };
                let count = value_as_u32(count, "env.calloc count")?;
                let size = value_as_u32(size, "env.calloc size")?;
                let total = match count.checked_mul(size) {
                    Some(value) => value,
                    None => return Ok(Some(Value::I32(0))),
                };
                let ptr = self.allocate_heap_block(total, 8)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, total as usize);
                }
                return Ok(Some(Value::I32(ptr as i32)));
            }
            ("env", "realloc") => {
                let [ptr, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.realloc expects (i32 ptr, i32 size)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "env.realloc ptr")?;
                let size = value_as_u32(size, "env.realloc size")?;
                if size == 0 {
                    if ptr != 0 {
                        self.heap_allocations.remove(&ptr);
                    }
                    return Ok(Some(Value::I32(0)));
                }
                if ptr == 0 {
                    let new_ptr = self.allocate_heap_block(size, 8)?;
                    if new_ptr != 0 {
                        self.heap_allocations.insert(new_ptr, size as usize);
                    }
                    return Ok(Some(Value::I32(new_ptr as i32)));
                }
                let old_size = self.heap_allocations.get(&ptr).copied().unwrap_or(0) as u32;
                let new_ptr = self.allocate_heap_block(size, 8)?;
                if new_ptr != 0 {
                    let copy_len = old_size.min(size);
                    if copy_len > 0 {
                        let data = self.read_bytes(ptr, copy_len)?;
                        self.store_bytes(new_ptr, 0, &data)?;
                    }
                    self.heap_allocations.remove(&ptr);
                    self.heap_allocations.insert(new_ptr, size as usize);
                }
                return Ok(Some(Value::I32(new_ptr as i32)));
            }
            ("env", "free") => {
                let [ptr] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.free expects (i32 ptr)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "env.free ptr")?;
                if ptr != 0 {
                    self.heap_allocations.remove(&ptr);
                }
                return Ok(None);
            }
            ("env", "posix_memalign") => {
                let [out_ptr, align, size] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.posix_memalign expects (i32 out_ptr, i32 align, i32 size)"
                            .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(out_ptr, "env.posix_memalign out_ptr")?;
                let align = value_as_u32(align, "env.posix_memalign align")?;
                let size = value_as_u32(size, "env.posix_memalign size")?;
                let ptr = self.allocate_heap_block(size, align)?;
                self.write_u32(out_ptr, ptr)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                    return Ok(Some(Value::I32(0)));
                }
                return Ok(Some(Value::I32(1)));
            }
            ("env", "memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memcpy dest")?;
                let src = value_as_ptr_u32(src, "env.memcpy src")?;
                let len = value_as_u32(len, "env.memcpy len")?;
                if len > 0 {
                    let data = self.read_bytes(src, len)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                return Ok(None);
            }
            ("env", "memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memmove dest")?;
                let src = value_as_ptr_u32(src, "env.memmove src")?;
                let len = value_as_u32(len, "env.memmove len")?;
                if len > 0 {
                    let data = self.read_bytes(src, len)?;
                    self.store_bytes(dest, 0, &data)?;
                }
                return Ok(None);
            }
            ("env", "memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "env.memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "env.memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "env.memset len")?;
                if len > 0 {
                    if std::env::var_os("CHIC_DEBUG_WASM_MEMSET").is_some() {
                        eprintln!(
                            "[wasm-mem] memset[env] dest=0x{dest:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                            self.memory_len(),
                            self.current_wasm_context(),
                        );
                    }
                    self.fill(dest, 0, len, value)?;
                }
                return Ok(None);
            }
            ("chic_rt", "alloc") | ("chic_rt", "chic_rt_alloc") => {
                let [Value::I32(out_ptr), size, align] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.alloc expects (i32 out_ptr, i32 size, i32 align)".into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let size = value_as_u32(size, "chic_rt.alloc size")?;
                let align = value_as_u32(align, "chic_rt.alloc align")?.max(1);
                let ptr = self.allocate_heap_block(size, align)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                    if size > 0 {
                        self.fill(ptr, 0, size, 0)?;
                    }
                }
                self.write_value_ptr(out_ptr, ptr, size, align)?;
                if std::env::var_os("CHIC_DEBUG_WASM_ALLOC").is_some() {
                    let (check_ptr, check_size, check_align) = self
                        .read_value_ptr(out_ptr)
                        .unwrap_or((u32::MAX, u32::MAX, u32::MAX));
                    eprintln!(
                        "[wasm-alloc] alloc out=0x{out_ptr:08X} wrote={{ptr=0x{check_ptr:08X} size={check_size} align={check_align}}} req={{size={size} align={align}}} heap_ptr=0x{ptr:08X} heap_cursor=0x{:08X} mem_len={}",
                        self.heap_cursor,
                        self.memory_len()
                    );
                }
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "alloc_zeroed") | ("chic_rt", "chic_rt_alloc_zeroed") => {
                let [Value::I32(out_ptr), size, align] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.alloc_zeroed expects (i32 out_ptr, i32 size, i32 align)"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let size = value_as_u32(size, "chic_rt.alloc_zeroed size")?;
                let align = value_as_u32(align, "chic_rt.alloc_zeroed align")?.max(1);
                let ptr = self.allocate_heap_block(size, align)?;
                if ptr != 0 {
                    self.heap_allocations.insert(ptr, size as usize);
                }
                self.write_value_ptr(out_ptr, ptr, size, align)?;
                if std::env::var_os("CHIC_DEBUG_WASM_ALLOC").is_some() {
                    let (check_ptr, check_size, check_align) = self
                        .read_value_ptr(out_ptr)
                        .unwrap_or((u32::MAX, u32::MAX, u32::MAX));
                    eprintln!(
                        "[wasm-alloc] alloc_zeroed out=0x{out_ptr:08X} wrote={{ptr=0x{check_ptr:08X} size={check_size} align={check_align}}} req={{size={size} align={align}}} heap_ptr=0x{ptr:08X} heap_cursor=0x{:08X} mem_len={}",
                        self.heap_cursor,
                        self.memory_len()
                    );
                }
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "realloc") | ("chic_rt", "chic_rt_realloc") => {
                let [Value::I32(out_ptr), ptr, old_size, new_size, align] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.realloc expects (i32 out_ptr, i32 ptr, i32 old_size, i32 new_size, i32 align)"
                            .into(),
                    });
                };
                let out_ptr = *out_ptr as u32;
                let ptr = value_as_ptr_u32(ptr, "chic_rt.realloc ptr")?;
                let old_size = value_as_u32(old_size, "chic_rt.realloc old_size")?;
                let new_size = value_as_u32(new_size, "chic_rt.realloc new_size")?;
                let align = value_as_u32(align, "chic_rt.realloc align")?.max(1);
                let (base_ptr, _, _) = self.read_value_ptr(ptr)?;
                if new_size == 0 {
                    if base_ptr != 0 && old_size != 0 {
                        self.heap_allocations.remove(&base_ptr);
                    }
                    self.write_value_ptr(out_ptr, 0, 0, align)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let new_ptr = self.allocate_heap_block(new_size, align)?;
                if new_ptr != 0 && base_ptr != 0 {
                    let copy_len = old_size.min(new_size);
                    if copy_len > 0 {
                        let data = self.read_bytes(base_ptr, copy_len)?;
                        self.store_bytes(new_ptr, 0, &data)?;
                    }
                }
                if base_ptr != 0 {
                    self.heap_allocations.remove(&base_ptr);
                }
                if new_ptr != 0 {
                    self.heap_allocations.insert(new_ptr, new_size as usize);
                }
                self.write_value_ptr(out_ptr, new_ptr, new_size, align)?;
                return Ok(Some(Value::I32(out_ptr as i32)));
            }
            ("chic_rt", "free") | ("chic_rt", "chic_rt_free") => {
                let [ptr] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.free expects (i32 ptr)".into(),
                    });
                };
                let ptr = value_as_ptr_u32(ptr, "chic_rt.free ptr")?;
                let (base_ptr, _, _) = self.read_value_ptr(ptr)?;
                if base_ptr != 0 {
                    self.heap_allocations.remove(&base_ptr);
                }
                return Ok(None);
            }
            ("chic_rt", "memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memcpy dest")?;
                let src = value_as_ptr_u32(src, "chic_rt.memcpy src")?;
                let len = value_as_u32(len, "chic_rt.memcpy len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 || src == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memcpy[raw] dest=0x{dest:08X} src=0x{src:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src, len)?;
                self.store_bytes(dest, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memcpy") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memcpy expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memcpy dest")?;
                let src = value_as_ptr_u32(src, "chic_rt_memcpy src")?;
                let len = value_as_u32(len, "chic_rt_memcpy len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                let (src_ptr, _, _) = self.read_value_ptr(src)?;
                if dest_ptr == 0 || src_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memcpy[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} src=0x{src:08X}->0x{src_ptr:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memmove dest").map_err(|err| {
                    WasmExecutionError {
                        message: format!("{} (ctx={})", err.message, self.current_wasm_context()),
                    }
                })?;
                let src = value_as_ptr_u32(src, "chic_rt.memmove src").map_err(|err| {
                    WasmExecutionError {
                        message: format!("{} (ctx={})", err.message, self.current_wasm_context()),
                    }
                })?;
                let len = value_as_u32(len, "chic_rt.memmove len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 || src == 0 {
                    if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some()
                        && std::env::var_os("CHIC_DEBUG_WASM_MEM_NULLS").is_some()
                    {
                        eprintln!(
                            "[wasm-mem] memmove[raw] skipped dest=0x{dest:08X} src=0x{src:08X} len={len} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memmove[raw] dest=0x{dest:08X} src=0x{src:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                if len == 12 && std::env::var_os("CHIC_DEBUG_WASM_VALUEPTR_MOVES").is_some() {
                    if let (Ok(p0), Ok(p1), Ok(p2)) = (
                        self.read_u32(src),
                        self.read_u32(src + 4),
                        self.read_u32(src + 8),
                    ) {
                        eprintln!(
                            "[wasm-valueptr] memmove src=0x{src:08X} {{ptr=0x{p0:08X} size={p1} align={p2}}} -> dest=0x{dest:08X} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                }
                let data = self.read_bytes(src, len)?;
                self.store_bytes(dest, 0, &data)?;
                if len == 12 && std::env::var_os("CHIC_DEBUG_WASM_VALUEPTR_MOVES").is_some() {
                    if let (Ok(p0), Ok(p1), Ok(p2)) = (
                        self.read_u32(dest),
                        self.read_u32(dest + 4),
                        self.read_u32(dest + 8),
                    ) {
                        eprintln!(
                            "[wasm-valueptr] memmove dest=0x{dest:08X} now {{ptr=0x{p0:08X} size={p1} align={p2}}} caller={}",
                            self.current_wasm_context(),
                        );
                    }
                }
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memmove") => {
                let [dest, src, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memmove expects (i32 dest, i32 src, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memmove dest")?;
                let src = value_as_ptr_u32(src, "chic_rt_memmove src")?;
                let len = value_as_u32(len, "chic_rt_memmove len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                let (src_ptr, _, _) = self.read_value_ptr(src)?;
                if dest_ptr == 0 || src_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memmove[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} src=0x{src:08X}->0x{src_ptr:08X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                let data = self.read_bytes(src_ptr, len)?;
                self.store_bytes(dest_ptr, 0, &data)?;
                return Ok(None);
            }
            ("chic_rt", "memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt.memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "chic_rt.memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "chic_rt.memset len")?;
                if len == 0 {
                    return Ok(None);
                }
                if dest == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memset[raw] dest=0x{dest:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                self.fill(dest, 0, len, value)?;
                return Ok(None);
            }
            ("chic_rt", "chic_rt_memset") => {
                let [dest, value, len] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt_memset expects (i32 dest, i32 value, i32 len)".into(),
                    });
                };
                let dest = value_as_ptr_u32(dest, "chic_rt_memset dest")?;
                let value = match value {
                    Value::I32(v) => *v as u8,
                    Value::I64(v) => *v as u8,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "chic_rt_memset value must be integer".into(),
                        });
                    }
                };
                let len = value_as_u32(len, "chic_rt_memset len")?;
                if len == 0 {
                    return Ok(None);
                }
                let (dest_ptr, _, _) = self.read_value_ptr(dest)?;
                if dest_ptr == 0 {
                    return Ok(None);
                }
                if std::env::var_os("CHIC_DEBUG_WASM_MEM").is_some() {
                    eprintln!(
                        "[wasm-mem] memset[valueptr] dest=0x{dest:08X}->0x{dest_ptr:08X} value=0x{value:02X} len={len} mem_len={} caller={}",
                        self.memory_len(),
                        self.current_wasm_context(),
                    );
                }
                self.fill(dest_ptr, 0, len, value)?;
                return Ok(None);
            }
            ("env", "fmodf") => {
                let [Value::F32(lhs), Value::F32(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmodf expects (f32 lhs, f32 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F32(lhs % rhs)));
            }
            ("env", "fmod") => {
                let [Value::F64(lhs), Value::F64(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmod expects (f64 lhs, f64 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F64(lhs % rhs)));
            }
            ("env", "fopen") => {
                let [Value::I32(path), Value::I32(mode)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fopen expects (i32 path, i32 mode)".into(),
                    });
                };
                let path_ptr = u32::try_from(*path).map_err(|_| WasmExecutionError {
                    message: "env.fopen received negative path pointer".into(),
                })?;
                let mode_ptr = u32::try_from(*mode).map_err(|_| WasmExecutionError {
                    message: "env.fopen received negative mode pointer".into(),
                })?;
                let handle = self.host_fopen(path_ptr, mode_ptr)?;
                return Ok(Some(Value::I32(handle)));
            }
            ("env", "fread") => {
                let [
                    Value::I32(ptr),
                    Value::I32(size),
                    Value::I32(count),
                    Value::I32(stream),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.fread expects (i32 ptr, i32 size, i32 count, i32 stream)"
                            .into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative element size".into(),
                })?;
                let count = u32::try_from(*count).map_err(|_| WasmExecutionError {
                    message: "env.fread received negative count".into(),
                })?;
                let read = self.host_fread(*stream, ptr, size, count)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "fwrite") => {
                let [
                    Value::I32(ptr),
                    Value::I32(size),
                    Value::I32(count),
                    Value::I32(stream),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.fwrite expects (i32 ptr, i32 size, i32 count, i32 stream)"
                            .into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative pointer".into(),
                })?;
                let size = u32::try_from(*size).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative element size".into(),
                })?;
                let count = u32::try_from(*count).map_err(|_| WasmExecutionError {
                    message: "env.fwrite received negative count".into(),
                })?;
                let written = self.host_fwrite(*stream, ptr, size, count)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "fflush") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fflush expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fflush(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fclose") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fclose expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fclose(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fileno") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fileno expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fileno(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "ftell") => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftell expects (i32 stream)".into(),
                    });
                };
                let pos = self.host_ftell(*stream)?;
                return Ok(Some(Value::I64(pos)));
            }
            ("env", "ftruncate") => {
                let [Value::I32(stream), length] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftruncate expects (i32 stream, i64 length)".into(),
                    });
                };
                let length = value_as_i64(length, "env.ftruncate length")?;
                let code = self.host_ftruncate(*stream, length)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "fprintf") => {
                if params.len() < 2 {
                    return Err(WasmExecutionError {
                        message: "env.fprintf expects (i32 stream, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "snprintf") => {
                if params.len() < 3 {
                    return Err(WasmExecutionError {
                        message: "env.snprintf expects (i32 buffer, i32 size, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "fputc") => {
                let [Value::I32(ch), Value::I32(_stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fputc expects (i32 ch, i32 stream)".into(),
                    });
                };
                return Ok(Some(Value::I32(*ch)));
            }
            ("env", "pthread_mutex_init") => {
                let [Value::I32(_mutex), Value::I32(_attr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_init expects (i32 mutex, i32 attr)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_mutex_lock") => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_lock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_mutex_unlock") => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_unlock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_create") => {
                let [thread_ptr, _attrs, entry, arg] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_create expects (i32 thread_ptr, i32 attrs, i64 entry, i32 arg)".into(),
                    });
                };
                let thread_ptr = value_as_ptr_u32(thread_ptr, "env.pthread_create thread_ptr")?;
                let entry = value_as_i64(entry, "env.pthread_create entry")?;
                let arg = value_as_ptr_u32(arg, "env.pthread_create arg")?;
                let func_index = entry as i32;
                if func_index < 0 {
                    return Err(WasmExecutionError {
                        message: "env.pthread_create received negative function pointer".into(),
                    });
                }
                self.invoke(func_index as u32, &[Value::I32(arg as i32)])?;
                let thread_id = self.allocate_thread_id();
                self.write_u32(thread_ptr, thread_id)?;
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_join") => {
                let [_thread, _retval] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_join expects (i32 thread, i32 retval)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_detach") => {
                let [_thread] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_detach expects (i32 thread)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "pthread_setname_np") => {
                let [_thread, _name] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_setname_np expects (i32 thread, i32 name)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            ("env", "sched_yield") => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.sched_yield expects ()".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            ("env", "clock_gettime") => {
                let [Value::I32(clock_id), Value::I32(ts_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.clock_gettime expects (i32 clock_id, i32 ts_ptr)".into(),
                    });
                };
                let ts_ptr = u32::try_from(*ts_ptr).map_err(|_| WasmExecutionError {
                    message: "env.clock_gettime received negative timespec pointer".into(),
                })?;
                let code = self.host_clock_gettime(*clock_id, ts_ptr)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "nanosleep") => {
                let [Value::I32(req_ptr), Value::I32(rem_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.nanosleep expects (i32 req_ptr, i32 rem_ptr)".into(),
                    });
                };
                let req = u32::try_from(*req_ptr).map_err(|_| WasmExecutionError {
                    message: "env.nanosleep received negative request pointer".into(),
                })?;
                let rem = if *rem_ptr == 0 {
                    None
                } else {
                    Some(u32::try_from(*rem_ptr).map_err(|_| WasmExecutionError {
                        message: "env.nanosleep received negative remainder pointer".into(),
                    })?)
                };
                let code = self.host_nanosleep(req, rem)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "accept") => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.accept expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "bind") => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.bind expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "recvfrom") => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.recvfrom expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "sendto") => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.sendto expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            ("env", "chic_thread_invoke") => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_invoke expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            ("env", "chic_thread_drop") => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_drop expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            ("env", "socket") => {
                let [Value::I32(domain), Value::I32(typ), Value::I32(proto)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.socket expects (i32 domain, i32 type, i32 proto)".into(),
                    });
                };
                let fd = self.host_socket(*domain, *typ, *proto)?;
                return Ok(Some(Value::I32(fd)));
            }
            ("env", "connect") => {
                let [Value::I32(fd), Value::I32(addr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.connect expects (i32 fd, i32 sockaddr, i32 len)".into(),
                    });
                };
                let addr_ptr = u32::try_from(*addr).map_err(|_| WasmExecutionError {
                    message: "env.connect received negative sockaddr pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.connect received negative sockaddr length".into(),
                })?;
                let code = self.host_connect(*fd, addr_ptr, len)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "recv") => {
                let [
                    Value::I32(fd),
                    Value::I32(ptr),
                    Value::I32(len),
                    Value::I32(_flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.recv expects (i32 fd, i32 ptr, i32 len, i32 flags)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.recv received negative buffer pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.recv received negative length".into(),
                })?;
                let read = self.host_recv(*fd, ptr, len)?;
                return Ok(Some(Value::I32(read)));
            }
            ("env", "send") => {
                let [
                    Value::I32(fd),
                    Value::I32(ptr),
                    Value::I32(len),
                    Value::I32(_flags),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.send expects (i32 fd, i32 ptr, i32 len, i32 flags)".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "env.send received negative buffer pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "env.send received negative length".into(),
                })?;
                let written = self.host_send(*fd, ptr, len)?;
                return Ok(Some(Value::I32(written)));
            }
            ("env", "shutdown") => {
                let [Value::I32(fd), Value::I32(how)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.shutdown expects (i32 fd, i32 how)".into(),
                    });
                };
                let code = self.host_shutdown_socket(*fd, *how)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "close") => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.close expects (i32 fd)".into(),
                    });
                };
                let code = self.host_close_socket(*fd)?;
                return Ok(Some(Value::I32(code)));
            }
            ("env", "htons") => {
                let [Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.htons expects (i32 value)".into(),
                    });
                };
                let v = u16::try_from(*value as i32).map_err(|_| WasmExecutionError {
                    message: "env.htons received negative value".into(),
                })?;
                let converted = self.host_htons(v);
                return Ok(Some(Value::I32(converted)));
            }
            ("env", "inet_pton") => {
                let [Value::I32(af), Value::I32(src), Value::I32(dst)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.inet_pton expects (i32 af, i32 src, i32 dst)".into(),
                    });
                };
                let src_ptr = u32::try_from(*src).map_err(|_| WasmExecutionError {
                    message: "env.inet_pton received negative src pointer".into(),
                })?;
                let dst_ptr = u32::try_from(*dst).map_err(|_| WasmExecutionError {
                    message: "env.inet_pton received negative dst pointer".into(),
                })?;
                let code = self.host_inet_pton(*af, src_ptr, dst_ptr)?;
                return Ok(Some(Value::I32(code)));
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
            ("chic_rt", "arc_new") | ("chic_rt", "chic_rt_arc_new") => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(src_ptr),
                    Value::I32(size),
                    Value::I32(align),
                    Value::I32(drop_fn),
                    Value::I64(type_id),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_new expects (i32 dest, i32 src, i32 size, i32 align, i32 drop_fn, i64 type_id)"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_new received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_new received negative source pointer".into(),
                })?;
                let mut size = (*size).max(0) as u32;
                let mut align = (*align).max(1) as u32;
                if size == 0 && *src_ptr != 0 {
                    size = 8;
                    align = align.max(4);
                }
                if std::env::var("CHIC_DEBUG_WASM_ARC").is_ok() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] new dest=0x{dest:08x} src=0x{src:08x} size={size} align={align} drop_fn=0x{drop_fn:08x} type_id=0x{type_id:016x} func={func}"
                    );
                    if src != 0 {
                        if let Ok(bytes) = self.read_bytes(src, 8) {
                            eprintln!("[wasm-arc] src_preview={bytes:?}");
                        }
                    }
                }
                if dest == 0 {
                    return Ok(Some(Value::I32(-1)));
                }
                if size > 0 && src == 0 {
                    self.store_i32(dest, 0, 0)?;
                    return Ok(Some(Value::I32(-1)));
                }
                let mut data = if size == 0 {
                    Vec::new()
                } else {
                    match self.read_bytes(src, size) {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            let fallback = size.min(4);
                            if std::env::var("CHIC_DEBUG_WASM_ARC").is_ok() {
                                eprintln!(
                                    "[wasm-arc] clamp read size from {} to {} after error `{}`",
                                    size, fallback, err.message
                                );
                            }
                            size = fallback;
                            self.read_bytes(src, fallback).unwrap_or_default()
                        }
                    }
                };
                if data.iter().all(|byte| *byte == 0) {
                    if let Some(obj) = self.last_object_new {
                        if let Ok(snapshot) = self.read_bytes(obj, size) {
                            if arc_debug_enabled() {
                                eprintln!(
                                    "[wasm-arc] substituting zero payload from last object_new 0x{obj:08x}"
                                );
                            }
                            data = snapshot;
                        }
                    }
                }
                if size == 4 && data.len() >= 4 {
                    let mut ptr_bytes = [0u8; 4];
                    ptr_bytes.copy_from_slice(&data[..4]);
                    let value_ptr = u32::from_le_bytes(ptr_bytes);
                    let mut substituted = false;
                    if value_ptr < super::scheduler::LINEAR_MEMORY_HEAP_BASE {
                        if let Some(obj) = self.last_object_new {
                            data = obj.to_le_bytes().to_vec();
                            substituted = true;
                            if arc_debug_enabled() {
                                eprintln!(
                                    "[wasm-arc] substituted payload pointer 0x{value_ptr:08x} with last object_new 0x{obj:08x}"
                                );
                            }
                        }
                    } else if let Ok(snapshot) = self.read_bytes(value_ptr, 24) {
                        if snapshot.iter().all(|byte| *byte == 0) {
                            if let Some(obj) = self.last_object_new {
                                data = obj.to_le_bytes().to_vec();
                                substituted = true;
                                if arc_debug_enabled() {
                                    eprintln!(
                                        "[wasm-arc] substituted zeroed payload at 0x{value_ptr:08x} with last object_new 0x{obj:08x}"
                                    );
                                }
                            }
                        } else if arc_debug_enabled() {
                            eprintln!(
                                "[wasm-arc] value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                    if arc_debug_enabled() && !substituted && value_ptr != 0 {
                        if let Ok(snapshot) = self.read_bytes(value_ptr, 24) {
                            eprintln!(
                                "[wasm-arc] value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                }
                if arc_debug_enabled() && size >= 4 && data.len() >= 4 {
                    let mut ptr_bytes = [0u8; 4];
                    ptr_bytes.copy_from_slice(&data[0..4]);
                    let value_ptr = u32::from_le_bytes(ptr_bytes);
                    if value_ptr != 0 {
                        if let Ok(snapshot) = self.read_bytes(value_ptr, 64) {
                            eprintln!(
                                "[wasm-arc] final value_ptr=0x{value_ptr:08x} snapshot={snapshot:?}"
                            );
                        }
                    }
                }
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_new layout overflow".into(),
                    })?;
                let total_size =
                    data_offset
                        .checked_add(size)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_new size overflow".into(),
                        })?;
                let base_align = ARC_HEADER_ALIGN.max(align);
                let base = match self.allocate_heap_block(total_size, base_align) {
                    Ok(ptr) => ptr,
                    Err(_) => {
                        let _ = self.store_i32(dest, 0, 0);
                        return Ok(Some(Value::I32(-2)));
                    }
                };
                if arc_debug_enabled() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] new dest=0x{dest:08x} src=0x{src:08x} size={size} align={align} base=0x{base:08x} drop_fn=0x{drop_fn:08x} type_id=0x{type_id:016x} func={func}"
                    );
                }
                self.store_bytes(base, ARC_STRONG_OFFSET, &1u32.to_le_bytes())?;
                self.store_bytes(base, ARC_WEAK_OFFSET, &1u32.to_le_bytes())?;
                self.store_bytes(base, ARC_SIZE_OFFSET, &size.to_le_bytes())?;
                self.store_bytes(base, ARC_ALIGN_OFFSET, &align.to_le_bytes())?;
                self.store_bytes(base, ARC_DROP_FN_OFFSET, &(*drop_fn as u32).to_le_bytes())?;
                self.store_bytes(base, ARC_TYPE_ID_OFFSET, &(*type_id as u64).to_le_bytes())?;
                self.store_bytes(base, data_offset, &data)?;
                self.store_i32(dest, 0, base as i32)?;
                self.last_arc_header = Some(base);
                self.last_arc_payload = Some(base.saturating_add(data_offset));
                if arc_debug_enabled() {
                    if let Ok(stored) = self.read_u32(dest) {
                        eprintln!(
                            "[wasm-arc] new wrote handle=0x{stored:08x} at dest=0x{dest:08x}"
                        );
                    }
                }
                return Ok(Some(Value::I32(0)));
            }
            ("chic_rt", "arc_clone") | ("chic_rt", "chic_rt_arc_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_clone expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_clone received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if src >= super::scheduler::LINEAR_MEMORY_HEAP_BASE {
                        header = src;
                    } else if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                self.last_arc_header = Some(header);
                let align = self
                    .read_u32(header + ARC_ALIGN_OFFSET)
                    .unwrap_or(ARC_HEADER_MIN_ALIGN)
                    .max(ARC_HEADER_MIN_ALIGN);
                let data_offset = ((ARC_HEADER_SIZE + align - 1) / align) * align;
                self.last_arc_payload = Some(header.saturating_add(data_offset));
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                if strong == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if arc_debug_enabled() {
                    let func = self.current_function.unwrap_or(0);
                    eprintln!(
                        "[wasm-arc] clone dest=0x{dest:08x} src=0x{src:08x} header=0x{header:08x} strong={} func={func}",
                        strong
                    );
                }
                self.write_u32(header + ARC_STRONG_OFFSET, strong.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            ("chic_rt", "arc_drop") | ("chic_rt", "chic_rt_arc_drop") => {
                let [Value::I32(target_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                            if let Some(last) = self.last_arc_header {
                                header = last;
                            } else {
                                let _ = self.store_i32(target, 0, 0);
                                return Ok(None);
                            }
                        }
                        if header != 0 {
                            if let Ok(strong) = self.read_u32(header + ARC_STRONG_OFFSET) {
                                if arc_debug_enabled() {
                                    eprintln!(
                                        "[wasm-arc] drop target=0x{target:08x} header=0x{header:08x} strong={}",
                                        strong
                                    );
                                }
                                if strong > 0 {
                                    let _ = self.write_u32(
                                        header + ARC_STRONG_OFFSET,
                                        strong.saturating_sub(1),
                                    );
                                }
                            }
                        }
                        let _ = self.store_i32(target, 0, 0);
                    }
                }
                return Ok(None);
            }
            ("chic_rt", "arc_get") | ("chic_rt", "chic_rt_arc_get") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] get src=0x{src:08x} header=0x{header:08x}",
                        src = src,
                        header = header
                    );
                }
                let align = self.read_u32(header + ARC_ALIGN_OFFSET)?.max(1);
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_get layout overflow".into(),
                    })?;
                let data_ptr =
                    header
                        .checked_add(data_offset)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_get pointer overflow".into(),
                        })?;
                return Ok(Some(Value::I32(data_ptr as i32)));
            }
            ("chic_rt", "arc_get_mut") | ("chic_rt", "chic_rt_arc_get_mut") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_get_mut expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_get_mut received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] get_mut src=0x{src:08x} header=0x{header:08x} strong={} weak={}",
                        strong, weak
                    );
                }
                if strong != 1 || weak != 1 {
                    return Ok(Some(Value::I32(0)));
                }
                let align = self.read_u32(header + ARC_ALIGN_OFFSET)?.max(1);
                let data_offset =
                    align_up_u32(ARC_HEADER_SIZE, align).ok_or_else(|| WasmExecutionError {
                        message: "chic_rt.arc_get_mut layout overflow".into(),
                    })?;
                let data_ptr =
                    header
                        .checked_add(data_offset)
                        .ok_or_else(|| WasmExecutionError {
                            message: "chic_rt.arc_get_mut pointer overflow".into(),
                        })?;
                return Ok(Some(Value::I32(data_ptr as i32)));
            }
            ("chic_rt", "arc_downgrade") | ("chic_rt", "chic_rt_arc_downgrade") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_downgrade expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_downgrade received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_downgrade received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if weak == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if arc_debug_enabled() {
                    eprintln!(
                        "[wasm-arc] downgrade dest=0x{dest:08x} src=0x{src:08x} header=0x{header:08x} weak={}",
                        weak
                    );
                }
                self.write_u32(header + ARC_WEAK_OFFSET, weak.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            ("chic_rt", "weak_clone") | ("chic_rt", "chic_rt_weak_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_clone expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_clone received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let weak = self.read_u32(header + ARC_WEAK_OFFSET)?;
                if weak == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                self.write_u32(header + ARC_WEAK_OFFSET, weak.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                return Ok(Some(Value::I32(0)));
            }
            ("chic_rt", "weak_drop") | ("chic_rt", "chic_rt_weak_drop") => {
                let [Value::I32(target_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_drop expects (i32 target)".into(),
                    });
                };
                let target = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_drop received negative target pointer".into(),
                })?;
                if target != 0 {
                    if let Ok(mut header) = self.read_u32(target) {
                        if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                            if let Some(last) = self.last_arc_header {
                                header = last;
                            } else {
                                let _ = self.store_i32(target, 0, 0);
                                return Ok(None);
                            }
                        }
                        if header != 0 {
                            if let Ok(count) = self.read_u32(header + ARC_WEAK_OFFSET) {
                                if count > 0 {
                                    let _ = self.write_u32(
                                        header + ARC_WEAK_OFFSET,
                                        count.saturating_sub(1),
                                    );
                                }
                            }
                        }
                        let _ = self.store_i32(target, 0, 0);
                    }
                }
                return Ok(None);
            }
            ("chic_rt", "weak_upgrade") | ("chic_rt", "chic_rt_weak_upgrade") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.weak_upgrade expects (i32 dest, i32 src)".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_upgrade received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.weak_upgrade received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        if dest != 0 {
                            let _ = self.store_i32(dest, 0, 0);
                        }
                        return Ok(Some(Value::I32(-1)));
                    }
                }
                let strong = self.read_u32(header + ARC_STRONG_OFFSET)?;
                if strong == u32::MAX {
                    return Ok(Some(Value::I32(-3)));
                }
                if strong == 0 {
                    if dest != 0 {
                        let _ = self.store_i32(dest, 0, 0);
                    }
                    return Ok(Some(Value::I32(-1)));
                }
                self.write_u32(header + ARC_STRONG_OFFSET, strong.saturating_add(1))?;
                if dest != 0 {
                    self.store_i32(dest, 0, header as i32)?;
                }
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "arc_strong_count") | ("chic_rt", "chic_rt_arc_strong_count") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_strong_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_strong_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let count = self
                    .read_u32(header + ARC_STRONG_OFFSET)
                    .unwrap_or_default() as i32;
                return Ok(Some(Value::I32(count)));
            }
            ("chic_rt", "arc_weak_count") | ("chic_rt", "chic_rt_arc_weak_count") => {
                let [Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.arc_weak_count expects (i32 src)".into(),
                    });
                };
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.arc_weak_count received negative source pointer".into(),
                })?;
                let mut header = self.read_u32(src).unwrap_or(0);
                if header < super::scheduler::LINEAR_MEMORY_HEAP_BASE || header == 0 {
                    if let Some(last) = self.last_arc_header {
                        header = last;
                    } else {
                        return Ok(Some(Value::I32(0)));
                    }
                }
                let count = self.read_u32(header + ARC_WEAK_OFFSET).unwrap_or_default() as i32;
                return Ok(Some(Value::I32(count)));
            }
            ("chic_rt", "object_new") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.object_new expects a single i64 argument".into(),
                    });
                };
                let address = self.allocate_object_instance(*type_id as u64)?;
                if address > i32::MAX as u32 {
                    return Err(WasmExecutionError {
                        message: format!(
                            "chic_rt.object_new produced address 0x{address:08X} that exceeds the wasm32 address space"
                        ),
                    });
                }
                self.last_object_new = Some(address);
                Ok(Some(Value::I32(address as i32)))
            }
            ("chic_rt", "panic") => {
                let [Value::I32(code)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.panic expects a single i32 argument".into(),
                    });
                };
                if std::env::var_os("CHIC_DEBUG_WASM_PANIC").is_some() {
                    let stack = if self.call_stack.is_empty() {
                        "<empty>".to_string()
                    } else {
                        self.call_stack
                            .iter()
                            .map(|idx| self.wasm_context_for_function_index(*idx))
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    };
                    eprintln!(
                        "[wasm-panic] code={code} current={} stack={}",
                        self.current_wasm_context(),
                        stack
                    );
                }
                Err(panic_trap(*code))
            }
            ("chic_rt", "abort") => {
                let [Value::I32(code)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.abort expects a single i32 argument".into(),
                    });
                };
                Err(abort_trap(*code))
            }
            ("chic_rt", "coverage_hit") => {
                let [Value::I64(id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.coverage_hit expects a single i64 argument".into(),
                    });
                };
                if let Some(hook) = self.options.coverage_hook.as_ref() {
                    if *id < 0 {
                        return Err(WasmExecutionError {
                            message: "chic_rt.coverage_hit received negative id".into(),
                        });
                    }
                    hook(*id as u64);
                }
                Ok(None)
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
            ("chic_rt", "async_cancel") => {
                let [Value::I32(future)] = params.as_slice() else {
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
            ("chic_rt", "async_scope") => {
                let [Value::I32(ptr)] = params.as_slice() else {
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
            ("chic_rt", "async_block_on") => {
                let [Value::I32(ptr)] = params.as_slice() else {
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
            ("chic_rt", "async_spawn_local") => {
                let [Value::I32(ptr)] = params.as_slice() else {
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
            ("chic_rt", "async_spawn") => {
                let [Value::I32(ptr)] = params.as_slice() else {
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
            ("chic_rt", "async_task_header") => {
                let [Value::I32(task_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.async_task_header expects a single i32 argument".into(),
                    });
                };
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_task_header {:#x}", task_ptr);
                }
                Ok(Some(Value::I32(*task_ptr)))
            }
            ("chic_rt", "async_task_result") => {
                let [
                    Value::I32(src_ptr),
                    Value::I32(out_ptr),
                    Value::I32(out_len),
                ] = params.as_slice()
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
            ("chic_rt", "async_token_new") => {
                let ptr = self.allocate_heap_block(1, 1)?;
                if std::env::var("CHIC_DEBUG_WASM_ASYNC").is_ok() {
                    eprintln!("[wasm-async] async_token_new -> {:#x}", ptr);
                }
                self.store_bytes(ptr, 0, &[0])?;
                Ok(Some(Value::I32(ptr as i32)))
            }
            ("chic_rt", "async_token_state") => {
                let [Value::I32(state_ptr)] = params.as_slice() else {
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
            ("chic_rt", "async_token_cancel") => {
                let [Value::I32(state_ptr)] = params.as_slice() else {
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
            ("chic_rt", "throw") => {
                let [Value::I32(payload), Value::I64(type_id)] = params.as_slice() else {
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
            ("chic_rt", "await") => {
                let start = Instant::now();
                let [Value::I32(_ctx), Value::I32(future)] = params.as_slice() else {
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
            ("chic_rt", "yield") => {
                let [Value::I32(_ctx)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.yield expects a single i32 argument".into(),
                    });
                };
                let status = self.yield_current();
                Ok(Some(Value::I32(status as i32)))
            }
            ("chic_rt", "borrow_shared") => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_shared expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_shared received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Shared)?;
                Ok(None)
            }
            ("chic_rt", "borrow_unique") => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_unique expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_unique received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Unique)?;
                Ok(None)
            }
            ("chic_rt", "borrow_release") => {
                let [Value::I32(borrow_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_release expects a single i32 argument".into(),
                    });
                };
                self.release_borrow(*borrow_id)?;
                Ok(None)
            }
            ("chic_rt", "drop_invoke") => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(None);
                }
                let func_index = value_as_u32(&Value::I32(*func), "chic_rt.drop_invoke func")?;
                let value_ptr = value_as_ptr_u32(&Value::I32(*value), "chic_rt.drop_invoke value")?;
                if std::env::var_os("CHIC_DEBUG_WASM_DROP_INVOKE").is_some() {
                    let total = self.module.imports.len() + self.module.functions.len();
                    let mut current_exports = Vec::new();
                    if let Some(current) = self.current_function {
                        for (name, &idx) in &self.module.exports {
                            if idx == current {
                                current_exports.push(name.as_str());
                            }
                        }
                    }
                    eprintln!(
                        "[wasm-drop-invoke] func_index={} value_ptr=0x{value_ptr:08x} total_funcs={} current={:?} current_exports={:?} call_stack={:?}",
                        func_index, total, self.current_function, current_exports, self.call_stack
                    );
                }
                let _ = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                Ok(None)
            }
            ("chic_rt", "hash_invoke") => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hash_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative function pointer".into(),
                })?;
                let value_ptr = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative value pointer".into(),
                })?;
                let result = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                let value = result.first().copied();
                match value {
                    Some(Value::I64(hash)) => Ok(Some(Value::I64(hash))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!(
                            "chic_rt.hash_invoke expected i64 result, found {other:?}"
                        ),
                    }),
                    None => Ok(Some(Value::I64(0))),
                }
            }
            ("chic_rt", "eq_invoke") => {
                let [Value::I32(func), Value::I32(left), Value::I32(right)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.eq_invoke expects (i32, i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *left == 0 || *right == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative function pointer".into(),
                })?;
                let left_ptr = u32::try_from(*left).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative left pointer".into(),
                })?;
                let right_ptr = u32::try_from(*right).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative right pointer".into(),
                })?;
                let result = self.invoke(
                    func_index,
                    &[Value::I32(left_ptr as i32), Value::I32(right_ptr as i32)],
                )?;
                let value = result.first().copied();
                match value {
                    Some(Value::I32(value)) => Ok(Some(Value::I32(value))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!("chic_rt.eq_invoke expected i32 result, found {other:?}"),
                    }),
                    None => Ok(Some(Value::I32(0))),
                }
            }
            ("chic_rt", "drop_resource") => {
                let [Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_resource expects a single i32 argument".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.drop_resource received negative address".into(),
                })?;
                self.drop_resource(address)?;
                Ok(None)
            }
            ("chic_rt", "type_metadata") | ("chic_rt", "chic_rt_type_metadata") => {
                let [Value::I64(type_id), Value::I32(out_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_metadata expects (i64 type_id, i32 out_ptr)".into(),
                    });
                };
                let out = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.type_metadata received negative out pointer".into(),
                })?;
                if out == 0 {
                    return Ok(Some(Value::I32(2)));
                }
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let Some(meta) = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                else {
                    return Ok(Some(Value::I32(1)));
                };
                let size = meta.size as i32;
                let align = meta.align as i32;
                let _ = self.store_i32(out, 0, size);
                let _ = self.store_i32(out, 4, align);
                let _ = self.store_i32(out, 8, 0);
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_size") | ("chic_rt", "chic_rt_type_size") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_size expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let size = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                    .map(|meta| meta.size as i32)
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] size[{idx}] type_id=0x{:016X} -> {} (meta_entries={})",
                            key,
                            size,
                            self.type_metadata.len()
                        );
                    }
                }
                Ok(Some(Value::I32(size)))
            }
            ("chic_rt", "type_align") | ("chic_rt", "chic_rt_type_align") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_align expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let align = self
                    .type_metadata
                    .get(&key)
                    .or_else(|| self.type_metadata.get(&swapped))
                    .map(|meta| meta.align as i32)
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] align[{idx}] type_id=0x{:016X} -> {} (meta_entries={})",
                            key,
                            align,
                            self.type_metadata.len()
                        );
                    }
                }
                Ok(Some(Value::I32(align)))
            }
            ("chic_rt", "type_drop_glue") | ("chic_rt", "chic_rt_type_drop_glue") => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_drop_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_clone_glue") | ("chic_rt", "chic_rt_type_clone_glue") => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_clone_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "type_hash_glue") | ("chic_rt", "chic_rt_type_hash_glue") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_hash_glue expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let glue = self
                    .hash_glue
                    .get(&key)
                    .or_else(|| self.hash_glue.get(&swapped))
                    .copied()
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] hash_glue[{idx}] type_id=0x{:016X} -> 0x{glue:08X} (hash_entries={})",
                            key,
                            self.hash_glue.len()
                        );
                    }
                }
                Ok(Some(Value::I32(glue as i32)))
            }
            ("chic_rt", "type_eq_glue") | ("chic_rt", "chic_rt_type_eq_glue") => {
                let [Value::I64(type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_eq_glue expects a single i64 argument".into(),
                    });
                };
                let key = *type_id as u64;
                let swapped = key.swap_bytes();
                let glue = self
                    .eq_glue
                    .get(&key)
                    .or_else(|| self.eq_glue.get(&swapped))
                    .copied()
                    .unwrap_or(0);
                if std::env::var_os("CHIC_DEBUG_WASM_TYPECALLS").is_some() {
                    let idx = TYPECALL_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 80 {
                        eprintln!(
                            "[wasm-type] eq_glue[{idx}] type_id=0x{:016X} -> 0x{glue:08X} (eq_entries={})",
                            key,
                            self.eq_glue.len()
                        );
                    }
                }
                Ok(Some(Value::I32(glue as i32)))
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
            ("chic_rt", "string_new") => {
                let [Value::I32(out_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_new expects a single i32 argument".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_new received negative return pointer".into(),
                })?;
                self.init_empty_string(out_ptr)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_with_capacity") => {
                let [Value::I32(out_ptr), Value::I32(capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_with_capacity expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_with_capacity received negative return pointer".into(),
                })?;
                let capacity = u32::try_from(*capacity).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_with_capacity received negative capacity".into(),
                })?;
                if capacity == 0 {
                    self.init_empty_string(out_ptr)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let ptr = self.allocate_heap_block(capacity, 1)?;
                self.write_string_repr(
                    out_ptr,
                    WasmStringRepr {
                        ptr,
                        len: 0,
                        cap: capacity,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_from_slice") => {
                let [Value::I32(out_ptr), Value::I32(slice_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_from_slice expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_slice received negative return pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_slice received negative slice pointer".into(),
                })?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    let stride = self.ptr_stride();
                    let raw_ptr = self.read_u32(slice_ptr).ok();
                    let raw_len = self.read_u32(slice_ptr + stride).ok();
                    let raw_extra = self.read_u32(slice_ptr + stride * 2).ok();
                    let raw_tail = self.read_u32(slice_ptr + stride * 3).ok();
                    let raw_u64_lo = self.read_u64(slice_ptr).ok();
                    let raw_u64_hi = self.read_u64(slice_ptr + 8).ok();
                    eprintln!(
                        "[wasm-string] string_from_slice out=0x{out_ptr:08X} slice_ptr=0x{slice_ptr:08X} stride={} mem_len={} raw_ptr={raw_ptr:?} raw_len={raw_len:?} raw_extra={raw_extra:?} raw_tail={raw_tail:?} raw_u64_lo={raw_u64_lo:?} raw_u64_hi={raw_u64_hi:?}",
                        stride,
                        self.memory_len(),
                    );
                }
                let (ptr, len) = self.read_str_ptr(slice_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_from_slice data_ptr=0x{ptr:08X} len={len} mem_len={}",
                        self.memory_len()
                    );
                }
                if len == 0 || ptr == 0 {
                    self.init_empty_string(out_ptr)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let data = self.read_bytes(ptr, len)?;
                self.store_string_bytes(out_ptr, &data)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_from_char") => {
                let [Value::I32(out_ptr), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_from_char expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_char received negative return pointer".into(),
                })?;
                let code = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_from_char received negative value".into(),
                })?;
                let mut buf = [0u8; 4];
                let slice = if let Some(ch) = char::from_u32(code) {
                    let encoded = ch.encode_utf8(&mut buf);
                    encoded.as_bytes()
                } else {
                    &[][..]
                };
                self.store_string_bytes(out_ptr, slice)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_push_slice") => {
                let [Value::I32(target_ptr), Value::I32(slice_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_push_slice expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_push_slice received negative target pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_push_slice received negative slice pointer".into(),
                })?;
                let (ptr, len) = self.read_str_ptr(slice_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_push_slice target=0x{target_ptr:08X} slice_ptr=0x{slice_ptr:08X} data_ptr=0x{ptr:08X} len={len} mem_len={}",
                        self.memory_len()
                    );
                }
                let data = if len == 0 || ptr == 0 {
                    Vec::new()
                } else {
                    self.read_bytes(ptr, len)?
                };
                let status = self.append_string_bytes(target_ptr, &data)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_truncate") => {
                let [Value::I32(target_ptr), Value::I32(new_len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_truncate expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_truncate received negative target pointer".into(),
                })?;
                let new_len = u32::try_from(*new_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_truncate received negative length".into(),
                })?;
                let mut repr = self.read_string_repr(target_ptr)?;
                let inline = self.is_inline_string(&repr);
                if repr.len > 0 && repr.ptr == 0 && !inline {
                    return Ok(Some(Value::I32(STRING_INVALID_POINTER)));
                }
                if new_len > repr.len {
                    return Ok(Some(Value::I32(STRING_OUT_OF_BOUNDS)));
                }
                repr.len = new_len;
                self.write_string_repr(target_ptr, repr)?;
                Ok(Some(Value::I32(STRING_SUCCESS)))
            }
            ("chic_rt", "string_reserve") => {
                let [Value::I32(target_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_reserve expects (i32, i32) arguments".into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_reserve received negative target pointer".into(),
                })?;
                let additional = u32::try_from(*additional).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_reserve received negative additional".into(),
                })?;
                if additional == 0 {
                    return Ok(Some(Value::I32(STRING_SUCCESS)));
                }
                let mut repr = self.read_string_repr(target_ptr)?;
                let inline = self.is_inline_string(&repr);
                if repr.len > 0 && repr.ptr == 0 && !inline {
                    return Ok(Some(Value::I32(STRING_INVALID_POINTER)));
                }
                let new_len = match repr.len.checked_add(additional) {
                    Some(len) => len,
                    None => return Ok(Some(Value::I32(STRING_CAPACITY_OVERFLOW))),
                };
                let capacity = if inline {
                    STRING_INLINE_CAPACITY
                } else {
                    repr.cap
                };
                if new_len > capacity {
                    let new_ptr = self.allocate_heap_block(new_len, 1)?;
                    if repr.len > 0 {
                        let existing_ptr = if inline {
                            self.inline_string_ptr(target_ptr)?
                        } else {
                            repr.ptr
                        };
                        let existing = self.read_bytes(existing_ptr, repr.len)?;
                        self.store_bytes(new_ptr, 0, &existing)?;
                    }
                    repr.ptr = new_ptr;
                    repr.cap = new_len;
                }
                self.write_string_repr(target_ptr, repr)?;
                Ok(Some(Value::I32(STRING_SUCCESS)))
            }
            ("chic_rt", "string_error_message") => {
                let [Value::I32(out_ptr), Value::I32(code)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_error_message expects (i32, i32) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_error_message received negative return pointer".into(),
                })?;
                let message = match *code {
                    STRING_UTF8 => "utf8",
                    STRING_CAPACITY_OVERFLOW => "capacity overflow",
                    STRING_ALLOCATION_FAILED => "allocation failed",
                    STRING_INVALID_POINTER => "invalid pointer",
                    STRING_OUT_OF_BOUNDS => "out of bounds",
                    _ => "success",
                };
                let bytes = message.as_bytes();
                let len = u32::try_from(bytes.len()).unwrap_or(0);
                if len == 0 {
                    self.write_str_ptr(out_ptr, 0, 0)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let ptr = self.allocate_heap_block(len, 1)?;
                self.store_bytes(ptr, 0, bytes)?;
                self.write_str_ptr(out_ptr, ptr, len)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "string_debug_ping") => Ok(Some(Value::I32(42))),
            ("chic_rt", "string_get_ptr") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_ptr expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_ptr received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                if repr.len == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let data_ptr = if self.is_inline_string(&repr) {
                    self.inline_string_ptr(base)?
                } else {
                    repr.ptr
                };
                Ok(Some(Value::I32(data_ptr as i32)))
            }
            ("chic_rt", "string_get_len") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_len expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_len received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            ("chic_rt", "string_get_cap") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_get_cap expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_get_cap received negative pointer".into(),
                })?;
                let repr = self.read_string_repr(base)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            ("chic_rt", "string_set_ptr") => {
                let [Value::I32(ptr), Value::I32(data_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_ptr expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_ptr received negative pointer".into(),
                })?;
                let data_ptr = u32::try_from(*data_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_ptr received negative data pointer".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.ptr = data_ptr;
                if data_ptr != 0 {
                    repr.cap &= !STRING_INLINE_TAG;
                }
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_set_len") => {
                let [Value::I32(ptr), Value::I32(len)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_len expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_len received negative pointer".into(),
                })?;
                let len = u32::try_from(*len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_len received negative length".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.len = len;
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_set_cap") => {
                let [Value::I32(ptr), Value::I32(cap)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_set_cap expects (i32, i32) arguments".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_cap received negative pointer".into(),
                })?;
                let cap = u32::try_from(*cap).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_set_cap received negative cap".into(),
                })?;
                let mut repr = self.read_string_repr(base)?;
                repr.cap = cap;
                self.write_string_repr(base, repr)?;
                Ok(None)
            }
            ("chic_rt", "string_inline_ptr") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_inline_ptr expects a single i32 argument".into(),
                    });
                };
                let base = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_inline_ptr received negative pointer".into(),
                })?;
                let inline_ptr = self.inline_string_ptr(base)?;
                Ok(Some(Value::I32(inline_ptr as i32)))
            }
            ("chic_rt", "string_inline_capacity") => {
                Ok(Some(Value::I32(STRING_INLINE_CAPACITY as i32)))
            }
            ("chic_rt", "string_append_slice") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_slice expects (i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative target pointer".into(),
                })?;
                let slice_ptr = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative slice pointer".into(),
                })?;
                let slice_len = u32::try_from(*slice_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_slice received negative slice length".into(),
                })?;
                if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                    eprintln!(
                        "[wasm-string] string_append_slice target=0x{target_ptr:08X} slice_ptr=0x{slice_ptr:08X} slice_len={slice_len} mem_len={}",
                        self.memory_len()
                    );
                }
                let data = if slice_len == 0 || slice_ptr == 0 {
                    Vec::new()
                } else {
                    self.read_bytes(slice_ptr, slice_len)?
                };
                let status = self.append_string_bytes(target_ptr, &data)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_bool") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_bool expects (i32, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_bool received negative target pointer".into(),
                })?;
                let text = if *value != 0 { "true" } else { "false" };
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_char") => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_char expects (i32, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_char received negative target pointer".into(),
                })?;
                let code = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_char received negative value".into(),
                })?;
                let mut buf = [0u8; 4];
                let bytes = if let Some(ch) = char::from_u32(code) {
                    let encoded = ch.encode_utf8(&mut buf);
                    encoded.as_bytes()
                } else {
                    &[][..]
                };
                let status = self.append_string_bytes(target_ptr, bytes)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_signed") => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_signed expects (i32, i64, i64, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_signed received negative target pointer".into(),
                })?;
                let value = ((*high as i128) << 64) | (*low as u64 as i128);
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_unsigned") => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_unsigned expects (i32, i64, i64, i32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_unsigned received negative target pointer"
                        .into(),
                })?;
                let value = ((u128::from(*high as u64)) << 64) | u128::from(*low as u64);
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_f32") => {
                let [
                    Value::I32(target_ptr),
                    Value::F32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_f32 expects (i32, f32, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_f32 received negative target pointer".into(),
                })?;
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_append_f64") => {
                let [
                    Value::I32(target_ptr),
                    Value::F64(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.string_append_f64 expects (i32, f64, i32, i32, i32, i32) arguments"
                                .into(),
                    });
                };
                let target_ptr = u32::try_from(*target_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_append_f64 received negative target pointer".into(),
                })?;
                let text = value.to_string();
                let status = self.append_string_bytes(target_ptr, text.as_bytes())?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_clone expects (i32, i32) arguments".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone received negative source pointer".into(),
                })?;
                let status = self.clone_string(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_clone_slice") => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_clone_slice expects (i32, i32, i32) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative destination pointer"
                        .into(),
                })?;
                let slice_base = u32::try_from(*slice_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative slice pointer".into(),
                })?;
                let slice_len = u32::try_from(*slice_len).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_clone_slice received negative slice length".into(),
                })?;
                let status = self.clone_string_from_slice(dest, slice_base, slice_len)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "string_drop") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.string_drop expects a single i32 argument".into(),
                    });
                };
                let ptr = u32::try_from(*ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.string_drop received negative pointer".into(),
                })?;
                self.drop_string(ptr)?;
                Ok(None)
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
            ("chic_rt", "vec_drop") => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_drop expects a single i32 argument".into(),
                    });
                };
                let ptr = value_as_ptr_u32(&Value::I32(*ptr), "chic_rt.vec_drop ptr")?;
                self.drop_vec(ptr)?;
                Ok(None)
            }
            ("chic_rt", "vec_with_capacity") => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(capacity),
                    Value::I32(drop_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_with_capacity expects (i32 out, i32 elem_size, i32 elem_align, i32 capacity, i32 drop_fn) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative return pointer".into(),
                })?;
                let elem_size = u32::try_from(*elem_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element size".into(),
                })?;
                let elem_align = u32::try_from(*elem_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element alignment".into(),
                })?;
                let capacity = u32::try_from(*capacity).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative capacity".into(),
                })?;
                let drop_fn = u32::try_from(*drop_fn).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative drop function index"
                        .into(),
                })?;

                let base_align = elem_align.max(1);
                if capacity == 0 || elem_size == 0 {
                    self.write_vec_repr(
                        out_ptr,
                        WasmVecRepr {
                            ptr: 0,
                            len: 0,
                            cap: 0,
                            elem_size,
                            elem_align: base_align,
                            drop_fn,
                        },
                    )?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }

                let bytes = capacity
                    .checked_mul(elem_size)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_with_capacity length overflow".into(),
                    })?;
                let ptr = self.allocate_heap_block(bytes, base_align)?;
                self.write_vec_repr(
                    out_ptr,
                    WasmVecRepr {
                        ptr,
                        len: 0,
                        cap: capacity,
                        elem_size,
                        elem_align: base_align,
                        drop_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            ("chic_rt", "vec_clone") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_clone expects (i32, i32) arguments".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            ("chic_rt", "vec_into_array") | ("chic_rt", "array_into_vec") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_into_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative source pointer".into(),
                })?;
                self.move_vec(dest, src)?;
                Ok(Some(Value::I32(0)))
            }
            ("chic_rt", "vec_copy_to_array") | ("chic_rt", "array_copy_to_vec") => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_copy_to_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative destination pointer"
                        .into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
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
