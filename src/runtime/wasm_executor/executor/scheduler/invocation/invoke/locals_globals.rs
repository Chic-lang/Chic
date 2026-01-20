use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_locals_globals(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let func_index = ctx.func_index;
        let stack = &mut *ctx.stack;
        let locals = &mut *ctx.locals;
        match &code[pc] {
            Instruction::LocalGet(index) => {
                let value = locals
                    .get(*index as usize)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!(
                            "local.get index {index} out of range (locals={} func={})",
                            locals.len(),
                            func_index
                        ),
                    })?;
                stack.push(*value);
                pc += 1;
            }
            Instruction::LocalSet(index) => {
                let value = stack.pop().unwrap_or(Value::I32(0));
                if let Some(slot) = locals.get_mut(*index as usize) {
                    *slot = value;
                } else {
                    return Err(WasmExecutionError {
                        message: format!(
                            "local.set index {index} out of range (locals={} func={})",
                            locals.len(),
                            func_index
                        ),
                    });
                }
                pc += 1;
            }
            Instruction::LocalTee(index) => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on local.tee".into(),
                })?;
                if let Some(slot) = locals.get_mut(*index as usize) {
                    *slot = value;
                } else {
                    return Err(WasmExecutionError {
                        message: format!(
                            "local.tee index {index} out of range (locals={} func={})",
                            locals.len(),
                            func_index
                        ),
                    });
                }
                stack.push(value);
                pc += 1;
            }
            Instruction::GlobalGet(index) => {
                let value = self
                    .globals
                    .get(*index as usize)
                    .ok_or_else(|| WasmExecutionError {
                        message: format!("global index {index} out of range"),
                    })?
                    .value;
                if *index == 0 && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                    eprintln!(
                        "[wasm-sp] get func={} pc={} value={:?}",
                        func_index, pc, value
                    );
                }
                stack.push(value);
                pc += 1;
            }
            Instruction::GlobalSet(index) => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on global.set".into(),
                })?;
                let global =
                    self.globals
                        .get_mut(*index as usize)
                        .ok_or_else(|| WasmExecutionError {
                            message: format!("global index {index} out of range"),
                        })?;
                if !global.mutable {
                    return Err(WasmExecutionError {
                        message: format!("global {index} is immutable"),
                    });
                }
                if !value_matches_type(value, global.ty) {
                    return Err(WasmExecutionError {
                        message: "type mismatch during global.set".into(),
                    });
                }
                if *index == 0 && std::env::var_os("CHIC_DEBUG_WASM_SP").is_some() {
                    eprintln!(
                        "[wasm-sp] set func={} pc={} value={:?}",
                        func_index, pc, value
                    );
                }
                global.value = value;
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
