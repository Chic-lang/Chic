use super::*;

impl<'a> Executor<'a> {
    fn invoke_type_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "type_metadata" | "chic_rt_type_metadata" => {
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
            "type_size" | "chic_rt_type_size" => {
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
            "type_align" | "chic_rt_type_align" => {
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
            "type_drop_glue" | "chic_rt_type_drop_glue" => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_drop_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            "type_clone_glue" | "chic_rt_type_clone_glue" => {
                let [Value::I64(_type_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.type_clone_glue expects a single i64 argument".into(),
                    });
                };
                Ok(Some(Value::I32(0)))
            }
            "type_hash_glue" | "chic_rt_type_hash_glue" => {
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
            "type_eq_glue" | "chic_rt_type_eq_glue" => {
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
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import chic_rt::{name} encountered during execution"
                ),
            }),
        }
    }
}
