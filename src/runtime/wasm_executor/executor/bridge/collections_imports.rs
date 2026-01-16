use super::*;

impl<'a> Executor<'a> {
    fn invoke_hashset_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "hashset_new" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(drop_fn),
                    Value::I32(eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_new expects (i32 out, i32 elem_size, i32 elem_align, i32 drop_fn, i32 eq_fn) arguments"
                                .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_new out")?;
                let elem_size =
                    value_as_u32(&Value::I32(*elem_size), "chic_rt.hashset_new elem_size")?;
                let elem_align =
                    value_as_u32(&Value::I32(*elem_align), "chic_rt.hashset_new elem_align")?;
                let drop_fn = value_as_u32(&Value::I32(*drop_fn), "chic_rt.hashset_new drop_fn")?;
                let eq_fn = value_as_u32(&Value::I32(*eq_fn), "chic_rt.hashset_new eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
                    let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 20 {
                        let total = self.module.imports.len() + self.module.functions.len();
                        eprintln!(
                            "[wasm-hashset] new[{idx}] out=0x{out_ptr:08x} elem_size={elem_size} elem_align={elem_align} drop_fn={drop_fn} eq_fn={eq_fn} total_funcs={total}"
                        );
                    }
                }
                self.write_hashset_repr(
                    out_ptr,
                    WasmHashSetRepr {
                        entries: 0,
                        states: 0,
                        hashes: 0,
                        len: 0,
                        cap: 0,
                        tombstones: 0,
                        elem_size,
                        elem_align: elem_align.max(1),
                        drop_fn,
                        eq_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashset_with_capacity" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(capacity),
                    Value::I32(drop_fn),
                    Value::I32(eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_with_capacity expects (i32 out, i32 elem_size, i32 elem_align, i32 cap, i32 drop_fn, i32 eq_fn) arguments"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_with_capacity out")?;
                let elem_size = value_as_u32(
                    &Value::I32(*elem_size),
                    "chic_rt.hashset_with_capacity elem_size",
                )?;
                let elem_align = value_as_u32(
                    &Value::I32(*elem_align),
                    "chic_rt.hashset_with_capacity elem_align",
                )?;
                let capacity =
                    value_as_u32(&Value::I32(*capacity), "chic_rt.hashset_with_capacity cap")?;
                let drop_fn = value_as_u32(
                    &Value::I32(*drop_fn),
                    "chic_rt.hashset_with_capacity drop_fn",
                )?;
                let eq_fn =
                    value_as_u32(&Value::I32(*eq_fn), "chic_rt.hashset_with_capacity eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let elem_align = elem_align.max(1);
                let mut repr = WasmHashSetRepr {
                    entries: 0,
                    states: 0,
                    hashes: 0,
                    len: 0,
                    cap: 0,
                    tombstones: 0,
                    elem_size,
                    elem_align,
                    drop_fn,
                    eq_fn,
                };
                let normalized = if capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(capacity).unwrap_or(0)
                };
                if normalized != 0 {
                    let entry_bytes = normalized.saturating_mul(elem_size);
                    let hash_bytes = normalized.saturating_mul(8);
                    repr.entries = self.allocate_heap_block(entry_bytes, elem_align)?;
                    repr.states = self.allocate_heap_block(normalized, 1)?;
                    repr.hashes = self.allocate_heap_block(hash_bytes, 8)?;
                    repr.cap = normalized;
                }
                self.write_hashset_repr(out_ptr, repr)?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashset_len" => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_len expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_len set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            "hashset_capacity" => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_capacity expects (i32 set)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_capacity set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            "hashset_tombstones" => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_tombstones expects (i32 set)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_tombstones set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                Ok(Some(Value::I32(repr.tombstones as i32)))
            }
            "hashset_clear" => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_clear expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_clear set")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 {
                    repr.len = 0;
                    repr.tombstones = 0;
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.states != 0 && repr.entries != 0 && repr.elem_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                            self.hashset_drop_value(&repr, entry_ptr)?;
                        }
                    }
                    self.fill(repr.states, 0, repr.cap, TABLE_STATE_EMPTY)?;
                    if repr.hashes != 0 {
                        let hash_bytes = repr.cap.saturating_mul(8);
                        self.fill(repr.hashes, 0, hash_bytes, 0)?;
                    }
                }
                repr.len = 0;
                repr.tombstones = 0;
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_drop" => {
                let [Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_drop expects (i32 set)".into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_drop set")?;
                if set_ptr == 0 {
                    return Ok(None);
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap != 0 && repr.states != 0 && repr.entries != 0 && repr.elem_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                            self.hashset_drop_value(&repr, entry_ptr)?;
                        }
                    }
                }
                repr.entries = 0;
                repr.states = 0;
                repr.hashes = 0;
                repr.len = 0;
                repr.cap = 0;
                repr.tombstones = 0;
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(None)
            }
            "hashset_reserve" => {
                let [Value::I32(set_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_reserve expects (i32 set, i32 additional)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_reserve set")?;
                let additional = value_as_u32(
                    &Value::I32(*additional),
                    "chic_rt.hashset_reserve additional",
                )?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if !self.table_should_grow(repr.len, repr.tombstones, repr.cap, additional) {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let needed = match repr.len.checked_add(additional) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let doubled = match needed.checked_add(needed) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                    return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                };
                let rebuilt = self.hashset_rehash(&repr, target)?;
                self.write_hashset_repr(set_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_shrink_to" => {
                let [Value::I32(set_ptr), Value::I32(min_capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_shrink_to expects (i32 set, i32 min_capacity)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_shrink_to set")?;
                let min_capacity = value_as_u32(
                    &Value::I32(*min_capacity),
                    "chic_rt.hashset_shrink_to min_capacity",
                )?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                let min_cap = if min_capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(min_capacity)
                        .ok_or_else(|| WasmExecutionError {
                            message: "hashset shrink_to min capacity overflow".into(),
                        })?
                };
                let mut desired = min_cap;
                if repr.len != 0 {
                    let doubled = match repr.len.checked_add(repr.len) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let expanded = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    desired = match self.table_round_up_pow2(expanded) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                }
                let target = desired.max(min_cap);
                if target >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if target == 0 {
                    repr.entries = 0;
                    repr.states = 0;
                    repr.hashes = 0;
                    repr.len = 0;
                    repr.cap = 0;
                    repr.tombstones = 0;
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let rebuilt = self.hashset_rehash(&repr, target)?;
                self.write_hashset_repr(set_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_insert" => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(value_ptr),
                    Value::I32(inserted_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_insert expects (i32 set, i64 hash, i32 value, i32 inserted)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_insert set")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashset_insert value")?;
                let inserted_ptr = value_as_ptr_u32(
                    &Value::I32(*inserted_ptr),
                    "chic_rt.hashset_insert inserted",
                )?;
                if inserted_ptr != 0 {
                    let _ = self.write_u32(inserted_ptr, 0);
                }
                if set_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let status = {
                        let Some(needed) = repr.len.checked_add(1) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(doubled) = needed.checked_add(needed) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(desired) = doubled.checked_add(TABLE_MIN_CAPACITY) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let target = self.table_round_up_pow2(desired).unwrap_or(0);
                        if target == 0 {
                            TABLE_CAPACITY_OVERFLOW
                        } else {
                            let rebuilt = self.hashset_rehash(&repr, target)?;
                            self.write_hashset_repr(set_ptr, rebuilt)?;
                            TABLE_SUCCESS
                        }
                    };
                    if status != TABLE_SUCCESS {
                        return Ok(Some(Value::I32(status)));
                    }
                    repr = self.read_hashset_repr(set_ptr)?;
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, value_data)?;
                if found {
                    if inserted_ptr != 0 {
                        let _ = self.write_u32(inserted_ptr, 0);
                    }
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let dest_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if repr.elem_size != 0 && dest_ptr != 0 && value_data != 0 {
                    let bytes = self.read_bytes(value_data, repr.elem_size)?;
                    self.store_bytes(dest_ptr, 0, &bytes)?;
                }
                let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if inserted_ptr != 0 {
                    let _ = self.write_u32(inserted_ptr, 1);
                }
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_replace" => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(value_ptr),
                    Value::I32(dest_ptr),
                    Value::I32(replaced_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_replace expects (i32 set, i64 hash, i32 value, i32 dest, i32 replaced)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_replace set")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashset_replace value")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_replace dest")?;
                let replaced_ptr = value_as_ptr_u32(
                    &Value::I32(*replaced_ptr),
                    "chic_rt.hashset_replace replaced",
                )?;
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                if set_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let status = {
                        let Some(needed) = repr.len.checked_add(1) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(doubled) = needed.checked_add(needed) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let Some(desired) = doubled.checked_add(TABLE_MIN_CAPACITY) else {
                            return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                        };
                        let target = self.table_round_up_pow2(desired).unwrap_or(0);
                        if target == 0 {
                            TABLE_CAPACITY_OVERFLOW
                        } else {
                            let rebuilt = self.hashset_rehash(&repr, target)?;
                            self.write_hashset_repr(set_ptr, rebuilt)?;
                            TABLE_SUCCESS
                        }
                    };
                    if status != TABLE_SUCCESS {
                        return Ok(Some(Value::I32(status)));
                    }
                    repr = self.read_hashset_repr(set_ptr)?;
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, value_data)?;
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if found {
                    if dest_ptr != 0 {
                        let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                        if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                            let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                            self.store_bytes(out_data, 0, &bytes)?;
                        }
                    }
                    self.hashset_drop_value(&repr, entry_ptr)?;
                    if repr.elem_size != 0 && entry_ptr != 0 && value_data != 0 {
                        let bytes = self.read_bytes(value_data, repr.elem_size)?;
                        self.store_bytes(entry_ptr, 0, &bytes)?;
                    }
                    let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                    if hash_slot != 0 {
                        self.write_u64(hash_slot, *hash as u64)?;
                    }
                    if replaced_ptr != 0 {
                        let _ = self.write_u32(replaced_ptr, 1);
                    }
                    self.write_hashset_repr(set_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.elem_size != 0 && entry_ptr != 0 && value_data != 0 {
                    let bytes = self.read_bytes(value_data, repr.elem_size)?;
                    self.store_bytes(entry_ptr, 0, &bytes)?;
                }
                let hash_slot = self.hashset_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_contains" => {
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_contains expects (i32 set, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_contains set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_contains key")?;
                if set_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                if key_data == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                let (found, _) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                Ok(Some(Value::I32(if found { 1 } else { 0 })))
            }
            "hashset_get_ptr" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_get_ptr expects (i32 out, i32 set, i64 hash, i32 key)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_get_ptr out")?;
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_get_ptr set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_get_ptr key")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if set_ptr == 0 || key_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                if key_data == 0 || repr.cap == 0 {
                    self.write_value_ptr(out_ptr, 0, repr.elem_size, repr.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    self.write_value_ptr(out_ptr, 0, repr.elem_size, repr.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                self.write_value_ptr(out_ptr, entry_ptr, repr.elem_size, repr.elem_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashset_take" => {
                let [
                    Value::I32(set_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashset_take expects (i32 set, i64 hash, i32 key, i32 dest)"
                                .into(),
                    });
                };
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_take set")?;
                let key_ptr = value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_take key")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_take dest")?;
                if set_ptr == 0 || key_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                    let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                    self.store_bytes(out_data, 0, &bytes)?;
                }
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_remove" => {
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_remove expects (i32 set, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_remove set")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashset_remove key")?;
                if set_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (found, index) = self.hashset_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(0)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(1)))
            }
            "hashset_take_at" => {
                let [Value::I32(set_ptr), Value::I32(index), Value::I32(dest_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_take_at expects (i32 set, i32 index, i32 dest)"
                            .into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_take_at set")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashset_take_at index")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_take_at dest")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashset_repr(set_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
                    let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 120 {
                        if idx < 5 {
                            if let Ok(bytes) = self.read_bytes(set_ptr, 40) {
                                let mut hex = String::new();
                                for (i, b) in bytes.iter().enumerate() {
                                    if i != 0 {
                                        hex.push(' ');
                                    }
                                    hex.push_str(&format!("{b:02x}"));
                                }
                                eprintln!("[wasm-hashset] take_at[{idx}] raw={hex}");
                                let maybe_ptr =
                                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                                if maybe_ptr != 0 {
                                    if let Ok(deref) = self.read_bytes(maybe_ptr, 40) {
                                        let mut hex2 = String::new();
                                        for (i, b) in deref.iter().enumerate() {
                                            if i != 0 {
                                                hex2.push(' ');
                                            }
                                            hex2.push_str(&format!("{b:02x}"));
                                        }
                                        eprintln!(
                                            "[wasm-hashset] take_at[{idx}] maybe_ptr=0x{maybe_ptr:08x} deref40={hex2}"
                                        );
                                    }
                                }
                            }
                        }
                        eprintln!(
                            "[wasm-hashset] take_at[{idx}] set=0x{set_ptr:08x} index={index} cap={} len={} tombstones={} drop_fn={} eq_fn={}",
                            repr.cap, repr.len, repr.tombstones, repr.drop_fn, repr.eq_fn
                        );
                    }
                }
                if repr.cap == 0 || index >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let state = self.read_u8(state_addr)?;
                if state != TABLE_STATE_FULL {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, index)?;
                if dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                    if out_data != 0 && repr.elem_size != 0 && entry_ptr != 0 {
                        let bytes = self.read_bytes(entry_ptr, repr.elem_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                self.hashset_drop_value(&repr, entry_ptr)?;
                self.write_u8(state_addr, TABLE_STATE_TOMBSTONE)?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashset_repr(set_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashset_bucket_state" => {
                let [Value::I32(set_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_bucket_state expects (i32 set, i32 index)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_bucket_state set")?;
                let index =
                    value_as_u32(&Value::I32(*index), "chic_rt.hashset_bucket_state index")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.states == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let state = self.read_u8(repr.states.checked_add(index).unwrap_or(0))?;
                Ok(Some(Value::I32(i32::from(state))))
            }
            "hashset_bucket_hash" => {
                let [Value::I32(set_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_bucket_hash expects (i32 set, i32 index)".into(),
                    });
                };
                let set_ptr =
                    value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_bucket_hash set")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashset_bucket_hash index")?;
                if set_ptr == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.hashes == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let slot = self.hashset_hash_slot(repr.hashes, index)?;
                let value = if slot == 0 { 0 } else { self.read_u64(slot)? };
                Ok(Some(Value::I64(value as i64)))
            }
            "hashset_iter" => {
                let [Value::I32(out_ptr), Value::I32(set_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter expects (i32 out, i32 set)".into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_iter out")?;
                let set_ptr = value_as_ptr_u32(&Value::I32(*set_ptr), "chic_rt.hashset_iter set")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if set_ptr == 0 {
                    self.write_hashset_iter_repr(
                        out_ptr,
                        WasmHashSetIterRepr {
                            entries: 0,
                            states: 0,
                            index: 0,
                            cap: 0,
                            elem_size: 0,
                            elem_align: 1,
                        },
                    )?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashset_repr(set_ptr)?;
                self.write_hashset_iter_repr(
                    out_ptr,
                    WasmHashSetIterRepr {
                        entries: repr.entries,
                        states: repr.states,
                        index: 0,
                        cap: repr.cap,
                        elem_size: repr.elem_size,
                        elem_align: repr.elem_align.max(1),
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashset_iter_next" => {
                let [Value::I32(iter_ptr), Value::I32(dest_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter_next expects (i32 iter, i32 dest)".into(),
                    });
                };
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashset_iter_next iter")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashset_iter_next dest")?;
                if iter_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut iter = self.read_hashset_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    return Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashset_entry_ptr(iter.entries, iter.elem_size, idx)?;
                        let (out_data, _, _) = self.read_value_ptr(dest_ptr)?;
                        if out_data != 0 && iter.elem_size != 0 && entry_ptr != 0 {
                            let bytes = self.read_bytes(entry_ptr, iter.elem_size)?;
                            self.store_bytes(out_data, 0, &bytes)?;
                        }
                        self.write_hashset_iter_repr(iter_ptr, iter)?;
                        return Ok(Some(Value::I32(TABLE_SUCCESS)));
                    }
                }
                self.write_hashset_iter_repr(iter_ptr, iter)?;
                Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)))
            }
            "hashset_iter_next_ptr" => {
                let [Value::I32(out_ptr), Value::I32(iter_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashset_iter_next_ptr expects (i32 out, i32 iter)".into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashset_iter_next_ptr out")?;
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashset_iter_next_ptr iter")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if iter_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let mut iter = self.read_hashset_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    self.write_value_ptr(out_ptr, 0, iter.elem_size, iter.elem_align.max(1))?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashset_entry_ptr(iter.entries, iter.elem_size, idx)?;
                        self.write_hashset_iter_repr(iter_ptr, iter)?;
                        self.write_value_ptr(
                            out_ptr,
                            entry_ptr,
                            iter.elem_size,
                            iter.elem_align.max(1),
                        )?;
                        return Ok(Some(Value::I32(out_ptr as i32)));
                    }
                }
                self.write_hashset_iter_repr(iter_ptr, iter)?;
                self.write_value_ptr(out_ptr, 0, iter.elem_size, iter.elem_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import chic_rt::{name} encountered during execution"
                ),
            }),
        }
    }

    fn invoke_hashmap_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "hashmap_new" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(key_size),
                    Value::I32(key_align),
                    Value::I32(value_size),
                    Value::I32(value_align),
                    Value::I32(key_drop_fn),
                    Value::I32(value_drop_fn),
                    Value::I32(key_eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_new expects (i32 out, i32 key_size, i32 key_align, i32 value_size, i32 value_align, i32 key_drop_fn, i32 value_drop_fn, i32 key_eq_fn)"
                                .into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_new out")?;
                let key_size =
                    value_as_u32(&Value::I32(*key_size), "chic_rt.hashmap_new key_size")?;
                let key_align =
                    value_as_u32(&Value::I32(*key_align), "chic_rt.hashmap_new key_align")?.max(1);
                let value_size =
                    value_as_u32(&Value::I32(*value_size), "chic_rt.hashmap_new value_size")?;
                let value_align =
                    value_as_u32(&Value::I32(*value_align), "chic_rt.hashmap_new value_align")?
                        .max(1);
                let key_drop_fn =
                    value_as_u32(&Value::I32(*key_drop_fn), "chic_rt.hashmap_new key_drop_fn")?;
                let value_drop_fn = value_as_u32(
                    &Value::I32(*value_drop_fn),
                    "chic_rt.hashmap_new value_drop_fn",
                )?;
                let key_eq_fn =
                    value_as_u32(&Value::I32(*key_eq_fn), "chic_rt.hashmap_new key_eq_fn")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let offset = self.align_up(key_size, value_align).unwrap_or(key_size);
                self.write_hashmap_repr(
                    out_ptr,
                    WasmHashMapRepr {
                        entries: 0,
                        states: 0,
                        hashes: 0,
                        len: 0,
                        cap: 0,
                        tombstones: 0,
                        key_size,
                        key_align,
                        value_size,
                        value_align,
                        entry_size: offset.saturating_add(value_size),
                        value_offset: offset,
                        key_drop_fn,
                        value_drop_fn,
                        key_eq_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashmap_with_capacity" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(key_size),
                    Value::I32(key_align),
                    Value::I32(value_size),
                    Value::I32(value_align),
                    Value::I32(capacity),
                    Value::I32(key_drop_fn),
                    Value::I32(value_drop_fn),
                    Value::I32(key_eq_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_with_capacity expects (i32 out, i32 key_size, i32 key_align, i32 value_size, i32 value_align, i32 cap, i32 key_drop_fn, i32 value_drop_fn, i32 key_eq_fn)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_with_capacity out")?;
                let key_size = value_as_u32(
                    &Value::I32(*key_size),
                    "chic_rt.hashmap_with_capacity key_size",
                )?;
                let key_align = value_as_u32(
                    &Value::I32(*key_align),
                    "chic_rt.hashmap_with_capacity key_align",
                )?
                .max(1);
                let value_size = value_as_u32(
                    &Value::I32(*value_size),
                    "chic_rt.hashmap_with_capacity value_size",
                )?;
                let value_align = value_as_u32(
                    &Value::I32(*value_align),
                    "chic_rt.hashmap_with_capacity value_align",
                )?
                .max(1);
                let capacity =
                    value_as_u32(&Value::I32(*capacity), "chic_rt.hashmap_with_capacity cap")?;
                let key_drop_fn = value_as_u32(
                    &Value::I32(*key_drop_fn),
                    "chic_rt.hashmap_with_capacity key_drop_fn",
                )?;
                let value_drop_fn = value_as_u32(
                    &Value::I32(*value_drop_fn),
                    "chic_rt.hashmap_with_capacity value_drop_fn",
                )?;
                let key_eq_fn = value_as_u32(
                    &Value::I32(*key_eq_fn),
                    "chic_rt.hashmap_with_capacity key_eq_fn",
                )?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 16 {
                        eprintln!(
                            "[wasm-hashmap] with_capacity[{idx}] out=0x{out_ptr:08X} key_size={key_size} key_align={key_align} value_size={value_size} value_align={value_align} cap={capacity} key_drop=0x{key_drop_fn:08X} value_drop=0x{value_drop_fn:08X} key_eq=0x{key_eq_fn:08X}",
                        );
                    }
                }
                let offset = self.align_up(key_size, value_align).unwrap_or(key_size);
                let entry_size = offset.saturating_add(value_size);
                let mut repr = WasmHashMapRepr {
                    entries: 0,
                    states: 0,
                    hashes: 0,
                    len: 0,
                    cap: 0,
                    tombstones: 0,
                    key_size,
                    key_align,
                    value_size,
                    value_align,
                    entry_size,
                    value_offset: offset,
                    key_drop_fn,
                    value_drop_fn,
                    key_eq_fn,
                };
                let normalized = if capacity == 0 {
                    0
                } else {
                    self.table_round_up_pow2(capacity).unwrap_or(0)
                };
                if normalized != 0 {
                    let max_align = key_align.max(value_align).max(1);
                    let entry_bytes = normalized.saturating_mul(entry_size);
                    let hash_bytes = normalized.saturating_mul(8);
                    repr.entries = self.allocate_heap_block(entry_bytes, max_align)?;
                    repr.states = self.allocate_heap_block(normalized, 1)?;
                    repr.hashes = self.allocate_heap_block(hash_bytes, 8)?;
                    repr.cap = normalized;
                }
                self.write_hashmap_repr(out_ptr, repr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER
                        .load(Ordering::Relaxed)
                        .saturating_sub(1);
                    if idx < 16 {
                        let written = self.read_hashmap_repr(out_ptr).unwrap_or_default();
                        eprintln!(
                            "[wasm-hashmap] with_capacity[{idx}] wrote out=0x{out_ptr:08X} cap={} len={} key_size={} value_size={} entry_size={} entries=0x{:08X} states=0x{:08X} hashes=0x{:08X}",
                            written.cap,
                            written.len,
                            written.key_size,
                            written.value_size,
                            written.entry_size,
                            written.entries,
                            written.states,
                            written.hashes
                        );
                    }
                }
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashmap_drop" => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_drop expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_drop map")?;
                if map_ptr == 0 {
                    return Ok(None);
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap != 0 && repr.states != 0 && repr.entries != 0 && repr.entry_size != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                            self.hashmap_drop_entry(&repr, entry_ptr)?;
                        }
                    }
                }
                repr.entries = 0;
                repr.states = 0;
                repr.hashes = 0;
                repr.len = 0;
                repr.cap = 0;
                repr.tombstones = 0;
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(None)
            }
            "hashmap_clear" => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_clear expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_clear map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 {
                    repr.len = 0;
                    repr.tombstones = 0;
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.states != 0 && repr.entries != 0 {
                    for idx in 0..repr.cap {
                        let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                        if state == TABLE_STATE_FULL {
                            let entry_ptr =
                                self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                            self.hashmap_drop_entry(&repr, entry_ptr)?;
                        }
                    }
                    self.fill(repr.states, 0, repr.cap, TABLE_STATE_EMPTY)?;
                    if repr.hashes != 0 {
                        let hash_bytes = repr.cap.saturating_mul(8);
                        self.fill(repr.hashes, 0, hash_bytes, 0)?;
                    }
                }
                repr.len = 0;
                repr.tombstones = 0;
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_len" => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_len expects (i32 map)".into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_len map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                Ok(Some(Value::I32(repr.len as i32)))
            }
            "hashmap_capacity" => {
                let [Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_capacity expects (i32 map)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_capacity map")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                Ok(Some(Value::I32(repr.cap as i32)))
            }
            "hashmap_reserve" => {
                let [Value::I32(map_ptr), Value::I32(additional)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_reserve expects (i32 map, i32 additional)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_reserve map")?;
                let additional = value_as_u32(
                    &Value::I32(*additional),
                    "chic_rt.hashmap_reserve additional",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if !self.table_should_grow(repr.len, repr.tombstones, repr.cap, additional) {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let needed = match repr.len.checked_add(additional) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let doubled = match needed.checked_add(needed) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                    Some(v) => v,
                    None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                };
                let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                    return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                };
                let rebuilt = self.hashmap_rehash(&repr, target)?;
                self.write_hashmap_repr(map_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_shrink_to" => {
                let [Value::I32(map_ptr), Value::I32(min_capacity)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_shrink_to expects (i32 map, i32 min_capacity)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_shrink_to map")?;
                let min_capacity = value_as_u32(
                    &Value::I32(*min_capacity),
                    "chic_rt.hashmap_shrink_to min_capacity",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let min_cap = if min_capacity == 0 {
                    0
                } else {
                    match self.table_round_up_pow2(min_capacity) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    }
                };
                let mut desired = min_cap;
                if repr.len != 0 {
                    let doubled = match repr.len.checked_add(repr.len) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let expanded = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    desired = match self.table_round_up_pow2(expanded) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                }
                let target = desired.max(min_cap);
                if target >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if target == 0 {
                    repr.entries = 0;
                    repr.states = 0;
                    repr.hashes = 0;
                    repr.len = 0;
                    repr.cap = 0;
                    repr.tombstones = 0;
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                let rebuilt = self.hashmap_rehash(&repr, target)?;
                self.write_hashmap_repr(map_ptr, rebuilt)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_insert" => {
                let [
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(value_ptr),
                    Value::I32(previous_value_ptr),
                    Value::I32(replaced_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_insert expects (i32 map, i64 hash, i32 key, i32 value, i32 prev, i32 replaced)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_insert map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_insert key")?;
                let value_ptr =
                    value_as_ptr_u32(&Value::I32(*value_ptr), "chic_rt.hashmap_insert value")?;
                let previous_value_ptr = value_as_ptr_u32(
                    &Value::I32(*previous_value_ptr),
                    "chic_rt.hashmap_insert prev",
                )?;
                let replaced_ptr = value_as_ptr_u32(
                    &Value::I32(*replaced_ptr),
                    "chic_rt.hashmap_insert replaced",
                )?;
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                if map_ptr == 0 || key_ptr == 0 || value_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if idx < 16 {
                        let key_words = (
                            self.read_u32(key_ptr).unwrap_or(0),
                            self.read_u32(key_ptr + 4).unwrap_or(0),
                            self.read_u32(key_ptr + 8).unwrap_or(0),
                        );
                        let value_words = (
                            self.read_u32(value_ptr).unwrap_or(0),
                            self.read_u32(value_ptr + 4).unwrap_or(0),
                            self.read_u32(value_ptr + 8).unwrap_or(0),
                        );
                        eprintln!(
                            "[wasm-hashmap] insert[{idx}] map=0x{map_ptr:08X} hash=0x{hash:016X} key_handle=0x{key_ptr:08X} key={{ptr=0x{:08X} size={} align={}}} value_handle=0x{value_ptr:08X} value={{ptr=0x{:08X} size={} align={}}} prev=0x{previous_value_ptr:08X} replaced=0x{replaced_ptr:08X}",
                            key_words.0,
                            key_words.1,
                            key_words.2,
                            value_words.0,
                            value_words.1,
                            value_words.2
                        );
                    }
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (value_data, _, _) = self.read_value_ptr(value_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if std::env::var_os("CHIC_DEBUG_WASM_HASHMAP").is_some() {
                    let idx = HASHMAP_DEBUG_COUNTER
                        .load(Ordering::Relaxed)
                        .saturating_sub(1);
                    if idx < 16 {
                        let key_preview = self.read_bytes(key_data, 32).unwrap_or_default();
                        let value_preview = self.read_bytes(value_data, 32).unwrap_or_default();
                        eprintln!(
                            "[wasm-hashmap] insert[{idx}] repr cap={} len={} tomb={} key_size={} key_align={} value_size={} value_align={} entry_size={} value_off={} entries=0x{:08X} states=0x{:08X} hashes=0x{:08X} key_eq=0x{:08X} key_data=0x{key_data:08X} key_bytes={:02X?} value_data=0x{value_data:08X} value_bytes={:02X?}",
                            repr.cap,
                            repr.len,
                            repr.tombstones,
                            repr.key_size,
                            repr.key_align,
                            repr.value_size,
                            repr.value_align,
                            repr.entry_size,
                            repr.value_offset,
                            repr.entries,
                            repr.states,
                            repr.hashes,
                            repr.key_eq_fn,
                            key_preview,
                            value_preview
                        );
                    }
                }
                if repr.cap == 0 || self.table_should_grow(repr.len, repr.tombstones, repr.cap, 1) {
                    let needed = match repr.len.checked_add(1) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let doubled = match needed.checked_add(needed) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let desired = match doubled.checked_add(TABLE_MIN_CAPACITY) {
                        Some(v) => v,
                        None => return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW))),
                    };
                    let Some(target) = self.table_round_up_pow2(desired).filter(|v| *v != 0) else {
                        return Ok(Some(Value::I32(TABLE_CAPACITY_OVERFLOW)));
                    };
                    let rebuilt = self.hashmap_rehash(&repr, target)?;
                    self.write_hashmap_repr(map_ptr, rebuilt)?;
                    repr = self.read_hashmap_repr(map_ptr)?;
                }
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if found {
                    if previous_value_ptr != 0 {
                        let (prev_data, prev_size, _) = self.read_value_ptr(previous_value_ptr)?;
                        if prev_data != 0 && prev_size != 0 && repr.value_size != 0 {
                            let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                            let bytes = self.read_bytes(value_src, repr.value_size)?;
                            self.store_bytes(prev_data, 0, &bytes)?;
                        } else if prev_size == 0 && repr.value_drop_fn != 0 {
                            let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                            let _ =
                                self.invoke(repr.value_drop_fn, &[Value::I32(value_src as i32)])?;
                        }
                    } else if repr.value_drop_fn != 0 {
                        let value_src = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let _ = self.invoke(repr.value_drop_fn, &[Value::I32(value_src as i32)])?;
                    }
                    if repr.key_size != 0 && key_data != 0 {
                        let key_bytes = self.read_bytes(key_data, repr.key_size)?;
                        self.store_bytes(entry_ptr, 0, &key_bytes)?;
                    }
                    if repr.value_size != 0 && value_data != 0 {
                        let value_dst = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let value_bytes = self.read_bytes(value_data, repr.value_size)?;
                        self.store_bytes(value_dst, 0, &value_bytes)?;
                    }
                    if replaced_ptr != 0 {
                        let _ = self.write_u32(replaced_ptr, 1);
                    }
                    self.write_hashmap_repr(map_ptr, repr)?;
                    return Ok(Some(Value::I32(TABLE_SUCCESS)));
                }
                if repr.key_size != 0 && key_data != 0 {
                    let key_bytes = self.read_bytes(key_data, repr.key_size)?;
                    self.store_bytes(entry_ptr, 0, &key_bytes)?;
                }
                if repr.value_size != 0 && value_data != 0 {
                    let value_dst = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                    let value_bytes = self.read_bytes(value_data, repr.value_size)?;
                    self.store_bytes(value_dst, 0, &value_bytes)?;
                }
                let hash_slot = self.hashmap_hash_slot(repr.hashes, index)?;
                if hash_slot != 0 {
                    self.write_u64(hash_slot, *hash as u64)?;
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let prior = self.read_u8(state_addr)?;
                if prior == TABLE_STATE_TOMBSTONE && repr.tombstones != 0 {
                    repr.tombstones -= 1;
                }
                self.write_u8(state_addr, TABLE_STATE_FULL)?;
                repr.len = repr.len.saturating_add(1);
                if replaced_ptr != 0 {
                    let _ = self.write_u32(replaced_ptr, 0);
                }
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_contains" => {
                let [Value::I32(map_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_contains expects (i32 map, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_contains map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_contains key")?;
                if map_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let repr = self.read_hashmap_repr(map_ptr)?;
                let (found, _) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                Ok(Some(Value::I32(if found { 1 } else { 0 })))
            }
            "hashmap_get_ptr" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_get_ptr expects (i32 out, i32 map, i64 hash, i32 key)"
                                .into(),
                    });
                };
                let out_ptr =
                    value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_get_ptr out")?;
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_get_ptr map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_get_ptr key")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if map_ptr == 0 || key_ptr == 0 {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    self.write_value_ptr(out_ptr, 0, 0, 1)?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                self.write_value_ptr(out_ptr, value_ptr, repr.value_size, repr.value_align.max(1))?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashmap_take" => {
                let [
                    Value::I32(map_ptr),
                    Value::I64(hash),
                    Value::I32(key_ptr),
                    Value::I32(dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message:
                            "chic_rt.hashmap_take expects (i32 map, i64 hash, i32 key, i32 dest)"
                                .into(),
                    });
                };
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_take map")?;
                let key_ptr = value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_take key")?;
                let dest_ptr =
                    value_as_ptr_u32(&Value::I32(*dest_ptr), "chic_rt.hashmap_take dest")?;
                if map_ptr == 0 || key_ptr == 0 || dest_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let (out_data, out_size, _) = self.read_value_ptr(dest_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if out_size != 0 && out_data != 0 && repr.value_size != 0 {
                    let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                    let bytes = self.read_bytes(value_ptr, repr.value_size)?;
                    self.store_bytes(out_data, 0, &bytes)?;
                }
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_remove" => {
                let [Value::I32(map_ptr), Value::I64(hash), Value::I32(key_ptr)] =
                    params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_remove expects (i32 map, i64 hash, i32 key)"
                            .into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_remove map")?;
                let key_ptr =
                    value_as_ptr_u32(&Value::I32(*key_ptr), "chic_rt.hashmap_remove key")?;
                if map_ptr == 0 || key_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let (key_data, _, _) = self.read_value_ptr(key_ptr)?;
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                let (found, index) = self.hashmap_find_slot(&repr, *hash as u64, key_data)?;
                if !found {
                    return Ok(Some(Value::I32(0)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(
                    repr.states.checked_add(index).unwrap_or(0),
                    TABLE_STATE_TOMBSTONE,
                )?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(1)))
            }
            "hashmap_bucket_state" => {
                let [Value::I32(map_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_bucket_state expects (i32 map, i32 index)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_bucket_state map")?;
                let index =
                    value_as_u32(&Value::I32(*index), "chic_rt.hashmap_bucket_state index")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.states == 0 {
                    return Ok(Some(Value::I32(TABLE_STATE_EMPTY as i32)));
                }
                let state = self.read_u8(repr.states.checked_add(index).unwrap_or(0))?;
                Ok(Some(Value::I32(i32::from(state))))
            }
            "hashmap_bucket_hash" => {
                let [Value::I32(map_ptr), Value::I32(index)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_bucket_hash expects (i32 map, i32 index)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_bucket_hash map")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashmap_bucket_hash index")?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap || repr.hashes == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let slot = self.hashmap_hash_slot(repr.hashes, index)?;
                let value = if slot == 0 { 0 } else { self.read_u64(slot)? };
                Ok(Some(Value::I64(value as i64)))
            }
            "hashmap_take_at" => {
                let [
                    Value::I32(map_ptr),
                    Value::I32(index),
                    Value::I32(key_dest_ptr),
                    Value::I32(value_dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_take_at expects (i32 map, i32 index, i32 key_dest, i32 value_dest)".into(),
                    });
                };
                let map_ptr =
                    value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_take_at map")?;
                let index = value_as_u32(&Value::I32(*index), "chic_rt.hashmap_take_at index")?;
                let key_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*key_dest_ptr),
                    "chic_rt.hashmap_take_at key_dest",
                )?;
                let value_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*value_dest_ptr),
                    "chic_rt.hashmap_take_at value_dest",
                )?;
                if map_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut repr = self.read_hashmap_repr(map_ptr)?;
                if repr.cap == 0 || index >= repr.cap {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let state_addr = repr.states.checked_add(index).unwrap_or(0);
                let state = self.read_u8(state_addr)?;
                if state != TABLE_STATE_FULL {
                    return Ok(Some(Value::I32(TABLE_NOT_FOUND)));
                }
                let entry_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, index)?;
                if key_dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(key_dest_ptr)?;
                    if out_data != 0 && repr.key_size != 0 {
                        let bytes = self.read_bytes(entry_ptr, repr.key_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                if value_dest_ptr != 0 {
                    let (out_data, _, _) = self.read_value_ptr(value_dest_ptr)?;
                    if out_data != 0 && repr.value_size != 0 {
                        let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
                        let bytes = self.read_bytes(value_ptr, repr.value_size)?;
                        self.store_bytes(out_data, 0, &bytes)?;
                    }
                }
                self.hashmap_drop_entry(&repr, entry_ptr)?;
                self.write_u8(state_addr, TABLE_STATE_TOMBSTONE)?;
                if repr.len != 0 {
                    repr.len -= 1;
                }
                repr.tombstones = repr.tombstones.saturating_add(1);
                self.write_hashmap_repr(map_ptr, repr)?;
                Ok(Some(Value::I32(TABLE_SUCCESS)))
            }
            "hashmap_iter" => {
                let [Value::I32(out_ptr), Value::I32(map_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_iter expects (i32 out, i32 map)".into(),
                    });
                };
                let out_ptr = value_as_ptr_u32(&Value::I32(*out_ptr), "chic_rt.hashmap_iter out")?;
                let map_ptr = value_as_ptr_u32(&Value::I32(*map_ptr), "chic_rt.hashmap_iter map")?;
                if out_ptr == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                if map_ptr == 0 {
                    self.write_hashmap_iter_repr(out_ptr, WasmHashMapIterRepr::default())?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }
                let repr = self.read_hashmap_repr(map_ptr)?;
                self.write_hashmap_iter_repr(
                    out_ptr,
                    WasmHashMapIterRepr {
                        entries: repr.entries,
                        states: repr.states,
                        index: 0,
                        cap: repr.cap,
                        entry_size: repr.entry_size,
                        key_size: repr.key_size,
                        key_align: repr.key_align,
                        value_size: repr.value_size,
                        value_align: repr.value_align,
                        value_offset: repr.value_offset,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "hashmap_iter_next" => {
                let [
                    Value::I32(iter_ptr),
                    Value::I32(key_dest_ptr),
                    Value::I32(value_dest_ptr),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hashmap_iter_next expects (i32 iter, i32 key_dest, i32 value_dest)".into(),
                    });
                };
                let iter_ptr =
                    value_as_ptr_u32(&Value::I32(*iter_ptr), "chic_rt.hashmap_iter_next iter")?;
                let key_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*key_dest_ptr),
                    "chic_rt.hashmap_iter_next key_dest",
                )?;
                let value_dest_ptr = value_as_ptr_u32(
                    &Value::I32(*value_dest_ptr),
                    "chic_rt.hashmap_iter_next value_dest",
                )?;
                if iter_ptr == 0 {
                    return Ok(Some(Value::I32(TABLE_INVALID_POINTER)));
                }
                let mut iter = self.read_hashmap_iter_repr(iter_ptr)?;
                if iter.cap == 0 || iter.states == 0 {
                    return Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)));
                }
                while iter.index < iter.cap {
                    let idx = iter.index;
                    iter.index = idx.saturating_add(1);
                    let state = self.read_u8(iter.states.checked_add(idx).unwrap_or(0))?;
                    if state == TABLE_STATE_FULL {
                        let entry_ptr =
                            self.hashmap_entry_ptr(iter.entries, iter.entry_size, idx)?;
                        if key_dest_ptr != 0 {
                            let (out_data, _, _) = self.read_value_ptr(key_dest_ptr)?;
                            if out_data != 0 && iter.key_size != 0 {
                                let bytes = self.read_bytes(entry_ptr, iter.key_size)?;
                                self.store_bytes(out_data, 0, &bytes)?;
                            }
                        }
                        if value_dest_ptr != 0 {
                            let (out_data, _, _) = self.read_value_ptr(value_dest_ptr)?;
                            if out_data != 0 && iter.value_size != 0 {
                                let value_ptr =
                                    self.hashmap_value_ptr(entry_ptr, iter.value_offset)?;
                                let bytes = self.read_bytes(value_ptr, iter.value_size)?;
                                self.store_bytes(out_data, 0, &bytes)?;
                            }
                        }
                        self.write_hashmap_iter_repr(iter_ptr, iter)?;
                        return Ok(Some(Value::I32(TABLE_SUCCESS)));
                    }
                }
                self.write_hashmap_iter_repr(iter_ptr, iter)?;
                Ok(Some(Value::I32(TABLE_ITERATION_COMPLETE)))
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import chic_rt::{name} encountered during execution"
                ),
            }),
        }
    }
}
