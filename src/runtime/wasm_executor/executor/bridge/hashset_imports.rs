use super::*;

impl<'a> Executor<'a> {
    pub(super) fn invoke_hashset_import(
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
                ] = params
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
                ] = params
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
                let [Value::I32(set_ptr)] = params else {
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
                let [Value::I32(set_ptr)] = params else {
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
                let [Value::I32(set_ptr)] = params else {
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
                let [Value::I32(set_ptr)] = params else {
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
                let [Value::I32(set_ptr)] = params else {
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
                let [Value::I32(set_ptr), Value::I32(additional)] = params else {
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
                let [Value::I32(set_ptr), Value::I32(min_capacity)] = params else {
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
                ] = params
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
                ] = params
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
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] = params else {
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
                ] = params
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
                ] = params
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
                let [Value::I32(set_ptr), Value::I64(hash), Value::I32(key_ptr)] = params else {
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
                let [Value::I32(set_ptr), Value::I32(index), Value::I32(dest_ptr)] = params else {
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
                let [Value::I32(set_ptr), Value::I32(index)] = params else {
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
                let [Value::I32(set_ptr), Value::I32(index)] = params else {
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
                let [Value::I32(out_ptr), Value::I32(set_ptr)] = params else {
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
                let [Value::I32(iter_ptr), Value::I32(dest_ptr)] = params else {
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
                let [Value::I32(out_ptr), Value::I32(iter_ptr)] = params else {
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
                message: format!("unsupported import chic_rt::{name} encountered during execution"),
            }),
        }
    }
}
