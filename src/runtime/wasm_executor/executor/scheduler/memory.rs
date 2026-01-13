use crate::runtime::wasm_executor::errors::WasmExecutionError;

use super::super::runtime::{
    WasmHashMapIterRepr, WasmHashMapRepr, WasmHashSetIterRepr, WasmHashSetRepr, WasmStringRepr,
    WasmVecRepr,
};
use super::{Executor, LINEAR_MEMORY_HEAP_BASE};

impl<'a> Executor<'a> {
    pub(crate) fn read_u8(&self, address: u32) -> Result<u8, WasmExecutionError> {
        let addr = address as usize;
        if addr >= self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory read of 1 byte at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        Ok(self.memory[addr])
    }

    pub(crate) fn write_u8(&mut self, address: u32, value: u8) -> Result<(), WasmExecutionError> {
        let addr = address as usize;
        if addr >= self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory write of 1 byte at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        self.memory[addr] = value;
        Ok(())
    }

    pub(crate) fn read_u32(&self, address: u32) -> Result<u32, WasmExecutionError> {
        let addr = address as usize;
        if addr + 4 > self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory read of 4 bytes at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        let bytes: [u8; 4] = self.memory[addr..addr + 4]
            .try_into()
            .expect("slice length verified");
        Ok(u32::from_le_bytes(bytes))
    }

    pub(crate) fn write_u32(&mut self, address: u32, value: u32) -> Result<(), WasmExecutionError> {
        let addr = address as usize;
        if addr + 4 > self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory write of 4 bytes at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        self.memory[addr..addr + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub(crate) fn read_u64(&self, address: u32) -> Result<u64, WasmExecutionError> {
        let addr = address as usize;
        if addr + 8 > self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory read of 8 bytes at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        let bytes: [u8; 8] = self.memory[addr..addr + 8]
            .try_into()
            .expect("slice length verified");
        Ok(u64::from_le_bytes(bytes))
    }

    pub(crate) fn write_u64(&mut self, address: u32, value: u64) -> Result<(), WasmExecutionError> {
        let addr = address as usize;
        if addr + 8 > self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!(
                    "memory write of 8 bytes at 0x{address:08X} exceeds linear memory bounds (ctx={ctx})"
                ),
            });
        }
        self.memory[addr..addr + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub(crate) fn read_bytes(&self, address: u32, len: u32) -> Result<Vec<u8>, WasmExecutionError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let addr = address as usize;
        let length = len as usize;
        if addr.checked_add(length).unwrap_or(usize::MAX) > self.memory.len() {
            let import = self
                .current_import
                .as_ref()
                .map(|(module, name)| format!("{module}.{name}"));
            let ctx = self.current_wasm_context();
            let stack = self.format_call_stack();
            return Err(WasmExecutionError {
                message: format!(
                    "slice of length {len} at 0x{address:08X} exceeds linear memory bounds (ctx={ctx} stack={stack} import={import:?})"
                ),
            });
        }
        Ok(self.memory[addr..addr + length].to_vec())
    }

    pub(crate) fn read_string_repr(
        &self,
        address: u32,
    ) -> Result<WasmStringRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmStringRepr {
            ptr: self.read_u32(address)?,
            len: self.read_u32(address + stride)?,
            cap: self.read_u32(address + stride * 2)?,
        })
    }

    pub(crate) fn write_string_repr(
        &mut self,
        address: u32,
        repr: WasmStringRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.ptr)?;
        self.write_u32(address + stride, repr.len)?;
        self.write_u32(address + stride * 2, repr.cap)
    }

    pub(crate) fn read_vec_repr(&self, address: u32) -> Result<WasmVecRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmVecRepr {
            ptr: self.read_u32(address)?,
            len: self.read_u32(address + stride)?,
            cap: self.read_u32(address + stride * 2)?,
            elem_size: self.read_u32(address + stride * 3)?,
            elem_align: self.read_u32(address + stride * 4)?,
            drop_fn: self.read_u32(address + stride * 5)?,
        })
    }

    pub(crate) fn write_vec_repr(
        &mut self,
        address: u32,
        repr: WasmVecRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.ptr)?;
        self.write_u32(address + stride, repr.len)?;
        self.write_u32(address + stride * 2, repr.cap)?;
        self.write_u32(address + stride * 3, repr.elem_size)?;
        self.write_u32(address + stride * 4, repr.elem_align)?;
        self.write_u32(address + stride * 5, repr.drop_fn)
    }

    pub(crate) fn read_hashset_repr(
        &self,
        address: u32,
    ) -> Result<WasmHashSetRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmHashSetRepr {
            entries: self.read_u32(address)?,
            states: self.read_u32(address + stride)?,
            hashes: self.read_u32(address + stride * 2)?,
            len: self.read_u32(address + stride * 3)?,
            cap: self.read_u32(address + stride * 4)?,
            tombstones: self.read_u32(address + stride * 5)?,
            elem_size: self.read_u32(address + stride * 6)?,
            elem_align: self.read_u32(address + stride * 7)?,
            drop_fn: self.read_u32(address + stride * 8)?,
            eq_fn: self.read_u32(address + stride * 9)?,
        })
    }

    pub(crate) fn write_hashset_repr(
        &mut self,
        address: u32,
        repr: WasmHashSetRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.entries)?;
        self.write_u32(address + stride, repr.states)?;
        self.write_u32(address + stride * 2, repr.hashes)?;
        self.write_u32(address + stride * 3, repr.len)?;
        self.write_u32(address + stride * 4, repr.cap)?;
        self.write_u32(address + stride * 5, repr.tombstones)?;
        self.write_u32(address + stride * 6, repr.elem_size)?;
        self.write_u32(address + stride * 7, repr.elem_align)?;
        self.write_u32(address + stride * 8, repr.drop_fn)?;
        self.write_u32(address + stride * 9, repr.eq_fn)
    }

    pub(crate) fn read_hashset_iter_repr(
        &self,
        address: u32,
    ) -> Result<WasmHashSetIterRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmHashSetIterRepr {
            entries: self.read_u32(address)?,
            states: self.read_u32(address + stride)?,
            index: self.read_u32(address + stride * 2)?,
            cap: self.read_u32(address + stride * 3)?,
            elem_size: self.read_u32(address + stride * 4)?,
            elem_align: self.read_u32(address + stride * 5)?,
        })
    }

    pub(crate) fn write_hashset_iter_repr(
        &mut self,
        address: u32,
        repr: WasmHashSetIterRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.entries)?;
        self.write_u32(address + stride, repr.states)?;
        self.write_u32(address + stride * 2, repr.index)?;
        self.write_u32(address + stride * 3, repr.cap)?;
        self.write_u32(address + stride * 4, repr.elem_size)?;
        self.write_u32(address + stride * 5, repr.elem_align)
    }

    pub(crate) fn read_hashmap_repr(
        &self,
        address: u32,
    ) -> Result<WasmHashMapRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmHashMapRepr {
            entries: self.read_u32(address)?,
            states: self.read_u32(address + stride)?,
            hashes: self.read_u32(address + stride * 2)?,
            len: self.read_u32(address + stride * 3)?,
            cap: self.read_u32(address + stride * 4)?,
            tombstones: self.read_u32(address + stride * 5)?,
            key_size: self.read_u32(address + stride * 6)?,
            key_align: self.read_u32(address + stride * 7)?,
            value_size: self.read_u32(address + stride * 8)?,
            value_align: self.read_u32(address + stride * 9)?,
            entry_size: self.read_u32(address + stride * 10)?,
            value_offset: self.read_u32(address + stride * 11)?,
            key_drop_fn: self.read_u32(address + stride * 12)?,
            value_drop_fn: self.read_u32(address + stride * 13)?,
            key_eq_fn: self.read_u32(address + stride * 14)?,
        })
    }

    pub(crate) fn write_hashmap_repr(
        &mut self,
        address: u32,
        repr: WasmHashMapRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.entries)?;
        self.write_u32(address + stride, repr.states)?;
        self.write_u32(address + stride * 2, repr.hashes)?;
        self.write_u32(address + stride * 3, repr.len)?;
        self.write_u32(address + stride * 4, repr.cap)?;
        self.write_u32(address + stride * 5, repr.tombstones)?;
        self.write_u32(address + stride * 6, repr.key_size)?;
        self.write_u32(address + stride * 7, repr.key_align)?;
        self.write_u32(address + stride * 8, repr.value_size)?;
        self.write_u32(address + stride * 9, repr.value_align)?;
        self.write_u32(address + stride * 10, repr.entry_size)?;
        self.write_u32(address + stride * 11, repr.value_offset)?;
        self.write_u32(address + stride * 12, repr.key_drop_fn)?;
        self.write_u32(address + stride * 13, repr.value_drop_fn)?;
        self.write_u32(address + stride * 14, repr.key_eq_fn)
    }

    pub(crate) fn read_hashmap_iter_repr(
        &self,
        address: u32,
    ) -> Result<WasmHashMapIterRepr, WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        Ok(WasmHashMapIterRepr {
            entries: self.read_u32(address)?,
            states: self.read_u32(address + stride)?,
            index: self.read_u32(address + stride * 2)?,
            cap: self.read_u32(address + stride * 3)?,
            entry_size: self.read_u32(address + stride * 4)?,
            key_size: self.read_u32(address + stride * 5)?,
            key_align: self.read_u32(address + stride * 6)?,
            value_size: self.read_u32(address + stride * 7)?,
            value_align: self.read_u32(address + stride * 8)?,
            value_offset: self.read_u32(address + stride * 9)?,
        })
    }

    pub(crate) fn write_hashmap_iter_repr(
        &mut self,
        address: u32,
        repr: WasmHashMapIterRepr,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.async_layout.ptr_size;
        self.write_u32(address, repr.entries)?;
        self.write_u32(address + stride, repr.states)?;
        self.write_u32(address + stride * 2, repr.index)?;
        self.write_u32(address + stride * 3, repr.cap)?;
        self.write_u32(address + stride * 4, repr.entry_size)?;
        self.write_u32(address + stride * 5, repr.key_size)?;
        self.write_u32(address + stride * 6, repr.key_align)?;
        self.write_u32(address + stride * 7, repr.value_size)?;
        self.write_u32(address + stride * 8, repr.value_align)?;
        self.write_u32(address + stride * 9, repr.value_offset)
    }

    pub(crate) fn load_bytes(
        &self,
        addr: u32,
        offset: u32,
        width: usize,
    ) -> Result<&[u8], WasmExecutionError> {
        let base = addr.checked_add(offset).ok_or_else(|| WasmExecutionError {
            message: "memory address overflow".into(),
        })? as usize;
        let end = base.checked_add(width).ok_or_else(|| WasmExecutionError {
            message: "memory address overflow".into(),
        })?;
        if end > self.memory.len() {
            let ctx = self.current_wasm_context();
            let stack = self.format_call_stack();
            return Err(WasmExecutionError {
                message: format!(
                    "memory access out of bounds (load base=0x{base:08x} end=0x{end:08x} width={width} ctx={ctx} stack={stack})"
                ),
            });
        }
        Ok(&self.memory[base..end])
    }

    pub(crate) fn load_i32(&self, addr: u32, offset: u32) -> Result<i32, WasmExecutionError> {
        let bytes = self.load_bytes(addr, offset, 4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(bytes);
        Ok(i32::from_le_bytes(buf))
    }

    pub(crate) fn load_i64(&self, addr: u32, offset: u32) -> Result<i64, WasmExecutionError> {
        let bytes = self.load_bytes(addr, offset, 8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        Ok(i64::from_le_bytes(buf))
    }

    pub(crate) fn load_f32(&self, addr: u32, offset: u32) -> Result<f32, WasmExecutionError> {
        let bytes = self.load_bytes(addr, offset, 4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(bytes);
        Ok(f32::from_le_bytes(buf))
    }

    pub(crate) fn load_f64(&self, addr: u32, offset: u32) -> Result<f64, WasmExecutionError> {
        let bytes = self.load_bytes(addr, offset, 8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        Ok(f64::from_le_bytes(buf))
    }

    pub(crate) fn store_bytes(
        &mut self,
        addr: u32,
        offset: u32,
        data: &[u8],
    ) -> Result<(), WasmExecutionError> {
        let base = addr.checked_add(offset).ok_or_else(|| WasmExecutionError {
            message: "memory address overflow".into(),
        })? as usize;
        let end = base
            .checked_add(data.len())
            .ok_or_else(|| WasmExecutionError {
                message: "memory address overflow".into(),
            })?;
        if end > self.memory.len() {
            let import = self
                .current_import
                .as_ref()
                .map(|(module, name)| format!("{module}.{name}"));
            let ctx = self.current_wasm_context();
            let stack = self.format_call_stack();
            return Err(WasmExecutionError {
                message: format!(
                    "memory access out of bounds (store base=0x{base:08x} end=0x{end:08x} len={} mem_len={} ctx={ctx} stack={stack} import={import:?})",
                    data.len(),
                    self.memory.len(),
                ),
            });
        }
        self.memory[base..end].copy_from_slice(data);
        Ok(())
    }

    pub(crate) fn store_i32(
        &mut self,
        addr: u32,
        offset: u32,
        value: i32,
    ) -> Result<(), WasmExecutionError> {
        self.store_bytes(addr, offset, &value.to_le_bytes())
    }

    pub(crate) fn store_i8(
        &mut self,
        addr: u32,
        offset: u32,
        value: u8,
    ) -> Result<(), WasmExecutionError> {
        self.store_bytes(addr, offset, &[value])
    }

    pub(crate) fn store_i64(
        &mut self,
        addr: u32,
        offset: u32,
        value: i64,
    ) -> Result<(), WasmExecutionError> {
        self.store_bytes(addr, offset, &value.to_le_bytes())
    }

    pub(crate) fn store_f32(
        &mut self,
        addr: u32,
        offset: u32,
        value: f32,
    ) -> Result<(), WasmExecutionError> {
        self.store_bytes(addr, offset, &value.to_le_bytes())
    }

    pub(crate) fn store_f64(
        &mut self,
        addr: u32,
        offset: u32,
        value: f64,
    ) -> Result<(), WasmExecutionError> {
        self.store_bytes(addr, offset, &value.to_le_bytes())
    }

    pub(crate) fn fill(
        &mut self,
        addr: u32,
        offset: u32,
        len: u32,
        value: u8,
    ) -> Result<(), WasmExecutionError> {
        let base = addr.checked_add(offset).ok_or_else(|| WasmExecutionError {
            message: "memory address overflow".into(),
        })? as usize;
        let end = base
            .checked_add(len as usize)
            .ok_or_else(|| WasmExecutionError {
                message: "memory address overflow".into(),
            })?;
        if end > self.memory.len() {
            let ctx = self.current_wasm_context();
            return Err(WasmExecutionError {
                message: format!("memory access out of bounds (ctx={ctx})"),
            });
        }
        self.memory[base..end].fill(value);
        Ok(())
    }

    pub(crate) fn allocate_heap_block(
        &mut self,
        size: u32,
        align: u32,
    ) -> Result<u32, WasmExecutionError> {
        if size == 0 {
            return Ok(0);
        }
        let align = align.max(1);
        let base = self.heap_cursor.max(LINEAR_MEMORY_HEAP_BASE);
        let start = align_address(base, align)?;
        let end = start.checked_add(size).ok_or_else(|| WasmExecutionError {
            message: "linear memory allocation overflowed address space".into(),
        })?;
        let start_index = start as usize;
        let end_index = end as usize;
        self.ensure_linear_memory(end_index, size as usize)?;
        self.memory[start_index..end_index].fill(0);
        self.heap_cursor = end;
        Ok(start)
    }

    fn ensure_linear_memory(
        &mut self,
        required_end: usize,
        allocation_size: usize,
    ) -> Result<(), WasmExecutionError> {
        if let Some(limit_pages) = self.options.memory_limit_pages {
            let limit_bytes = limit_pages as usize * super::WASM_PAGE_SIZE;
            if required_end > limit_bytes {
                return Err(WasmExecutionError {
                    message: format!(
                        "linear memory allocation of {allocation_size} byte(s) exceeds executor limit of {limit_bytes} byte(s)"
                    ),
                });
            }
        }
        if required_end > self.memory.len() {
            self.memory.resize(required_end, 0);
        }
        Ok(())
    }
}

fn align_address(value: u32, align: u32) -> Result<u32, WasmExecutionError> {
    if align <= 1 {
        return Ok(value);
    }
    let remainder = value % align;
    if remainder == 0 {
        return Ok(value);
    }
    value
        .checked_add(align - remainder)
        .ok_or_else(|| WasmExecutionError {
            message: "linear memory address overflowed while aligning allocation".into(),
        })
}
