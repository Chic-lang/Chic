use super::*;

impl<'a> Executor<'a> {
    pub(super) fn step_memory(
        &mut self,
        ctx: &mut StepContext<'_, '_>,
    ) -> Result<Option<StepOutcome>, WasmExecutionError> {
        let code = ctx.code;
        let mut pc = *ctx.pc;
        let func_index = ctx.func_index;
        let stack = &mut *ctx.stack;
        let locals = &mut *ctx.locals;
        match &code[pc] {
            Instruction::I32Load { offset } => {
                let addr = pop_address(stack, "i32.load")?;
                let value = self.load_i32(addr, *offset)?;
                stack.push(Value::I32(value));
                pc += 1;
            }
            Instruction::I32Load8S { offset } => {
                let addr = pop_address(stack, "i32.load8_s")?;
                let byte = match self.load_bytes(addr, *offset, 1) {
                    Ok(bytes) => bytes[0] as i8,
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                };
                stack.push(Value::I32(i32::from(byte)));
                pc += 1;
            }
            Instruction::I32Load8U { offset } => {
                let addr = pop_address(stack, "i32.load8_u")?;
                let byte = match self.load_bytes(addr, *offset, 1) {
                    Ok(bytes) => bytes[0],
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        let locals_snapshot =
                            if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                format!(" locals={locals:?}")
                            } else {
                                String::new()
                            };
                        let call_stack_snapshot =
                            if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                                format!(" call_stack={:?}", self.call_stack)
                            } else {
                                String::new()
                            };
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={}{}{})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len(),
                            locals_snapshot,
                            call_stack_snapshot
                        );
                        return Err(err);
                    }
                };
                stack.push(Value::I32(i32::from(byte)));
                pc += 1;
            }
            Instruction::I32Load16S { offset } => {
                let addr = pop_address(stack, "i32.load16_s")?;
                let bytes = match self.load_bytes(addr, *offset, 2) {
                    Ok(bytes) => bytes,
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                };
                let value = i16::from_le_bytes([bytes[0], bytes[1]]);
                stack.push(Value::I32(i32::from(value)));
                pc += 1;
            }
            Instruction::I32Load16U { offset } => {
                let addr = pop_address(stack, "i32.load16_u")?;
                let bytes = match self.load_bytes(addr, *offset, 2) {
                    Ok(bytes) => bytes,
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| match g.value {
                                Value::I32(v) => Some(v),
                                _ => None,
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            offset,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                };
                let value = u16::from_le_bytes([bytes[0], bytes[1]]);
                stack.push(Value::I32(i32::from(value)));
                pc += 1;
            }
            Instruction::I64Load { offset } => {
                let addr = match pop_address(stack, "i64.load") {
                    Ok(addr) => addr,
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| {
                                if let Value::I32(v) = g.value {
                                    Some(v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} sp_global={} stack_len={} stack={:?})",
                            err.message,
                            func_index,
                            pc,
                            sp_snapshot,
                            stack.len(),
                            self.call_stack
                        );
                        return Err(err);
                    }
                };
                let value = self.load_i64(addr, *offset)?;
                stack.push(Value::I64(value));
                pc += 1;
            }
            Instruction::F32Load { offset } => {
                let addr = pop_address(stack, "f32.load")?;
                let value = self.load_f32(addr, *offset)?;
                stack.push(Value::F32(value));
                pc += 1;
            }
            Instruction::F64Load { offset } => {
                let addr = pop_address(stack, "f64.load")?;
                let value = self.load_f64(addr, *offset)?;
                stack.push(Value::F64(value));
                pc += 1;
            }
            Instruction::I32Store { offset } => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.store".into(),
                    })?
                    .as_i32()?;
                let addr = match pop_address(stack, "i32.store") {
                    Ok(addr) => addr,
                    Err(mut err) => {
                        let sp_snapshot = self
                            .globals
                            .get(0)
                            .and_then(|g| {
                                if let Value::I32(v) = g.value {
                                    Some(v)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(-1);
                        err.message = format!(
                            "{} (func={} pc={} value={} sp_global={} stack_len={})",
                            err.message,
                            func_index,
                            pc,
                            value,
                            sp_snapshot,
                            stack.len()
                        );
                        return Err(err);
                    }
                };
                if (addr as i32) < 0 {
                    return Err(WasmExecutionError {
                        message: format!(
                            "negative memory address on i32.store (addr=0x{addr:08x} value={value} func={} pc={})",
                            func_index, pc
                        ),
                    });
                }
                if std::env::var_os("CHIC_DEBUG_WASM_FN_RUNTIME").is_some() {
                    if let Some((start, end)) = self.tracked_fn_range {
                        let effective = (addr as u32).saturating_add(*offset as u32);
                        if effective >= start && effective < end {
                            let snapshot = self.read_bytes(start, end.saturating_sub(start)).ok();
                            eprintln!(
                                "[wasm-fn-runtime] store i32 func={} pc={} addr=0x{:08x} off={} value=0x{:08x} range=0x{:08x}-0x{:08x} snapshot={:?}",
                                func_index,
                                pc,
                                addr as u32,
                                offset,
                                value as u32,
                                start,
                                end,
                                snapshot
                            );
                        }
                    }
                }
                if let Err(mut err) = self.store_i32(addr, *offset, value) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:08x} sp_global={} stack_len={})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        value as u32,
                        sp_snapshot,
                        stack.len()
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::I32Store8 { offset } => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.store8".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.store8")?;
                if let Err(mut err) = self.store_i8(addr, *offset, value as u8) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    let locals_snapshot =
                        if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                            format!(" locals={locals:?}")
                        } else {
                            String::new()
                        };
                    let call_stack_snapshot =
                        if std::env::var_os("CHIC_DEBUG_WASM_TRAP_LOCALS").is_some() {
                            format!(" call_stack={:?}", self.call_stack)
                        } else {
                            String::new()
                        };
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:02x} sp_global={} stack_len={}{}{})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        value as u8,
                        sp_snapshot,
                        stack.len(),
                        locals_snapshot,
                        call_stack_snapshot
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::I32Store16 { offset } => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.store16".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.store16")?;
                let bytes = (value as u16).to_le_bytes();
                if let Err(mut err) = self.store_bytes(addr as u32, *offset, &bytes) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:04x} sp_global={} stack_len={})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        value as u16,
                        sp_snapshot,
                        stack.len()
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::I64Store { offset } => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.store".into(),
                })?;
                let i64_value = match value {
                    Value::I64(v) => v,
                    _ => {
                        return Err(WasmExecutionError {
                            message: format!(
                                "type mismatch during i64.store (func={func_index} pc={pc} value={value:?})"
                            ),
                        });
                    }
                };
                let addr = pop_address(stack, "i64.store")?;
                if std::env::var_os("CHIC_DEBUG_WASM_FN_RUNTIME").is_some() {
                    if let Some((start, end)) = self.tracked_fn_range {
                        let effective = (addr as u32).saturating_add(*offset as u32);
                        if effective >= start && effective < end {
                            let snapshot = self.read_bytes(start, end.saturating_sub(start)).ok();
                            eprintln!(
                                "[wasm-fn-runtime] store i64 func={} pc={} addr=0x{:08x} off={} value=0x{:016x} range=0x{:08x}-0x{:08x} snapshot={:?}",
                                func_index,
                                pc,
                                addr as u32,
                                offset,
                                i64_value as u64,
                                start,
                                end,
                                snapshot
                            );
                        }
                    }
                }
                if let Err(mut err) = self.store_i64(addr, *offset, i64_value) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} value=0x{:016x} sp_global={} stack_len={})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        i64_value as u64,
                        sp_snapshot,
                        stack.len()
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::F32Store { offset } => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f32.store".into(),
                })?;
                let f32_value = match value {
                    Value::F32(v) => v,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f32.store".into(),
                        });
                    }
                };
                let addr = pop_address(stack, "f32.store")?;
                if let Err(mut err) = self.store_f32(addr, *offset, f32_value) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        sp_snapshot,
                        stack.len()
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::F64Store { offset } => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on f64.store".into(),
                })?;
                let f64_value = match value {
                    Value::F64(v) => v,
                    _ => {
                        return Err(WasmExecutionError {
                            message: "type mismatch during f64.store".into(),
                        });
                    }
                };
                let addr = pop_address(stack, "f64.store")?;
                if let Err(mut err) = self.store_f64(addr, *offset, f64_value) {
                    let sp_snapshot = self
                        .globals
                        .get(0)
                        .and_then(|g| match g.value {
                            Value::I32(v) => Some(v),
                            _ => None,
                        })
                        .unwrap_or(-1);
                    err.message = format!(
                        "{} (func={} pc={} addr=0x{addr:08x} offset={} sp_global={} stack_len={})",
                        err.message,
                        func_index,
                        pc,
                        offset,
                        sp_snapshot,
                        stack.len()
                    );
                    return Err(err);
                }
                pc += 1;
            }
            Instruction::I32AtomicLoad { offset } => {
                let addr = pop_address(stack, "i32.atomic.load")?;
                let value = self.load_i32(addr, *offset)?;
                stack.push(Value::I32(value));
                pc += 1;
            }
            Instruction::I64AtomicLoad { offset } => {
                let addr = pop_address(stack, "i64.atomic.load")?;
                let value = self.load_i64(addr, *offset)?;
                stack.push(Value::I64(value));
                pc += 1;
            }
            Instruction::I32AtomicStore { offset } => {
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.store".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.store")?;
                self.store_i32(addr, *offset, value)?;
                pc += 1;
            }
            Instruction::I64AtomicStore { offset } => {
                let value = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.store".into(),
                })?;
                let i64_value = value.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.store")?;
                self.store_i64(addr, *offset, i64_value)?;
                pc += 1;
            }
            Instruction::I32AtomicRmwAdd { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.add".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.add")?;
                let previous = self.load_i32(addr, *offset)?;
                let new_value = previous.wrapping_add(operand);
                self.store_i32(addr, *offset, new_value)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwAdd { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.add".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.add")?;
                let previous = self.load_i64(addr, *offset)?;
                let new_value = previous.wrapping_add(operand);
                self.store_i64(addr, *offset, new_value)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwSub { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.sub".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.sub")?;
                let previous = self.load_i32(addr, *offset)?;
                let new_value = previous.wrapping_sub(operand);
                self.store_i32(addr, *offset, new_value)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwSub { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.sub".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.sub")?;
                let previous = self.load_i64(addr, *offset)?;
                let new_value = previous.wrapping_sub(operand);
                self.store_i64(addr, *offset, new_value)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwAnd { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.and".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.and")?;
                let previous = self.load_i32(addr, *offset)?;
                self.store_i32(addr, *offset, previous & operand)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwAnd { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.and".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.and")?;
                let previous = self.load_i64(addr, *offset)?;
                self.store_i64(addr, *offset, previous & operand)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwOr { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.or".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.or")?;
                let previous = self.load_i32(addr, *offset)?;
                self.store_i32(addr, *offset, previous | operand)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwOr { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.or".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.or")?;
                let previous = self.load_i64(addr, *offset)?;
                self.store_i64(addr, *offset, previous | operand)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwXor { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.xor".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.xor")?;
                let previous = self.load_i32(addr, *offset)?;
                self.store_i32(addr, *offset, previous ^ operand)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwXor { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.xor".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.xor")?;
                let previous = self.load_i64(addr, *offset)?;
                self.store_i64(addr, *offset, previous ^ operand)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwXchg { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.xchg".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.xchg")?;
                let previous = self.load_i32(addr, *offset)?;
                self.store_i32(addr, *offset, operand)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwXchg { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.xchg".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.xchg")?;
                let previous = self.load_i64(addr, *offset)?;
                self.store_i64(addr, *offset, operand)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwCmpxchg { offset } => {
                let desired = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.cmpxchg (desired)".into(),
                    })?
                    .as_i32()?;
                let expected = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.cmpxchg (expected)"
                            .into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.cmpxchg")?;
                let previous = self.load_i32(addr, *offset)?;
                if previous == expected {
                    self.store_i32(addr, *offset, desired)?;
                }
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwCmpxchg { offset } => {
                let desired = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.cmpxchg (desired)".into(),
                })?;
                let desired = desired.as_i64()?;
                let expected = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.cmpxchg (expected)".into(),
                })?;
                let expected = expected.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.cmpxchg")?;
                let previous = self.load_i64(addr, *offset)?;
                if previous == expected {
                    self.store_i64(addr, *offset, desired)?;
                }
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwMinS { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.min_s".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.min_s")?;
                let previous = self.load_i32(addr, *offset)?;
                let new_value = previous.min(operand);
                self.store_i32(addr, *offset, new_value)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwMinS { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.min_s".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.min_s")?;
                let previous = self.load_i64(addr, *offset)?;
                let new_value = previous.min(operand);
                self.store_i64(addr, *offset, new_value)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::I32AtomicRmwMaxS { offset } => {
                let operand = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on i32.atomic.rmw.max_s".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "i32.atomic.rmw.max_s")?;
                let previous = self.load_i32(addr, *offset)?;
                let new_value = previous.max(operand);
                self.store_i32(addr, *offset, new_value)?;
                stack.push(Value::I32(previous));
                pc += 1;
            }
            Instruction::I64AtomicRmwMaxS { offset } => {
                let operand = stack.pop().ok_or_else(|| WasmExecutionError {
                    message: "value stack underflow on i64.atomic.rmw.max_s".into(),
                })?;
                let operand = operand.as_i64()?;
                let addr = pop_address(stack, "i64.atomic.rmw.max_s")?;
                let previous = self.load_i64(addr, *offset)?;
                let new_value = previous.max(operand);
                self.store_i64(addr, *offset, new_value)?;
                stack.push(Value::I64(previous));
                pc += 1;
            }
            Instruction::AtomicFence => {
                // The single-threaded interpreter does not model hardware ordering scopes.
                pc += 1;
            }
            Instruction::MemoryFill { mem } => {
                let len = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on memory.fill (len)".into(),
                    })?
                    .as_i32()?;
                let value = stack
                    .pop()
                    .ok_or_else(|| WasmExecutionError {
                        message: "value stack underflow on memory.fill (value)".into(),
                    })?
                    .as_i32()?;
                let addr = pop_address(stack, "memory.fill")?;
                if *mem != 0 {
                    return Err(WasmExecutionError {
                        message: "only default memory supported for memory.fill".into(),
                    });
                }
                if len < 0 {
                    return Err(WasmExecutionError {
                        message: "memory.fill length must be non-negative".into(),
                    });
                }
                self.fill(addr, 0, len as u32, value as u8)?;
                pc += 1;
            }
            _ => return Ok(None),
        }
        *ctx.pc = pc;
        Ok(Some(StepOutcome::Continue))
    }
}
