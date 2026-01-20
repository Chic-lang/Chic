use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_terminators(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let pc = *ctx.pc;
        let func_index = ctx.func_index;
        let expects_results = ctx.expects_results;
        let result_types = ctx.result_types;
        let stack = &mut *ctx.stack;
        let return_values = &mut *ctx.return_values;
        match &code[pc] {
            Instruction::Return => {
                if expects_results {
                    if stack.len() < result_types.len() {
                        return Err(WasmExecutionError {
                            message: "value stack underflow on return".into(),
                        });
                    }
                    let mut results = Vec::with_capacity(result_types.len());
                    for _ in 0..result_types.len() {
                        results.push(stack.pop().expect("stack length checked"));
                    }
                    results.reverse();
                    for (value, expected) in results.iter().zip(result_types.iter()) {
                        if !value_matches_type(*value, *expected) {
                            return Err(WasmExecutionError {
                                message: format!("return type mismatch: expected {:?}", expected),
                            });
                        }
                    }
                    return_values.clear();
                    return_values.extend(results);
                } else {
                    let _ = stack.pop();
                    return_values.clear();
                }
                return Ok(Some(StepOutcome::Halt));
            }
            Instruction::Unreachable => {
                let exported = self
                    .module
                    .exports
                    .iter()
                    .filter_map(|(name, &index)| {
                        if index == func_index {
                            Some(name.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let export_suffix = if exported.is_empty() {
                    String::new()
                } else {
                    format!(" exports=[{}]", exported.join(", "))
                };
                return Err(WasmExecutionError {
                    message: format!(
                        "reached unreachable instruction (func={func_index}{export_suffix} pc={pc})"
                    ),
                });
            }
            _ => return Ok(None),
        }
    }
}
