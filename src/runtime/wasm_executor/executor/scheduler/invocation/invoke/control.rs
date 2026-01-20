use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_control(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let stack = &mut *ctx.stack;
        let control_stack = &mut *ctx.control_stack;
        match &code[pc] {
            Instruction::Block { end } => {
                control_stack.push(ControlLabel {
                    kind: ControlKind::Block,
                    target_pc: *end,
                });
                pc += 1;
            }
            Instruction::Loop { .. } => {
                control_stack.push(ControlLabel {
                    kind: ControlKind::Loop,
                    target_pc: pc + 1,
                });
                pc += 1;
            }
            Instruction::If { end } => {
                let cond = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on `if`".into(),
                    })?
                    .as_i32()?;
                control_stack.push(ControlLabel {
                    kind: ControlKind::If,
                    target_pc: *end,
                });
                if cond != 0 {
                    pc += 1;
                } else {
                    control_stack.pop();
                    pc = *end;
                }
            }
            Instruction::End => {
                control_stack.pop();
                pc += 1;
            }
            Instruction::Br { depth } => {
                if *depth as usize >= control_stack.len() {
                    return Err(WasmExecutionError {
                        message: format!("branch depth {depth} exceeds control stack"),
                    });
                }
                let target_index = control_stack.len() - 1 - *depth as usize;
                let label = control_stack[target_index];
                if matches!(label.kind, ControlKind::Loop) {
                    control_stack.truncate(target_index + 1);
                } else {
                    control_stack.truncate(target_index);
                }
                pc = label.target_pc;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
