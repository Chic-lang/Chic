use super::*;

impl<'a> Executor<'a> {
    pub(super) fn table_round_up_pow2(&self, value: u32) -> Option<u32> {
        if value == 0 {
            return Some(0);
        }
        if value <= TABLE_MIN_CAPACITY {
            return Some(TABLE_MIN_CAPACITY);
        }
        let mut cap = TABLE_MIN_CAPACITY;
        while cap < value {
            cap = cap.checked_add(cap)?;
        }
        Some(cap)
    }

    pub(super) fn table_should_grow(
        &self,
        len: u32,
        tombstones: u32,
        cap: u32,
        additional: u32,
    ) -> bool {
        if cap == 0 {
            return true;
        }
        let filled = len as u64 + tombstones as u64;
        let needed = filled + additional as u64;
        needed.saturating_mul(TABLE_LOAD_DEN) > (cap as u64).saturating_mul(TABLE_LOAD_NUM)
    }

    pub(super) fn align_up(&self, value: u32, align: u32) -> Option<u32> {
        if align <= 1 {
            return Some(value);
        }
        let mask = align - 1;
        value.checked_add(mask).map(|v| v & !mask)
    }

    pub(super) fn hashset_entry_ptr(
        &self,
        entries: u32,
        elem_size: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entries == 0 {
            return Ok(0);
        }
        let offset = index
            .checked_mul(elem_size)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset entry offset overflow".into(),
            })?;
        entries
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset entry pointer overflow".into(),
            })
    }

    pub(super) fn hashset_hash_slot(
        &self,
        hashes: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if hashes == 0 {
            return Ok(0);
        }
        let offset = index.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashset hash offset overflow".into(),
        })?;
        hashes
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashset hash pointer overflow".into(),
            })
    }

    pub(super) fn hashset_drop_value(
        &mut self,
        repr: &WasmHashSetRepr,
        entry_ptr: u32,
    ) -> Result<(), WasmExecutionError> {
        if repr.drop_fn == 0 || entry_ptr == 0 {
            return Ok(());
        }
        if std::env::var_os("CHIC_DEBUG_WASM_HASHSET").is_some() {
            let idx = HASHSET_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
            if idx < 80 {
                let total = self.module.imports.len() + self.module.functions.len();
                eprintln!(
                    "[wasm-hashset] drop[{idx}] entry_ptr=0x{entry_ptr:08x} drop_fn={} total_funcs={}",
                    repr.drop_fn, total
                );
            }
        }
        let _ = self.invoke(repr.drop_fn, &[Value::I32(entry_ptr as i32)])?;
        Ok(())
    }

    pub(super) fn hashset_find_slot(
        &mut self,
        repr: &WasmHashSetRepr,
        hash: u64,
        key_ptr: u32,
    ) -> Result<(bool, u32), WasmExecutionError> {
        if repr.cap == 0 || repr.states == 0 {
            return Ok((false, 0));
        }
        let mask = repr.cap - 1;
        let start = (hash as u32) & mask;
        let mut first_tombstone = 0u32;
        let mut has_tombstone = false;
        let mut current = start;
        let mut probes = 0u32;
        while probes < repr.cap {
            let state = self.read_u8(repr.states.checked_add(current).unwrap_or(0))?;
            if state == TABLE_STATE_EMPTY {
                return Ok((
                    false,
                    if has_tombstone {
                        first_tombstone
                    } else {
                        current
                    },
                ));
            }
            if state == TABLE_STATE_TOMBSTONE {
                if !has_tombstone {
                    first_tombstone = current;
                    has_tombstone = true;
                }
            } else {
                let hash_slot = self.hashset_hash_slot(repr.hashes, current)?;
                let stored_hash = if hash_slot == 0 {
                    0
                } else {
                    self.read_u64(hash_slot)?
                };
                if stored_hash == hash && key_ptr != 0 && repr.entries != 0 && repr.eq_fn != 0 {
                    let entry_ptr =
                        self.hashset_entry_ptr(repr.entries, repr.elem_size, current)?;
                    let result = self.invoke(
                        repr.eq_fn,
                        &[Value::I32(entry_ptr as i32), Value::I32(key_ptr as i32)],
                    )?;
                    if matches!(result.first(), Some(Value::I32(v)) if *v != 0) {
                        return Ok((true, current));
                    }
                }
            }
            current = (current + 1) & mask;
            probes += 1;
        }
        Ok((false, 0))
    }

    pub(super) fn hashset_rehash(
        &mut self,
        repr: &WasmHashSetRepr,
        new_cap: u32,
    ) -> Result<WasmHashSetRepr, WasmExecutionError> {
        if new_cap == 0 {
            return Ok(WasmHashSetRepr {
                entries: 0,
                states: 0,
                hashes: 0,
                cap: 0,
                tombstones: 0,
                len: 0,
                ..*repr
            });
        }
        let elem_align = repr.elem_align.max(1);
        let entry_bytes =
            new_cap
                .checked_mul(repr.elem_size)
                .ok_or_else(|| WasmExecutionError {
                    message: "hashset rehash entry buffer overflow".into(),
                })?;
        let hash_bytes = new_cap.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashset rehash hash buffer overflow".into(),
        })?;
        let entries = self.allocate_heap_block(entry_bytes, elem_align)?;
        let states = self.allocate_heap_block(new_cap, 1)?;
        let hashes = self.allocate_heap_block(hash_bytes, 8)?;

        let mut rebuilt = WasmHashSetRepr {
            entries,
            states,
            hashes,
            cap: new_cap,
            tombstones: 0,
            len: 0,
            ..*repr
        };

        if repr.cap != 0 && repr.states != 0 {
            let mask = new_cap - 1;
            for idx in 0..repr.cap {
                let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                if state != TABLE_STATE_FULL {
                    continue;
                }
                let old_hash_slot = self.hashset_hash_slot(repr.hashes, idx)?;
                let hash_value = if old_hash_slot == 0 {
                    0
                } else {
                    self.read_u64(old_hash_slot)?
                };
                let mut insert_index = (hash_value as u32) & mask;
                while self.read_u8(states.checked_add(insert_index).unwrap_or(0))?
                    == TABLE_STATE_FULL
                {
                    insert_index = (insert_index + 1) & mask;
                }
                let src_ptr = self.hashset_entry_ptr(repr.entries, repr.elem_size, idx)?;
                let dest_ptr = self.hashset_entry_ptr(entries, repr.elem_size, insert_index)?;
                if repr.elem_size != 0 && src_ptr != 0 && dest_ptr != 0 {
                    let data = self.read_bytes(src_ptr, repr.elem_size)?;
                    self.store_bytes(dest_ptr, 0, &data)?;
                }
                let new_hash_slot = self.hashset_hash_slot(hashes, insert_index)?;
                if new_hash_slot != 0 {
                    self.write_u64(new_hash_slot, hash_value)?;
                }
                self.write_u8(
                    states.checked_add(insert_index).unwrap_or(0),
                    TABLE_STATE_FULL,
                )?;
                rebuilt.len = rebuilt.len.saturating_add(1);
            }
        }
        Ok(rebuilt)
    }

    pub(super) fn hashmap_entry_ptr(
        &self,
        entries: u32,
        entry_size: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entries == 0 {
            return Ok(0);
        }
        let offset = index
            .checked_mul(entry_size)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap entry offset overflow".into(),
            })?;
        entries
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap entry pointer overflow".into(),
            })
    }

    pub(super) fn hashmap_hash_slot(
        &self,
        hashes: u32,
        index: u32,
    ) -> Result<u32, WasmExecutionError> {
        if hashes == 0 {
            return Ok(0);
        }
        let offset = index.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashmap hash offset overflow".into(),
        })?;
        hashes
            .checked_add(offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap hash pointer overflow".into(),
            })
    }

    pub(super) fn hashmap_value_ptr(
        &self,
        entry_ptr: u32,
        value_offset: u32,
    ) -> Result<u32, WasmExecutionError> {
        if entry_ptr == 0 {
            return Ok(0);
        }
        entry_ptr
            .checked_add(value_offset)
            .ok_or_else(|| WasmExecutionError {
                message: "hashmap value pointer overflow".into(),
            })
    }

    pub(super) fn hashmap_drop_entry(
        &mut self,
        repr: &WasmHashMapRepr,
        entry_ptr: u32,
    ) -> Result<(), WasmExecutionError> {
        if entry_ptr == 0 {
            return Ok(());
        }
        if repr.key_drop_fn != 0 {
            let _ = self.invoke(repr.key_drop_fn, &[Value::I32(entry_ptr as i32)])?;
        }
        if repr.value_drop_fn != 0 {
            let value_ptr = self.hashmap_value_ptr(entry_ptr, repr.value_offset)?;
            let _ = self.invoke(repr.value_drop_fn, &[Value::I32(value_ptr as i32)])?;
        }
        Ok(())
    }

    pub(super) fn hashmap_find_slot(
        &mut self,
        repr: &WasmHashMapRepr,
        hash: u64,
        key_ptr: u32,
    ) -> Result<(bool, u32), WasmExecutionError> {
        if repr.cap == 0 || repr.states == 0 {
            return Ok((false, 0));
        }
        let mask = repr.cap - 1;
        let start = (hash as u32) & mask;
        let mut first_tombstone = 0u32;
        let mut has_tombstone = false;
        let mut current = start;
        let mut probes = 0u32;
        while probes < repr.cap {
            let state = self.read_u8(repr.states.checked_add(current).unwrap_or(0))?;
            if state == TABLE_STATE_EMPTY {
                return Ok((
                    false,
                    if has_tombstone {
                        first_tombstone
                    } else {
                        current
                    },
                ));
            }
            if state == TABLE_STATE_TOMBSTONE {
                if !has_tombstone {
                    first_tombstone = current;
                    has_tombstone = true;
                }
            } else {
                let hash_slot = self.hashmap_hash_slot(repr.hashes, current)?;
                let stored_hash = if hash_slot == 0 {
                    0
                } else {
                    self.read_u64(hash_slot)?
                };
                if stored_hash == hash && key_ptr != 0 && repr.entries != 0 && repr.key_eq_fn != 0 {
                    let entry_ptr =
                        self.hashmap_entry_ptr(repr.entries, repr.entry_size, current)?;
                    let result = self.invoke(
                        repr.key_eq_fn,
                        &[Value::I32(entry_ptr as i32), Value::I32(key_ptr as i32)],
                    )?;
                    if matches!(result.first(), Some(Value::I32(v)) if *v != 0) {
                        return Ok((true, current));
                    }
                }
            }
            current = (current + 1) & mask;
            probes += 1;
        }
        Ok((false, 0))
    }

    pub(super) fn hashmap_rehash(
        &mut self,
        repr: &WasmHashMapRepr,
        new_cap: u32,
    ) -> Result<WasmHashMapRepr, WasmExecutionError> {
        if new_cap == 0 {
            return Ok(WasmHashMapRepr {
                entries: 0,
                states: 0,
                hashes: 0,
                cap: 0,
                tombstones: 0,
                len: 0,
                ..*repr
            });
        }
        let entry_bytes =
            new_cap
                .checked_mul(repr.entry_size)
                .ok_or_else(|| WasmExecutionError {
                    message: "hashmap rehash entry buffer overflow".into(),
                })?;
        let hash_bytes = new_cap.checked_mul(8).ok_or_else(|| WasmExecutionError {
            message: "hashmap rehash hash buffer overflow".into(),
        })?;
        let max_align = repr.key_align.max(repr.value_align).max(1);
        let entries = self.allocate_heap_block(entry_bytes, max_align)?;
        let states = self.allocate_heap_block(new_cap, 1)?;
        let hashes = self.allocate_heap_block(hash_bytes, 8)?;

        let mut rebuilt = WasmHashMapRepr {
            entries,
            states,
            hashes,
            cap: new_cap,
            tombstones: 0,
            len: 0,
            ..*repr
        };
        if repr.cap != 0 && repr.states != 0 {
            let mask = new_cap - 1;
            for idx in 0..repr.cap {
                let state = self.read_u8(repr.states.checked_add(idx).unwrap_or(0))?;
                if state != TABLE_STATE_FULL {
                    continue;
                }
                let old_hash_slot = self.hashmap_hash_slot(repr.hashes, idx)?;
                let hash_value = if old_hash_slot == 0 {
                    0
                } else {
                    self.read_u64(old_hash_slot)?
                };
                let mut insert_index = (hash_value as u32) & mask;
                while self.read_u8(states.checked_add(insert_index).unwrap_or(0))?
                    == TABLE_STATE_FULL
                {
                    insert_index = (insert_index + 1) & mask;
                }
                let src_ptr = self.hashmap_entry_ptr(repr.entries, repr.entry_size, idx)?;
                let dest_ptr = self.hashmap_entry_ptr(entries, repr.entry_size, insert_index)?;
                if repr.entry_size != 0 && src_ptr != 0 && dest_ptr != 0 {
                    let data = self.read_bytes(src_ptr, repr.entry_size)?;
                    self.store_bytes(dest_ptr, 0, &data)?;
                }
                let new_hash_slot = self.hashmap_hash_slot(hashes, insert_index)?;
                if new_hash_slot != 0 {
                    self.write_u64(new_hash_slot, hash_value)?;
                }
                self.write_u8(
                    states.checked_add(insert_index).unwrap_or(0),
                    TABLE_STATE_FULL,
                )?;
                rebuilt.len = rebuilt.len.saturating_add(1);
            }
        }
        Ok(rebuilt)
    }
}
