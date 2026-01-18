use super::*;

impl<'a> Executor<'a> {
    pub(super) fn invoke_memory_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "alloc" | "chic_rt_alloc" => {
                let [Value::I32(out_ptr), size, align] = params else {
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
            "alloc_zeroed" | "chic_rt_alloc_zeroed" => {
                let [Value::I32(out_ptr), size, align] = params else {
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
            "realloc" | "chic_rt_realloc" => {
                let [Value::I32(out_ptr), ptr, old_size, new_size, align] = params else {
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
            "free" | "chic_rt_free" => {
                let [ptr] = params else {
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
            "memcpy" => {
                let [dest, src, len] = params else {
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
            "chic_rt_memcpy" => {
                let [dest, src, len] = params else {
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
            "memmove" => {
                let [dest, src, len] = params else {
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
            "chic_rt_memmove" => {
                let [dest, src, len] = params else {
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
            "memset" => {
                let [dest, value, len] = params else {
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
            "chic_rt_memset" => {
                let [dest, value, len] = params else {
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
            _ => Err(WasmExecutionError {
                message: format!("unsupported import chic_rt::{name} encountered during execution"),
            }),
        }
    }
}
