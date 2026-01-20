use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_int_ops(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let stack = &mut *ctx.stack;
        match &code[pc] {
            Instruction::I32Eq => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a == b))?;
                pc += 1;
            }
            Instruction::I32Ne => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a != b))?;
                pc += 1;
            }
            Instruction::I32Eqz => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on eqz".into(),
                    })?
                    .as_i32()?;
                stack.push(Value::I32(i32::from(value == 0)));
                pc += 1;
            }
            Instruction::I32LtS => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a < b))?;
                pc += 1;
            }
            Instruction::I32LtU => {
                binary_i32(stack, Value::I32, |a, b| i32::from((a as u32) < (b as u32)))?;
                pc += 1;
            }
            Instruction::I32LeS => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a <= b))?;
                pc += 1;
            }
            Instruction::I32LeU => {
                binary_i32(stack, Value::I32, |a, b| {
                    i32::from((a as u32) <= (b as u32))
                })?;
                pc += 1;
            }
            Instruction::I32GtS => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a > b))?;
                pc += 1;
            }
            Instruction::I32GtU => {
                binary_i32(stack, Value::I32, |a, b| i32::from((a as u32) > (b as u32)))?;
                pc += 1;
            }
            Instruction::I32GeS => {
                binary_i32(stack, Value::I32, |a, b| i32::from(a >= b))?;
                pc += 1;
            }
            Instruction::I32GeU => {
                binary_i32(stack, Value::I32, |a, b| {
                    i32::from((a as u32) >= (b as u32))
                })?;
                pc += 1;
            }
            Instruction::F32Eq => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.eq".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.eq".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? == rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F32Ne => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.ne".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.ne".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? != rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F32Lt => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.lt".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.lt".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? < rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F32Gt => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.gt".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.gt".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? > rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F32Le => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.le".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.le".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? <= rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F32Ge => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.ge".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.ge".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f32()? >= rhs.as_f32()?)));
                pc += 1;
            }
            Instruction::F64Eq => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.eq".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.eq".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? == rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::F64Ne => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.ne".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.ne".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? != rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::F64Lt => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.lt".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.lt".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? < rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::F64Gt => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.gt".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.gt".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? > rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::F64Le => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.le".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.le".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? <= rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::F64Ge => {
                let rhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.ge".into(),
                })?;
                let lhs = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.ge".into(),
                })?;
                stack.push(Value::I32(i32::from(lhs.as_f64()? >= rhs.as_f64()?)));
                pc += 1;
            }
            Instruction::I64Eq => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.eq".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.eq".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left == right)));
                pc += 1;
            }
            Instruction::I64Ne => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ne".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ne".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left != right)));
                pc += 1;
            }
            Instruction::I64LtS => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.lt_s".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.lt_s".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left < right)));
                pc += 1;
            }
            Instruction::I64LeS => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.le_s".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.le_s".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left <= right)));
                pc += 1;
            }
            Instruction::I64GtS => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.gt_s".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.gt_s".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left > right)));
                pc += 1;
            }
            Instruction::I64GeS => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ge_s".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ge_s".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from(left >= right)));
                pc += 1;
            }
            Instruction::I64LtU => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.lt_u".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.lt_u".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from((left as u64) < (right as u64))));
                pc += 1;
            }
            Instruction::I64LeU => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.le_u".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.le_u".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from((left as u64) <= (right as u64))));
                pc += 1;
            }
            Instruction::I64GtU => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.gt_u".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.gt_u".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from((left as u64) > (right as u64))));
                pc += 1;
            }
            Instruction::I64GeU => {
                let right = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ge_u".into(),
                    })?
                    .as_i64()?;
                let left = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.ge_u".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(i32::from((left as u64) >= (right as u64))));
                pc += 1;
            }
            Instruction::I32Add => {
                binary_i32(stack, Value::I32, i32::wrapping_add)?;
                pc += 1;
            }
            Instruction::I32Sub => {
                binary_i32(stack, Value::I32, i32::wrapping_sub)?;
                pc += 1;
            }
            Instruction::I32Mul => {
                binary_i32(stack, Value::I32, i32::wrapping_mul)?;
                pc += 1;
            }
            Instruction::I32DivS => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "division by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                stack.push(Value::I32(dividend.wrapping_div(divisor)));
                pc += 1;
            }
            Instruction::I32DivU => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "division by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                let result = (dividend as u32).wrapping_div(divisor as u32);
                stack.push(Value::I32(result as i32));
                pc += 1;
            }
            Instruction::I32RemS => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "remainder by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                stack.push(Value::I32(dividend.wrapping_rem(divisor)));
                pc += 1;
            }
            Instruction::I32RemU => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "remainder by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i32()?;
                let result = (dividend as u32).wrapping_rem(divisor as u32);
                stack.push(Value::I32(result as i32));
                pc += 1;
            }
            Instruction::I32And => {
                binary_i32(stack, Value::I32, |a, b| a & b)?;
                pc += 1;
            }
            Instruction::I32Or => {
                binary_i32(stack, Value::I32, |a, b| a | b)?;
                pc += 1;
            }
            Instruction::I32Xor => {
                binary_i32(stack, Value::I32, |a, b| a ^ b)?;
                pc += 1;
            }
            Instruction::I64And => {
                binary_i64(stack, Value::I64, |a, b| a & b)?;
                pc += 1;
            }
            Instruction::I64Or => {
                binary_i64(stack, Value::I64, |a, b| a | b)?;
                pc += 1;
            }
            Instruction::I64Xor => {
                binary_i64(stack, Value::I64, |a, b| a ^ b)?;
                pc += 1;
            }
            Instruction::I64Eqz => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.eqz".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32((value == 0) as i32));
                pc += 1;
            }
            Instruction::I64Add => {
                binary_i64(stack, Value::I64, |a, b| a.wrapping_add(b))?;
                pc += 1;
            }
            Instruction::I64Sub => {
                binary_i64(stack, Value::I64, |a, b| a.wrapping_sub(b))?;
                pc += 1;
            }
            Instruction::I64Mul => {
                binary_i64(stack, Value::I64, |a, b| a.wrapping_mul(b))?;
                pc += 1;
            }
            Instruction::I64DivS => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "division by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I64(dividend.wrapping_div(divisor)));
                pc += 1;
            }
            Instruction::I64DivU => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "division by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                let result = (dividend as u64).wrapping_div(divisor as u64);
                stack.push(Value::I64(result as i64));
                pc += 1;
            }
            Instruction::I64RemS => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "remainder by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I64(dividend.wrapping_rem(divisor)));
                pc += 1;
            }
            Instruction::I64RemU => {
                let divisor = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                if divisor == 0 {
                    return Err(WasmExecutionError {
                        message: "remainder by zero".into(),
                    });
                }
                let dividend = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow".into(),
                    })?
                    .as_i64()?;
                let result = (dividend as u64).wrapping_rem(divisor as u64);
                stack.push(Value::I64(result as i64));
                pc += 1;
            }
            Instruction::I32Shl => {
                binary_i32(stack, Value::I32, |a, b| a.wrapping_shl(shift_amount(b)))?;
                pc += 1;
            }
            Instruction::I32ShrS => {
                binary_i32(stack, Value::I32, |a, b| a.wrapping_shr(shift_amount(b)))?;
                pc += 1;
            }
            Instruction::I32ShrU => {
                binary_i32(stack, Value::I32, |a, b| {
                    let shifted = (a as u32).wrapping_shr(shift_amount(b));
                    i32::from_ne_bytes(shifted.to_ne_bytes())
                })?;
                pc += 1;
            }
            Instruction::I64Shl => {
                binary_i64(stack, Value::I64, |a, b| {
                    a.wrapping_shl(shift_amount_i64(b))
                })?;
                pc += 1;
            }
            Instruction::I64ShrS => {
                binary_i64(stack, Value::I64, |a, b| {
                    a.wrapping_shr(shift_amount_i64(b))
                })?;
                pc += 1;
            }
            Instruction::I64ShrU => {
                binary_i64(stack, Value::I64, |a, b| {
                    let shifted = (a as u64).wrapping_shr(shift_amount_i64(b));
                    i64::from_ne_bytes(shifted.to_ne_bytes())
                })?;
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
