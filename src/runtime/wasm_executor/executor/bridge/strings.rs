use super::*;

impl<'a> Executor<'a> {
    pub(super) fn inline_string_ptr(&self, base: u32) -> Result<u32, WasmExecutionError> {
        base.checked_add(self.ptr_stride() * 3)
            .ok_or_else(|| WasmExecutionError {
                message: "string inline pointer overflow".into(),
            })
    }

    pub(super) fn is_inline_string(&self, repr: &WasmStringRepr) -> bool {
        if repr.len == 0 || repr.len > STRING_INLINE_CAPACITY {
            return false;
        }
        if (repr.cap & STRING_INLINE_TAG) != 0 {
            return true;
        }
        repr.ptr == 0
    }

    pub(super) fn init_empty_string(&mut self, dest_ptr: u32) -> Result<(), WasmExecutionError> {
        self.write_string_repr(dest_ptr, WasmStringRepr::default())
    }

    pub(super) fn resolve_string_data_ptr(
        &self,
        base: u32,
        repr: &WasmStringRepr,
    ) -> Result<u32, WasmExecutionError> {
        if repr.len == 0 {
            return Ok(STRING_EMPTY_PTR);
        }
        if self.is_inline_string(repr) {
            return self.inline_string_ptr(base);
        }
        if repr.ptr == 0 {
            return Err(WasmExecutionError {
                message: format!(
                    "string at 0x{base:08X} has len {} cap=0x{:08X} but null pointer",
                    repr.len, repr.cap
                ),
            });
        }
        Ok(repr.ptr)
    }

    pub(super) fn store_string_bytes(
        &mut self,
        dest_ptr: u32,
        data: &[u8],
    ) -> Result<i32, WasmExecutionError> {
        let len = u32::try_from(data.len()).map_err(|_| WasmExecutionError {
            message: "string data exceeds addressable range for wasm32".into(),
        })?;
        if len == 0 {
            self.init_empty_string(dest_ptr)?;
            return Ok(0);
        }
        if len <= STRING_INLINE_CAPACITY {
            let inline_ptr = self.inline_string_ptr(dest_ptr)?;
            self.store_bytes(inline_ptr, 0, data)?;
            self.write_string_repr(
                dest_ptr,
                WasmStringRepr {
                    ptr: 0,
                    len,
                    cap: STRING_INLINE_TAG | STRING_INLINE_CAPACITY,
                },
            )?;
            return Ok(0);
        }
        let ptr = self.allocate_heap_block(len, 1)?;
        self.store_bytes(ptr, 0, data)?;
        self.write_string_repr(dest_ptr, WasmStringRepr { ptr, len, cap: len })?;
        Ok(0)
    }

    pub(super) fn append_string_bytes(
        &mut self,
        target_ptr: u32,
        data: &[u8],
    ) -> Result<i32, WasmExecutionError> {
        if target_ptr == 0 {
            return Ok(STRING_INVALID_POINTER);
        }
        if data.is_empty() {
            return Ok(STRING_SUCCESS);
        }
        let mut repr = self.read_string_repr(target_ptr)?;
        let inline = self.is_inline_string(&repr);
        if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
            eprintln!(
                "[wasm-string] append_bytes target=0x{target_ptr:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline} data_len={} mem_len={}",
                data.len(),
                self.memory_len(),
                ptr = repr.ptr,
                len = repr.len,
                cap = repr.cap
            );
        }
        if repr.len > 0 && repr.ptr == 0 && !inline {
            if std::env::var_os("CHIC_DEBUG_WASM_STRING").is_some() {
                eprintln!(
                    "[wasm-string] append_bytes invalid target=0x{target_ptr:08X} repr={{ptr=0x{ptr:08X} len={len} cap=0x{cap:08X}}} inline={inline}",
                    ptr = repr.ptr,
                    len = repr.len,
                    cap = repr.cap
                );
            }
            return Ok(STRING_INVALID_POINTER);
        }
        let data_len = u32::try_from(data.len()).map_err(|_| WasmExecutionError {
            message: "string append length exceeds wasm32 range".into(),
        })?;
        let new_len = match repr.len.checked_add(data_len) {
            Some(len) => len,
            None => return Ok(STRING_CAPACITY_OVERFLOW),
        };
        let capacity = if inline {
            STRING_INLINE_CAPACITY
        } else {
            repr.cap & !STRING_INLINE_TAG
        };

        if inline && new_len <= STRING_INLINE_CAPACITY {
            let inline_ptr = self.inline_string_ptr(target_ptr)?;
            self.store_bytes(inline_ptr, repr.len, data)?;
            repr.len = new_len;
            self.write_string_repr(target_ptr, repr)?;
            return Ok(STRING_SUCCESS);
        }

        let needs_alloc = new_len > capacity || repr.ptr == 0;
        if needs_alloc {
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
            self.store_bytes(new_ptr, repr.len, data)?;
            repr.ptr = new_ptr;
            repr.cap = new_len;
        } else {
            self.store_bytes(repr.ptr, repr.len, data)?;
        }

        repr.len = new_len;
        self.write_string_repr(target_ptr, repr)?;
        Ok(STRING_SUCCESS)
    }
}
