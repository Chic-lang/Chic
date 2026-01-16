use super::*;

impl<'a> Executor<'a> {
    fn invoke_runtime_helpers_import(
        &mut self,
        name: &str,
        params: &[Value],
    ) -> Result<Option<Value>, WasmExecutionError> {
        match name {
            "borrow_shared" => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_shared expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_shared received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Shared)?;
                Ok(None)
            }
            "borrow_unique" => {
                let [Value::I32(borrow_id), Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_unique expects (i32, i32) arguments".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.borrow_unique received negative address".into(),
                })?;
                self.register_borrow(*borrow_id, address, BorrowRuntimeKind::Unique)?;
                Ok(None)
            }
            "borrow_release" => {
                let [Value::I32(borrow_id)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.borrow_release expects a single i32 argument".into(),
                    });
                };
                self.release_borrow(*borrow_id)?;
                Ok(None)
            }
            "drop_invoke" => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(None);
                }
                let func_index = value_as_u32(&Value::I32(*func), "chic_rt.drop_invoke func")?;
                let value_ptr = value_as_ptr_u32(&Value::I32(*value), "chic_rt.drop_invoke value")?;
                if std::env::var_os("CHIC_DEBUG_WASM_DROP_INVOKE").is_some() {
                    let total = self.module.imports.len() + self.module.functions.len();
                    let mut current_exports = Vec::new();
                    if let Some(current) = self.current_function {
                        for (name, &idx) in &self.module.exports {
                            if idx == current {
                                current_exports.push(name.as_str());
                            }
                        }
                    }
                    eprintln!(
                        "[wasm-drop-invoke] func_index={} value_ptr=0x{value_ptr:08x} total_funcs={} current={:?} current_exports={:?} call_stack={:?}",
                        func_index, total, self.current_function, current_exports, self.call_stack
                    );
                }
                let _ = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                Ok(None)
            }
            "hash_invoke" => {
                let [Value::I32(func), Value::I32(value)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.hash_invoke expects (i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *value == 0 {
                    return Ok(Some(Value::I64(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative function pointer".into(),
                })?;
                let value_ptr = u32::try_from(*value).map_err(|_| WasmExecutionError {
                    message: "chic_rt.hash_invoke received negative value pointer".into(),
                })?;
                let result = self.invoke(func_index, &[Value::I32(value_ptr as i32)])?;
                let value = result.first().copied();
                match value {
                    Some(Value::I64(hash)) => Ok(Some(Value::I64(hash))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!(
                            "chic_rt.hash_invoke expected i64 result, found {other:?}"
                        ),
                    }),
                    None => Ok(Some(Value::I64(0))),
                }
            }
            "eq_invoke" => {
                let [Value::I32(func), Value::I32(left), Value::I32(right)] = params.as_slice()
                else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.eq_invoke expects (i32, i32, i32) arguments".into(),
                    });
                };
                if *func == 0 || *left == 0 || *right == 0 {
                    return Ok(Some(Value::I32(0)));
                }
                let func_index = u32::try_from(*func).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative function pointer".into(),
                })?;
                let left_ptr = u32::try_from(*left).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative left pointer".into(),
                })?;
                let right_ptr = u32::try_from(*right).map_err(|_| WasmExecutionError {
                    message: "chic_rt.eq_invoke received negative right pointer".into(),
                })?;
                let result = self.invoke(
                    func_index,
                    &[Value::I32(left_ptr as i32), Value::I32(right_ptr as i32)],
                )?;
                let value = result.first().copied();
                match value {
                    Some(Value::I32(value)) => Ok(Some(Value::I32(value))),
                    Some(other) => Err(WasmExecutionError {
                        message: format!("chic_rt.eq_invoke expected i32 result, found {other:?}"),
                    }),
                    None => Ok(Some(Value::I32(0))),
                }
            }
            "drop_resource" => {
                let [Value::I32(address)] = params.as_slice() else {
                    return Err(WasmExecutionError {
                        message: "chic_rt.drop_resource expects a single i32 argument".into(),
                    });
                };
                let address = u32::try_from(*address).map_err(|_| WasmExecutionError {
                    message: "chic_rt.drop_resource received negative address".into(),
                })?;
                self.drop_resource(address)?;
                Ok(None)
            }
            _ => Err(WasmExecutionError {
                message: format!(
                    "unsupported import chic_rt::{name} encountered during execution"
                ),
            }),
        }
    }
}
