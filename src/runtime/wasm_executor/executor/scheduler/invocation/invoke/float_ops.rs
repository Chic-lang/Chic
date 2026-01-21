use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_float_ops(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let stack = &mut *ctx.stack;
        match &code[pc] {
            Instruction::F32Add => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.add".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.add".into(),
                    })?,
                );
                let lhs = lhs.as_f32()?;
                let rhs = rhs.as_f32()?;
                let exact = f64::from(lhs) + f64::from(rhs);
                let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                record_arithmetic_flags(
                    f64::from(lhs),
                    f64::from(rhs),
                    exact,
                    f64::from(rounded),
                    false,
                );
                stack.push(Value::F32(rounded));
                pc += 1;
            }
            Instruction::F32Sub => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.sub".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.sub".into(),
                    })?,
                );
                let lhs = lhs.as_f32()?;
                let rhs = rhs.as_f32()?;
                let exact = f64::from(lhs) - f64::from(rhs);
                let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                record_arithmetic_flags(
                    f64::from(lhs),
                    f64::from(rhs),
                    exact,
                    f64::from(rounded),
                    false,
                );
                stack.push(Value::F32(rounded));
                pc += 1;
            }
            Instruction::F32Mul => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.mul".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.mul".into(),
                    })?,
                );
                let lhs = lhs.as_f32()?;
                let rhs = rhs.as_f32()?;
                let exact = f64::from(lhs) * f64::from(rhs);
                let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                record_arithmetic_flags(
                    f64::from(lhs),
                    f64::from(rhs),
                    exact,
                    f64::from(rounded),
                    false,
                );
                stack.push(Value::F32(rounded));
                pc += 1;
            }
            Instruction::F32Div => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.div".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f32.div".into(),
                    })?,
                );
                let lhs = lhs.as_f32()?;
                let rhs = rhs.as_f32()?;
                let exact = f64::from(lhs) / f64::from(rhs);
                let rounded = adjust_rounding_f32(exact, exact as f32, rounding_mode());
                record_arithmetic_flags(
                    f64::from(lhs),
                    f64::from(rhs),
                    exact,
                    f64::from(rounded),
                    true,
                );
                stack.push(Value::F32(rounded));
                pc += 1;
            }
            Instruction::F32DemoteF64 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.demote_f64".into(),
                })?;
                let converted = match value {
                    Value::F64(v) => round_f64_to_f32(v, rounding_mode()),
                    Value::F32(v) => v,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f32.demote_f64".into(),
                        });
                    }
                };
                if std::env::var_os("CHIC_DEBUG_WASM_DEMOTE").is_some() {
                    eprintln!(
                        "[wasm-demote] value={:016x} -> {:08x} flags={:?}",
                        match value {
                            Value::F64(v) => v.to_bits(),
                            _ => 0,
                        },
                        converted.to_bits(),
                        crate::runtime::float_env::read_flags()
                    );
                }
                stack.push(Value::F32(converted));
                pc += 1;
            }
            Instruction::F32Trunc => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.trunc".into(),
                })?;
                let source = value.as_f32()?;
                let truncated = source.trunc();
                let mut flags = FloatStatusFlags::default();
                if source.is_nan() {
                    flags.invalid = true;
                }
                if truncated != source {
                    flags.inexact = true;
                }
                record_flags(flags);
                stack.push(Value::F32(truncated));
                pc += 1;
            }
            Instruction::F64Add => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.add".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.add".into(),
                    })?,
                );
                let lhs = lhs.as_f64()?;
                let rhs = rhs.as_f64()?;
                let exact = lhs + rhs;
                let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                stack.push(Value::F64(rounded));
                pc += 1;
            }
            Instruction::F64Sub => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.sub".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.sub".into(),
                    })?,
                );
                let lhs = lhs.as_f64()?;
                let rhs = rhs.as_f64()?;
                let exact = lhs - rhs;
                let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                stack.push(Value::F64(rounded));
                pc += 1;
            }
            Instruction::F64Mul => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.mul".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.mul".into(),
                    })?,
                );
                let lhs = lhs.as_f64()?;
                let rhs = rhs.as_f64()?;
                let exact = lhs * rhs;
                let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                record_arithmetic_flags(lhs, rhs, exact, rounded, false);
                stack.push(Value::F64(rounded));
                pc += 1;
            }
            Instruction::F64Div => {
                let (rhs, lhs) = (
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.div".into(),
                    })?,
                    stack.pop().ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on f64.div".into(),
                    })?,
                );
                let lhs = lhs.as_f64()?;
                let rhs = rhs.as_f64()?;
                let exact = lhs / rhs;
                let rounded = adjust_rounding_f64(exact, exact, rounding_mode());
                record_arithmetic_flags(lhs, rhs, exact, rounded, true);
                stack.push(Value::F64(rounded));
                pc += 1;
            }
            Instruction::F64Trunc => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.trunc".into(),
                })?;
                let source = value.as_f64()?;
                let truncated = source.trunc();
                let mut flags = FloatStatusFlags::default();
                if source.is_nan() {
                    flags.invalid = true;
                }
                if truncated != source {
                    flags.inexact = true;
                }
                record_flags(flags);
                stack.push(Value::F64(truncated));
                pc += 1;
            }
            Instruction::I32ReinterpretF32 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i32.reinterpret_f32".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => Value::I32(i32::from_ne_bytes(v.to_bits().to_ne_bytes())),
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i32.reinterpret_f32".into(),
                        });
                    }
                };
                stack.push(converted);
                pc += 1;
            }
            Instruction::I64ReinterpretF64 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.reinterpret_f64".into(),
                })?;
                let converted = match value {
                    Value::F64(v) => Value::I64(i64::from_ne_bytes(v.to_bits().to_ne_bytes())),
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i64.reinterpret_f64".into(),
                        });
                    }
                };
                stack.push(converted);
                pc += 1;
            }
            Instruction::F32ReinterpretI32 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.reinterpret_i32".into(),
                })?;
                let converted = match value {
                    Value::I32(v) => Value::F32(f32::from_bits(v as u32)),
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f32.reinterpret_i32".into(),
                        });
                    }
                };
                stack.push(converted);
                pc += 1;
            }
            Instruction::F64ReinterpretI64 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.reinterpret_i64".into(),
                })?;
                let converted = match value {
                    Value::I64(v) => Value::F64(f64::from_bits(v as u64)),
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f64.reinterpret_i64".into(),
                        });
                    }
                };
                stack.push(converted);
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
