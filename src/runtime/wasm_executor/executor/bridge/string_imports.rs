use super::*;

impl<'a> Executor<'a> {
    pub(super) fn invoke_string_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "string_new" => {
                let [Value::I32(out_ptr)] = params else {
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
            "string_with_capacity" => {
                let [Value::I32(out_ptr), Value::I32(capacity)] = params else {
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
            "string_from_slice" => {
                let [Value::I32(out_ptr), Value::I32(slice_ptr)] = params else {
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
            "string_from_char" => {
                let [Value::I32(out_ptr), Value::I32(value)] = params else {
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
            "string_push_slice" => {
                let [Value::I32(target_ptr), Value::I32(slice_ptr)] = params else {
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
            "string_truncate" => {
                let [Value::I32(target_ptr), Value::I32(new_len)] = params else {
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
            "string_reserve" => {
                let [Value::I32(target_ptr), Value::I32(additional)] = params else {
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
            "string_error_message" => {
                let [Value::I32(out_ptr), Value::I32(code)] = params else {
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
            "string_debug_ping" => Ok(Some(Value::I32(42))),
            "string_get_ptr" => {
                let [Value::I32(ptr)] = params else {
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
            "string_get_len" => {
                let [Value::I32(ptr)] = params else {
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
            "string_get_cap" => {
                let [Value::I32(ptr)] = params else {
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
            "string_set_ptr" => {
                let [Value::I32(ptr), Value::I32(data_ptr)] = params else {
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
            "string_set_len" => {
                let [Value::I32(ptr), Value::I32(len)] = params else {
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
            "string_set_cap" => {
                let [Value::I32(ptr), Value::I32(cap)] = params else {
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
            "string_inline_ptr" => {
                let [Value::I32(ptr)] = params else {
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
            "string_inline_capacity" => Ok(Some(Value::I32(STRING_INLINE_CAPACITY as i32))),
            "string_append_slice" => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                ] = params
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
            "string_append_bool" => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_append_char" => {
                let [
                    Value::I32(target_ptr),
                    Value::I32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_append_signed" => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_append_unsigned" => {
                let [
                    Value::I32(target_ptr),
                    Value::I64(low),
                    Value::I64(high),
                    Value::I32(_bits),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_append_f32" => {
                let [
                    Value::I32(target_ptr),
                    Value::F32(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_append_f64" => {
                let [
                    Value::I32(target_ptr),
                    Value::F64(value),
                    Value::I32(_alignment),
                    Value::I32(_has_alignment),
                    Value::I32(_format_ptr),
                    Value::I32(_format_len),
                ] = params
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
            "string_clone" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params else {
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
            "string_clone_slice" => {
                let [
                    Value::I32(dest_ptr),
                    Value::I32(slice_ptr),
                    Value::I32(slice_len),
                ] = params
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
            "string_drop" => {
                let [Value::I32(ptr)] = params else {
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
            _ => Err(WasmExecutionError {
                message: format!("unsupported import chic_rt::{name} encountered during execution"),
            }),
        }
    }
}
