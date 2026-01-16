use super::*;

impl<'a> Executor<'a> {
    fn invoke_env_import(
        &mut self,
        name: &str,
        params: &[Value],
        _tracer: &mut SchedulerTracer,
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "write" => {
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
            "read" => {
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
            "isatty" => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.isatty expects (i32 fd)".into(),
                    });
                };
                let result = self.host_isatty(*fd);
                return Ok(Some(Value::I32(result)));
            }
            "monotonic_nanos" => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.monotonic_nanos expects ()".into(),
                    });
                }
                let value = self.host_monotonic_nanos()?;
                return Ok(Some(Value::I64(value)));
            }
            "sleep_millis" => {
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
            "malloc" => {
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
            "calloc" => {
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
            "realloc" => {
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
            "free" => {
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
            "posix_memalign" => {
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
            "memcpy" => {
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
            "memmove" => {
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
            "memset" => {
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
            "fmodf" => {
                let [Value::F32(lhs), Value::F32(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmodf expects (f32 lhs, f32 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F32(lhs % rhs)));
            }
            "fmod" => {
                let [Value::F64(lhs), Value::F64(rhs)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fmod expects (f64 lhs, f64 rhs)".into(),
                    });
                };
                return Ok(Some(Value::F64(lhs % rhs)));
            }
            "fopen" => {
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
            "fread" => {
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
            "fwrite" => {
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
            "fflush" => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fflush expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fflush(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            "fclose" => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fclose expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fclose(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            "fileno" => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fileno expects (i32 stream)".into(),
                    });
                };
                let code = self.host_fileno(*stream)?;
                return Ok(Some(Value::I32(code)));
            }
            "ftell" => {
                let [Value::I32(stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftell expects (i32 stream)".into(),
                    });
                };
                let pos = self.host_ftell(*stream)?;
                return Ok(Some(Value::I64(pos)));
            }
            "ftruncate" => {
                let [Value::I32(stream), length] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.ftruncate expects (i32 stream, i64 length)".into(),
                    });
                };
                let length = value_as_i64(length, "env.ftruncate length")?;
                let code = self.host_ftruncate(*stream, length)?;
                return Ok(Some(Value::I32(code)));
            }
            "fprintf" => {
                if params.len() < 2 {
                    return Err(WasmExecutionError {
                        message: "env.fprintf expects (i32 stream, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            "snprintf" => {
                if params.len() < 3 {
                    return Err(WasmExecutionError {
                        message: "env.snprintf expects (i32 buffer, i32 size, i32 fmt, ...)".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            "fputc" => {
                let [Value::I32(ch), Value::I32(_stream)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.fputc expects (i32 ch, i32 stream)".into(),
                    });
                };
                return Ok(Some(Value::I32(*ch)));
            }
            "pthread_mutex_init" => {
                let [Value::I32(_mutex), Value::I32(_attr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_init expects (i32 mutex, i32 attr)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "pthread_mutex_lock" => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_lock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "pthread_mutex_unlock" => {
                let [Value::I32(_mutex)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_mutex_unlock expects (i32 mutex)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "pthread_create" => {
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
            "pthread_join" => {
                let [_thread, _retval] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_join expects (i32 thread, i32 retval)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "pthread_detach" => {
                let [_thread] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_detach expects (i32 thread)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "pthread_setname_np" => {
                let [_thread, _name] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.pthread_setname_np expects (i32 thread, i32 name)".into(),
                    });
                };
                return Ok(Some(Value::I32(0)));
            }
            "sched_yield" => {
                if !params.is_empty() {
                    return Err(WasmExecutionError {
                        message: "env.sched_yield expects ()".into(),
                    });
                }
                return Ok(Some(Value::I32(0)));
            }
            "clock_gettime" => {
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
            "nanosleep" => {
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
            "accept" => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.accept expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            "bind" => {
                let [_fd, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.bind expects (i32 fd, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            "recvfrom" => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.recvfrom expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            "sendto" => {
                let [_fd, _ptr, _len, _flags, _addr, _addrlen] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.sendto expects (i32 fd, i32 ptr, i32 len, i32 flags, i32 addr, i32 addrlen)".into(),
                    });
                };
                return Ok(Some(Value::I32(-1)));
            }
            "chic_thread_invoke" => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_invoke expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            "chic_thread_drop" => {
                let [_ctx] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.chic_thread_drop expects (i32 ctx)".into(),
                    });
                };
                return Ok(None);
            }
            "socket" => {
                let [Value::I32(domain), Value::I32(typ), Value::I32(proto)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "env.socket expects (i32 domain, i32 type, i32 proto)".into(),
                    });
                };
                let fd = self.host_socket(*domain, *typ, *proto)?;
                return Ok(Some(Value::I32(fd)));
            }
            "connect" => {
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
            "recv" => {
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
            "send" => {
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
            "shutdown" => {
                let [Value::I32(fd), Value::I32(how)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.shutdown expects (i32 fd, i32 how)".into(),
                    });
                };
                let code = self.host_shutdown_socket(*fd, *how)?;
                return Ok(Some(Value::I32(code)));
            }
            "close" => {
                let [Value::I32(fd)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "env.close expects (i32 fd)".into(),
                    });
                };
                let code = self.host_close_socket(*fd)?;
                return Ok(Some(Value::I32(code)));
            }
            "htons" => {
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
            "inet_pton" => {
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
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import env::{name} encountered during execution"
                ),
            }),
        }
    }
}
