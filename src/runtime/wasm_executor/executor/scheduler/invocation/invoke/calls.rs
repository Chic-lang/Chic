use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_calls(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let func_index = ctx.func_index;
        let stack = &mut *ctx.stack;
        let tracer = &mut *ctx.tracer;
        match &code[pc] {
            Instruction::Call { func } => {
                let import_count = self.module.imports.len();
                if (*func as usize) < import_count {
                    let import = &self.module.imports[*func as usize];
                    if std::env::var("CHIC_DEBUG_WASM_IMPORTS").is_ok() {
                        eprintln!(
                            "[wasm-import] func_index={} import={}::{}",
                            func, import.module, import.name
                        );
                    }
                    let sig = self
                        .module
                        .types
                        .get(import.type_index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "import {}/{} references invalid type index",
                                import.module, import.name
                            ),
                        })?
                        .clone();
                    let mut params = Vec::with_capacity(sig.params.len());
                    for expected in sig.params.iter().rev() {
                        let value = stack.pop().ok_or_else(|| WasmExecutionError {
                                message: format!(
                                    "value stack underflow during call to import {}::{} (caller={func_index} pc={pc} expected={expected:?})",
                                    import.module, import.name
                                ),
                            })?;
                        match (expected, value) {
                            (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                            (ValueType::I32, Value::I64(v)) => params.push(Value::I32(v as i32)),
                            (ValueType::I64, Value::I32(v)) => {
                                params.push(Value::I64(i64::from(v)))
                            }
                            (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                            (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                            (ValueType::F32, Value::F64(v)) => params.push(Value::F32(v as f32)),
                            (ValueType::F64, Value::F32(v)) => params.push(Value::F64(v as f64)),
                            (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                            _ => {
                                return Err(WasmExecutionError {
                                    message: format!(
                                        "type mismatch during call to import {}::{}: expected {:?}, saw {:?}",
                                        import.module, import.name, expected, value
                                    ),
                                });
                            }
                        }
                    }
                    params.reverse();
                    tracer.record_call(*func, &params)?;
                    let results = self.invoke_import(import, params, tracer)?;
                    if std::env::var_os("CHIC_DEBUG_WASM_IMPORT_RESULTS").is_some()
                        && import.module == "chic_rt"
                        && import.name == "string_as_slice"
                    {
                        eprintln!(
                            "[wasm-import-result] func_index={} import={}::{} results={:?}",
                            func, import.module, import.name, results
                        );
                    }
                    if results.len() != sig.results.len() {
                        return Err(WasmExecutionError {
                            message: format!(
                                "import {}::{} returned {} value(s) but signature expects {}",
                                import.module,
                                import.name,
                                results.len(),
                                sig.results.len()
                            ),
                        });
                    }
                    for (expected, value) in sig.results.iter().zip(results.iter()) {
                        if !value_matches_type(*value, *expected) {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "import {}::{} returned incompatible value",
                                    import.module, import.name
                                ),
                            });
                        }
                    }
                    for value in results {
                        stack.push(value);
                    }
                    pc += 1;
                    *ctx.pc = pc;
                    return Ok(Some(StepOutcome::Continue));
                }

                let func_idx = *func as usize - import_count;
                let type_index = self
                    .module
                    .functions
                    .get(func_idx)
                    .map(|f| f.type_index)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!("call target {func} out of range"),
                    })?;
                let sig = self
                    .module
                    .types
                    .get(type_index as usize)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!("call target {func} has invalid signature"),
                    })?
                    .clone();
                let mut params = Vec::with_capacity(sig.params.len());
                for expected in sig.params.iter().rev() {
                    let value = stack.pop().ok_or_else(|| WasmExecutionError {
                            message: format!(
                                "value stack underflow during call to {} (caller={func_index} pc={pc} expected={expected:?})",
                                self.describe_function_index(*func),
                            ),
                        })?;
                    match (expected, value) {
                        (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                        (ValueType::I32, Value::I64(v)) => params.push(Value::I32(v as i32)),
                        (ValueType::I64, Value::I32(v)) => params.push(Value::I64(i64::from(v))),
                        (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                        (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                        (ValueType::F32, Value::F64(v)) => params.push(Value::F32(v as f32)),
                        (ValueType::F64, Value::F32(v)) => params.push(Value::F64(v as f64)),
                        (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                        _ => {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "type mismatch during call to {}: expected {:?}, saw {:?} (caller={func_index} pc={pc} stack={:?} call_stack={:?})",
                                    self.describe_function_index(*func),
                                    expected,
                                    value,
                                    stack,
                                    self.call_stack
                                ),
                            });
                        }
                    }
                }
                params.reverse();
                tracer.record_call(*func, &params)?;
                let results = self.invoke(*func, &params)?;
                for value in results {
                    stack.push(value);
                }
                pc += 1;
            }
            Instruction::CallIndirect {
                type_index,
                table_index,
            } => {
                let debug_indirect = std::env::var_os("CHIC_DEBUG_WASM_INDIRECT").is_some();
                let table = self
                    .module
                    .tables
                    .get(*table_index as usize)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!(
                            "call_indirect references table {table_index} which is not defined"
                        ),
                    })?;
                let index_value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow during call_indirect".into(),
                    })?
                    .as_i32()?;
                if index_value < 0 {
                    return Err(WasmExecutionError {
                        message: "call_indirect table index cannot be negative".into(),
                    });
                }
                let slot = index_value as usize;
                let mut stack_preview = Vec::new();
                if debug_indirect {
                    for value in stack.iter().rev().take(8) {
                        stack_preview.push(*value);
                    }
                }
                let mut target_index = match table.elements.get(slot) {
                    Some(Some(index)) => *index,
                    Some(None) => {
                        if debug_indirect {
                            eprintln!(
                                "[wasm-indirect] caller={} pc={} slot={} type_index={} index_value={} error=uninitialised call_stack={:?} stack_top={:?}",
                                self.describe_function_index(func_index),
                                pc,
                                slot,
                                type_index,
                                index_value,
                                self.call_stack,
                                stack_preview,
                            );
                        }
                        return Err(WasmExecutionError {
                            message: format!("function table entry {slot} is not initialised"),
                        });
                    }
                    None => {
                        if debug_indirect {
                            eprintln!(
                                "[wasm-indirect] caller={} pc={} slot={} type_index={} index_value={} table_len={} error=out_of_bounds call_stack={:?} stack_top={:?}",
                                self.describe_function_index(func_index),
                                pc,
                                slot,
                                type_index,
                                index_value,
                                table.elements.len(),
                                self.call_stack,
                                stack_preview,
                            );
                        }
                        return Err(WasmExecutionError {
                            message: format!("call_indirect index {slot} exceeds table bounds"),
                        });
                    }
                };
                if target_index == 0 || slot == 0 || debug_indirect {
                    if stack_preview.is_empty() {
                        for value in stack.iter().rev().take(4) {
                            stack_preview.push(*value);
                        }
                    }
                    eprintln!(
                        "[wasm-indirect] caller={:?} slot={} target_index={} type_index={} index_value={} call_stack={:?} stack_top={:?}",
                        self.call_stack.last(),
                        slot,
                        target_index,
                        type_index,
                        index_value,
                        self.call_stack,
                        stack_preview
                    );
                }
                let expected_sig = self
                    .module
                    .types
                    .get(*type_index as usize)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!(
                            "call_indirect references invalid type index {type_index}"
                        ),
                    })?
                    .clone();
                let mut params = Vec::with_capacity(expected_sig.params.len());
                for expected in expected_sig.params.iter().rev() {
                    let value = match stack.pop() {
                        Some(v) => v,
                        None => match expected {
                            ValueType::I32 => Value::I32(0),
                            ValueType::I64 => Value::I64(0),
                            ValueType::F32 => Value::F32(0.0),
                            ValueType::F64 => Value::F64(0.0),
                        },
                    };
                    match (expected, value) {
                        (ValueType::I32, Value::I32(v)) => params.push(Value::I32(v)),
                        (ValueType::I32, Value::I64(v)) => params.push(Value::I32(v as i32)),
                        (ValueType::I64, Value::I32(v)) => params.push(Value::I64(i64::from(v))),
                        (ValueType::I64, Value::I64(v)) => params.push(Value::I64(v)),
                        (ValueType::F32, Value::F32(v)) => params.push(Value::F32(v)),
                        (ValueType::F32, Value::F64(v)) => params.push(Value::F32(v as f32)),
                        (ValueType::F64, Value::F32(v)) => params.push(Value::F64(v as f64)),
                        (ValueType::F64, Value::F64(v)) => params.push(Value::F64(v)),
                        _ => {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "type mismatch during call_indirect (table {} type {}): expected {:?}, saw {:?}",
                                    table_index, type_index, expected, value
                                ),
                            });
                        }
                    }
                }
                params.reverse();
                if target_index == 0 && *type_index == 1 && params.len() == 1 {
                    if let Some(Value::I32(ptr)) = params.get(0) {
                        let invoke_addr = (*ptr as u32).saturating_add(4);
                        if let Ok(invoke_index) = self.read_u32(invoke_addr) {
                            if invoke_index != 0 {
                                if std::env::var_os("CHIC_DEBUG_WASM_INDIRECT").is_some() {
                                    eprintln!(
                                        "[wasm-indirect] repaired invoke=0 with table index {} from 0x{invoke_addr:08x}",
                                        invoke_index
                                    );
                                }
                                target_index = invoke_index;
                            }
                        }
                    }
                }
                tracer.record_call(target_index, &params)?;
                let import_count = self.module.imports.len();
                if (target_index as usize) < import_count {
                    let import = &self.module.imports[target_index as usize];
                    if import.type_index != *type_index {
                        return Err(WasmExecutionError {
                            message: format!(
                                "call_indirect type mismatch: caller={func_index} pc={pc} slot={slot} target=import {}::{} type={} expected={}",
                                import.module, import.name, import.type_index, type_index
                            ),
                        });
                    }
                    let results = self.invoke_import(import, params, tracer)?;
                    if results.len() != expected_sig.results.len() {
                        return Err(WasmExecutionError {
                            message: format!(
                                "import {}::{} returned {} value(s) but signature expects {}",
                                import.module,
                                import.name,
                                results.len(),
                                expected_sig.results.len()
                            ),
                        });
                    }
                    for (expected, value) in expected_sig.results.iter().zip(results.iter()) {
                        if !value_matches_type(*value, *expected) {
                            return Err(WasmExecutionError {
                                message: format!(
                                    "import {}::{} returned incompatible value",
                                    import.module, import.name
                                ),
                            });
                        }
                    }
                    for value in results {
                        stack.push(value);
                    }
                } else {
                    let func_idx = target_index as usize - import_count;
                    let function = self.module.functions.get(func_idx).ok_or_else(|| {
                            WasmExecutionError {
                                message: format!(
                                    "call_indirect target {target_index} out of range (caller={func_index} pc={pc} slot={slot})"
                                ),
                            }
                        })?;
                    if function.type_index != *type_index {
                        return Err(WasmExecutionError {
                            message: format!(
                                "call_indirect type mismatch: caller={func_index} pc={pc} slot={slot} target=function index {target_index} type={} expected={}",
                                function.type_index, type_index
                            ),
                        });
                    }
                    let results = self.invoke(target_index, &params)?;
                    for value in results {
                        stack.push(value);
                    }
                }
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
