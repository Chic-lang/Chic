use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_int128_comparison(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
        self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
        let hook = if signed {
            RuntimeHook::I128Cmp
        } else {
            RuntimeHook::U128Cmp
        };
        let call = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::Call(call));
        match op {
            crate::mir::BinOp::Eq => {
                emit_instruction(buf, Op::I32Eqz);
            }
            crate::mir::BinOp::Ne => {
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::I32Eqz);
            }
            crate::mir::BinOp::Lt => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32LtS);
            }
            crate::mir::BinOp::Le => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32LeS);
            }
            crate::mir::BinOp::Gt => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32GtS);
            }
            crate::mir::BinOp::Ge => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32GeS);
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported int128 comparison operator in WASM backend".into(),
                ));
            }
        }
        Ok(ValueType::I32)
    }

    pub(super) fn emit_int128_unary(
        &mut self,
        buf: &mut Vec<u8>,
        op: UnOp,
        operand: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        match op {
            UnOp::UnaryPlus => return self.emit_operand(buf, operand),
            UnOp::Neg if signed => {
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let call = self.runtime_hook_index(RuntimeHook::I128Neg)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            UnOp::BitNot => {
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = if signed {
                    RuntimeHook::I128Not
                } else {
                    RuntimeHook::U128Not
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            UnOp::Increment | UnOp::Decrement => {
                let one = Operand::Const(ConstOperand::new(ConstValue::Int(1)));
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.materialize_int128_operand(buf, &one, signed, self.stack_temp_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    UnOp::Increment => {
                        if signed {
                            RuntimeHook::I128Add
                        } else {
                            RuntimeHook::U128Add
                        }
                    }
                    UnOp::Decrement => {
                        if signed {
                            RuntimeHook::I128Sub
                        } else {
                            RuntimeHook::U128Sub
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            _ => {}
        }
        Err(Error::Codegen(format!(
            "unsupported int128 unary op {:?} in WASM backend (func={})",
            op, self.function.name
        )))
    }

    pub(super) fn emit_int128_binary(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        match op {
            crate::mir::BinOp::Add
            | crate::mir::BinOp::Sub
            | crate::mir::BinOp::Mul
            | crate::mir::BinOp::Div
            | crate::mir::BinOp::Rem
            | crate::mir::BinOp::BitAnd
            | crate::mir::BinOp::BitOr
            | crate::mir::BinOp::BitXor => {
                self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    crate::mir::BinOp::Add => {
                        if signed {
                            RuntimeHook::I128Add
                        } else {
                            RuntimeHook::U128Add
                        }
                    }
                    crate::mir::BinOp::Sub => {
                        if signed {
                            RuntimeHook::I128Sub
                        } else {
                            RuntimeHook::U128Sub
                        }
                    }
                    crate::mir::BinOp::Mul => {
                        if signed {
                            RuntimeHook::I128Mul
                        } else {
                            RuntimeHook::U128Mul
                        }
                    }
                    crate::mir::BinOp::Div => {
                        if signed {
                            RuntimeHook::I128Div
                        } else {
                            RuntimeHook::U128Div
                        }
                    }
                    crate::mir::BinOp::Rem => {
                        if signed {
                            RuntimeHook::I128Rem
                        } else {
                            RuntimeHook::U128Rem
                        }
                    }
                    crate::mir::BinOp::BitAnd => {
                        if signed {
                            RuntimeHook::I128And
                        } else {
                            RuntimeHook::U128And
                        }
                    }
                    crate::mir::BinOp::BitOr => {
                        if signed {
                            RuntimeHook::I128Or
                        } else {
                            RuntimeHook::U128Or
                        }
                    }
                    crate::mir::BinOp::BitXor => {
                        if signed {
                            RuntimeHook::I128Xor
                        } else {
                            RuntimeHook::U128Xor
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            crate::mir::BinOp::Shl | crate::mir::BinOp::Shr => {
                self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                let amount_ty = self.emit_operand(buf, rhs)?;
                Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    crate::mir::BinOp::Shl => {
                        if signed {
                            RuntimeHook::I128Shl
                        } else {
                            RuntimeHook::U128Shl
                        }
                    }
                    crate::mir::BinOp::Shr => {
                        if signed {
                            RuntimeHook::I128Shr
                        } else {
                            RuntimeHook::U128Shr
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            _ => {}
        }
        Err(Error::Codegen(format!(
            "unsupported int128 binary op {:?} in WASM backend (func={})",
            op, self.function.name
        )))
    }
}
