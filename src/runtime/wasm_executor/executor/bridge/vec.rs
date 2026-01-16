use super::*;

impl<'a> Executor<'a> {
    fn drop_vec(&mut self, ptr: u32) -> Result<(), WasmExecutionError> {
        if ptr == 0 {
            return Ok(());
        }
        let repr = self.read_vec_repr(ptr)?;
        if repr.ptr == 0 || repr.len == 0 || repr.elem_size == 0 {
            self.write_vec_repr(ptr, WasmVecRepr::default())?;
            return Ok(());
        }

        if repr.drop_fn != 0 {
            let func_index = repr.drop_fn;
            let elem_size = repr.elem_size;
            for index in 0..repr.len {
                let offset = index
                    .checked_mul(elem_size)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_drop element offset overflow".into(),
                    })?;
                let elem_ptr = repr
                    .ptr
                    .checked_add(offset)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_drop element pointer overflow".into(),
                    })?;
                let _ = self.invoke(func_index, &[Value::I32(elem_ptr as i32)])?;
            }
        }

        self.write_vec_repr(ptr, WasmVecRepr::default())?;
        Ok(())
    }

    fn clone_vec(&mut self, dest_ptr: u32, src_ptr: u32) -> Result<i32, WasmExecutionError> {
        if dest_ptr == 0 || src_ptr == 0 {
            return Ok(1);
        }

        self.drop_vec(dest_ptr)?;

        let src = self.read_vec_repr(src_ptr)?;
        if src.ptr == 0 || src.len == 0 || src.elem_size == 0 {
            self.write_vec_repr(dest_ptr, WasmVecRepr::default())?;
            return Ok(0);
        }

        let bytes = src
            .len
            .checked_mul(src.elem_size)
            .ok_or_else(|| WasmExecutionError {
                message: "vec_clone length overflow".into(),
            })?;
        let base_align = src.elem_align.max(1);
        let data = self.read_bytes(src.ptr, bytes)?;
        let new_ptr = self.allocate_heap_block(bytes, base_align)?;
        self.store_bytes(new_ptr, 0, &data)?;
        self.write_vec_repr(
            dest_ptr,
            WasmVecRepr {
                ptr: new_ptr,
                len: src.len,
                cap: src.len,
                elem_size: src.elem_size,
                elem_align: base_align,
                drop_fn: src.drop_fn,
            },
        )?;
        Ok(0)
    }

    fn move_vec(&mut self, dest_ptr: u32, src_ptr: u32) -> Result<(), WasmExecutionError> {
        if dest_ptr == 0 || src_ptr == 0 {
            return Ok(());
        }
        self.drop_vec(dest_ptr)?;
        let src = self.read_vec_repr(src_ptr)?;
        self.write_vec_repr(
            dest_ptr,
            WasmVecRepr {
                cap: src.len,
                ..src
            },
        )?;
        self.write_vec_repr(src_ptr, WasmVecRepr::default())?;
        Ok(())
    }




}
