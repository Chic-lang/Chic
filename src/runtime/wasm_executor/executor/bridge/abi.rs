use super::*;

impl<'a> Executor<'a> {
    fn read_i128(&self, ptr: u32) -> Result<i128, WasmExecutionError> {
        let lo = self.read_u64(ptr)?;
        let hi = self.read_u64(ptr + 8)?;
        let hi = i64::from_le_bytes(hi.to_le_bytes());
        Ok(((hi as i128) << 64) | lo as i128)
    }

    fn write_i128(&mut self, ptr: u32, value: i128) -> Result<(), WasmExecutionError> {
        let lo = value as u128 as u64;
        let hi = (value >> 64) as i64;
        self.write_u64(ptr, lo)?;
        self.write_u64(ptr + 8, hi as u64)?;
        Ok(())
    }

    fn read_str_ptr(&self, ptr: u32) -> Result<(u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let data_ptr = self.read_u32(ptr)?;
        let len = self.read_u32(ptr + stride)?;
        Ok((data_ptr, len))
    }

    fn write_str_ptr(&mut self, ptr: u32, data_ptr: u32, len: u32) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        self.write_u32(ptr, data_ptr)?;
        self.write_u32(ptr + stride, len)
    }

    fn read_value_ptr(&self, ptr: u32) -> Result<(u32, u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let data_ptr = self.read_u32(ptr)?;
        let size = self.read_u32(ptr + stride)?;
        let align = self.read_u32(ptr + stride * 2)?;
        Ok((data_ptr, size, align))
    }

    fn write_value_ptr(
        &mut self,
        ptr: u32,
        data_ptr: u32,
        size: u32,
        align: u32,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        self.write_u32(ptr, data_ptr)?;
        self.write_u32(ptr + stride, size)?;
        self.write_u32(ptr + stride * 2, align)
    }

    fn read_span_ptr(&self, ptr: u32) -> Result<(u32, u32, u32, u32), WasmExecutionError> {
        let stride = self.ptr_stride();
        let (data_ptr, elem_size, elem_align) = self.read_value_ptr(ptr)?;
        // WASM SpanPtr ABI matches `Std.Runtime.Collections.{SpanPtr,ReadOnlySpanPtr}`:
        //   ValuePtr data { ptr, size, align } (3 * stride)
        //   usize length
        //   usize elementSize
        //   usize elementAlignment
        let length_offset = stride * 3;
        let len = self.read_u32(ptr + length_offset)?;
        Ok((data_ptr, len, elem_size, elem_align))
    }

    fn write_span_ptr(
        &mut self,
        ptr: u32,
        data_ptr: u32,
        len: u32,
        elem_size: u32,
        elem_align: u32,
    ) -> Result<(), WasmExecutionError> {
        let stride = self.ptr_stride();
        let length_offset = stride * 3;
        self.write_value_ptr(ptr, data_ptr, elem_size, elem_align)?;
        self.write_u32(ptr + length_offset, len)?;
        self.write_u32(ptr + length_offset + stride, elem_size)?;
        self.write_u32(ptr + length_offset + stride * 2, elem_align)
    }

    fn ptr_stride(&self) -> u32 {
        self.async_layout.ptr_size
    }
}
