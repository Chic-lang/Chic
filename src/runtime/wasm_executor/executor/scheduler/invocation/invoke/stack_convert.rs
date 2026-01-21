use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_stack_convert(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let stack = &mut *ctx.stack;
        match &code[pc] {
            Instruction::Drop => {
                stack.pop();
                pc += 1;
            }
            Instruction::I32Const(value) => {
                stack.push(Value::I32(*value));
                pc += 1;
            }
            Instruction::I64Const(value) => {
                stack.push(Value::I64(*value));
                pc += 1;
            }
            Instruction::F32Const(value) => {
                stack.push(Value::F32(*value));
                pc += 1;
            }
            Instruction::F64Const(value) => {
                stack.push(Value::F64(*value));
                pc += 1;
            }
            Instruction::I32WrapI64 => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.wrap_i64".into(),
                    })?
                    .as_i64()?;
                stack.push(Value::I32(value as i32));
                pc += 1;
            }
            Instruction::I64ExtendI32S => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.extend_i32_s".into(),
                    })?
                    .as_i32()?;
                stack.push(Value::I64(value as i64));
                pc += 1;
            }
            Instruction::I64ExtendI32U => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i64.extend_i32_u".into(),
                    })?
                    .as_i32()?;
                let extended = u64::from(value as u32) as i64;
                stack.push(Value::I64(extended));
                pc += 1;
            }
            Instruction::F32ConvertI32S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.convert_i32_s".into(),
                })?;
                let raw = value.as_i32()?;
                let mode = rounding_mode();
                let converted = convert_int_to_f32(i128::from(raw), mode);
                stack.push(Value::F32(converted));
                pc += 1;
            }
            Instruction::F32ConvertI32U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.convert_i32_u".into(),
                })?;
                let raw = value.as_i32()?;
                let converted = convert_int_to_f32(i128::from(raw), rounding_mode());
                stack.push(Value::F32(converted));
                pc += 1;
            }
            Instruction::F32ConvertI64S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.convert_i64_s".into(),
                })?;
                let raw = value.as_i64()?;
                let converted = convert_int_to_f32(raw as i128, rounding_mode());
                stack.push(Value::F32(converted));
                pc += 1;
            }
            Instruction::F32ConvertI64U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.convert_i64_u".into(),
                })?;
                let raw = value.as_i64()?;
                let converted = convert_int_to_f32(raw as i128, rounding_mode());
                stack.push(Value::F32(converted));
                pc += 1;
            }
            Instruction::F64ConvertI32S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.convert_i32_s".into(),
                })?;
                let raw = value.as_i32()?;
                let converted = convert_int_to_f64(i128::from(raw), rounding_mode());
                stack.push(Value::F64(converted));
                pc += 1;
            }
            Instruction::F64ConvertI32U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.convert_i32_u".into(),
                })?;
                let raw = value.as_i32()?;
                let converted = convert_int_to_f64(i128::from(raw), rounding_mode());
                stack.push(Value::F64(converted));
                pc += 1;
            }
            Instruction::F64ConvertI64S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.convert_i64_s".into(),
                })?;
                let raw = value.as_i64()?;
                let converted = convert_int_to_f64(raw as i128, rounding_mode());
                stack.push(Value::F64(converted));
                pc += 1;
            }
            Instruction::F64ConvertI64U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.convert_i64_u".into(),
                })?;
                let raw = value.as_i64()?;
                let converted = convert_int_to_f64(raw as i128, rounding_mode());
                stack.push(Value::F64(converted));
                pc += 1;
            }
            Instruction::I32TruncF32S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i32.trunc_f32_s".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(
                            f64::from(v),
                            rounded,
                            f64::from(i32::MIN),
                            f64::from(i32::MAX),
                        );
                        rounded as i32
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(
                            v,
                            rounded,
                            f64::from(i32::MIN),
                            f64::from(i32::MAX),
                        );
                        rounded as i32
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i32.trunc_f32_s".into(),
                        });
                    }
                };
                stack.push(Value::I32(converted));
                pc += 1;
            }
            Instruction::I32TruncF32U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i32.trunc_f32_u".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                        rounded as u32
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                        rounded as u32
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i32.trunc_f32_u".into(),
                        });
                    }
                };
                stack.push(Value::I32(converted as i32));
                pc += 1;
            }
            Instruction::I32TruncF64S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i32.trunc_f64_s".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(
                            f64::from(v),
                            rounded,
                            f64::from(i32::MIN),
                            f64::from(i32::MAX),
                        );
                        rounded as i32
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(
                            v,
                            rounded,
                            f64::from(i32::MIN),
                            f64::from(i32::MAX),
                        );
                        rounded as i32
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i32.trunc_f64_s".into(),
                        });
                    }
                };
                stack.push(Value::I32(converted));
                pc += 1;
            }
            Instruction::I32TruncF64U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i32.trunc_f64_u".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                        rounded as u32
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(rounded, rounded, 0.0, f64::from(u32::MAX));
                        rounded as u32
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i32.trunc_f64_u".into(),
                        });
                    }
                };
                stack.push(Value::I32(converted as i32));
                pc += 1;
            }
            Instruction::I64TruncF32S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.trunc_f32_s".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(
                            f64::from(v),
                            rounded,
                            i64::MIN as f64,
                            i64::MAX as f64,
                        );
                        rounded as i64
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(v, rounded, i64::MIN as f64, i64::MAX as f64);
                        rounded as i64
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i64.trunc_f32_s".into(),
                        });
                    }
                };
                stack.push(Value::I64(converted));
                pc += 1;
            }
            Instruction::I64TruncF32U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.trunc_f32_u".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                        rounded as u64
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                        rounded as u64
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i64.trunc_f32_u".into(),
                        });
                    }
                };
                stack.push(Value::I64(converted as i64));
                pc += 1;
            }
            Instruction::I64TruncF64S => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.trunc_f64_s".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(
                            f64::from(v),
                            rounded,
                            i64::MIN as f64,
                            i64::MAX as f64,
                        );
                        rounded as i64
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(v, rounded, i64::MIN as f64, i64::MAX as f64);
                        rounded as i64
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i64.trunc_f64_s".into(),
                        });
                    }
                };
                stack.push(Value::I64(converted));
                pc += 1;
            }
            Instruction::I64TruncF64U => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.trunc_f64_u".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(f64::from(v), mode);
                        record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                        rounded as u64
                    }
                    Value::F64(v) => {
                        let mode = rounding_mode();
                        let rounded = round_value(v, mode);
                        record_conversion_flags(rounded, rounded, 0.0, u64::MAX as f64);
                        rounded as u64
                    }
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during i64.trunc_f64_u".into(),
                        });
                    }
                };
                stack.push(Value::I64(converted as i64));
                pc += 1;
            }
            Instruction::F64PromoteF32 => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.promote_f32".into(),
                })?;
                let converted = match value {
                    Value::F32(v) => v as f64,
                    Value::F64(v) => v,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f64.promote_f32".into(),
                        });
                    }
                };
                stack.push(Value::F64(converted));
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
