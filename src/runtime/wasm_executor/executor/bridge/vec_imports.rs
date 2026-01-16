use super::*;

impl<'a> Executor<'a> {
    fn invoke_vec_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "vec_drop" => {
                let [Value::I32(ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_drop expects a single i32 argument".into(),
                    });
                };
                let ptr = value_as_ptr_u32(&Value::I32(*ptr), "chic_rt.vec_drop ptr")?;
                self.drop_vec(ptr)?;
                Ok(None)
            }
            "vec_with_capacity" => {
                let [
                    Value::I32(out_ptr),
                    Value::I32(elem_size),
                    Value::I32(elem_align),
                    Value::I32(capacity),
                    Value::I32(drop_fn),
                ] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_with_capacity expects (i32 out, i32 elem_size, i32 elem_align, i32 capacity, i32 drop_fn) arguments".into(),
                    });
                };
                let out_ptr = u32::try_from(*out_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative return pointer".into(),
                })?;
                let elem_size = u32::try_from(*elem_size).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element size".into(),
                })?;
                let elem_align = u32::try_from(*elem_align).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative element alignment".into(),
                })?;
                let capacity = u32::try_from(*capacity).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative capacity".into(),
                })?;
                let drop_fn = u32::try_from(*drop_fn).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_with_capacity received negative drop function index"
                        .into(),
                })?;

                let base_align = elem_align.max(1);
                if capacity == 0 || elem_size == 0 {
                    self.write_vec_repr(
                        out_ptr,
                        WasmVecRepr {
                            ptr: 0,
                            len: 0,
                            cap: 0,
                            elem_size,
                            elem_align: base_align,
                            drop_fn,
                        },
                    )?;
                    return Ok(Some(Value::I32(out_ptr as i32)));
                }

                let bytes = capacity
                    .checked_mul(elem_size)
                    .ok_or_else(|| WasmExecutionError {
                        message: "vec_with_capacity length overflow".into(),
                    })?;
                let ptr = self.allocate_heap_block(bytes, base_align)?;
                self.write_vec_repr(
                    out_ptr,
                    WasmVecRepr {
                        ptr,
                        len: 0,
                        cap: capacity,
                        elem_size,
                        elem_align: base_align,
                        drop_fn,
                    },
                )?;
                Ok(Some(Value::I32(out_ptr as i32)))
            }
            "vec_clone" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_clone expects (i32, i32) arguments".into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_clone received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            "vec_into_array" | "array_into_vec" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_into_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative destination pointer".into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_into_array received negative source pointer".into(),
                })?;
                self.move_vec(dest, src)?;
                Ok(Some(Value::I32(0)))
            }
            "vec_copy_to_array" | "array_copy_to_vec" => {
                let [Value::I32(dest_ptr), Value::I32(src_ptr)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.vec_copy_to_array expects (i32 dest, i32 src) arguments"
                            .into(),
                    });
                };
                let dest = u32::try_from(*dest_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative destination pointer"
                        .into(),
                })?;
                let src = u32::try_from(*src_ptr).map_err(|_| WasmExecutionError {
                    message: "chic_rt.vec_copy_to_array received negative source pointer".into(),
                })?;
                let status = self.clone_vec(dest, src)?;
                Ok(Some(Value::I32(status)))
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import chic_rt::{name} encountered during execution"
                ),
            }),
        }
    }
}
