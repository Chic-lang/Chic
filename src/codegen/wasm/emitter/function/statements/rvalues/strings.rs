use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_string_like_equality(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        lhs_ty: Option<&Ty>,
        rhs_ty: Option<&Ty>,
    ) -> Result<ValueType, Error> {
        if !matches!(op, crate::mir::BinOp::Eq | crate::mir::BinOp::Ne) {
            return Err(Error::Codegen(
                "string equality helper invoked for non-equality operator".into(),
            ));
        }

        let left_ptr_local = self.block_local;
        let right_ptr_local = self.temp_local;
        let remaining_len_local = self.scratch_local;
        let result_local = self.stack_temp_local;

        // Temporarily stores the right length while we compare lengths.
        let right_len_tmp_local = result_local;

        // compare_done:
        emit_instruction(buf, Op::Block);

        // Null handling: string equality must treat null as distinct from empty.
        // Use `block_local`/`temp_local` to store null flags, then reuse them for ptr locals.
        let left_is_null_local = left_ptr_local;
        let right_is_null_local = right_ptr_local;
        match lhs_ty {
            Some(Ty::Str) => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::LocalSet(left_is_null_local));
            }
            Some(Ty::String) => {
                let value_ty = self.emit_operand(buf, lhs)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string null check")?;
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::LocalSet(left_is_null_local));
            }
            _ => match lhs {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Null => {
                        emit_instruction(buf, Op::I32Const(1));
                        emit_instruction(buf, Op::LocalSet(left_is_null_local));
                    }
                    ConstValue::Str { .. } => {
                        emit_instruction(buf, Op::I32Const(0));
                        emit_instruction(buf, Op::LocalSet(left_is_null_local));
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported LHS operand for string equality in WASM backend".into(),
                        ));
                    }
                },
                _ => {
                    return Err(Error::Codegen(
                        "unsupported LHS operand for string equality in WASM backend".into(),
                    ));
                }
            },
        }

        match rhs_ty {
            Some(Ty::Str) => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::LocalSet(right_is_null_local));
            }
            Some(Ty::String) => {
                let value_ty = self.emit_operand(buf, rhs)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string null check")?;
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::LocalSet(right_is_null_local));
            }
            _ => match rhs {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Null => {
                        emit_instruction(buf, Op::I32Const(1));
                        emit_instruction(buf, Op::LocalSet(right_is_null_local));
                    }
                    ConstValue::Str { .. } => {
                        emit_instruction(buf, Op::I32Const(0));
                        emit_instruction(buf, Op::LocalSet(right_is_null_local));
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported RHS operand for string equality in WASM backend".into(),
                        ));
                    }
                },
                _ => {
                    return Err(Error::Codegen(
                        "unsupported RHS operand for string equality in WASM backend".into(),
                    ));
                }
            },
        }

        // If both null -> result = 1; exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_is_null_local));
        emit_instruction(buf, Op::LocalGet(right_is_null_local));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(0));
        emit_instruction(buf, Op::End);

        // If either null -> result = 0; exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_is_null_local));
        emit_instruction(buf, Op::LocalGet(right_is_null_local));
        emit_instruction(buf, Op::I32Or);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(0));
        emit_instruction(buf, Op::End);

        self.emit_string_like_slice_ptr_len(buf, lhs, lhs_ty, left_ptr_local, remaining_len_local)?;
        self.emit_string_like_slice_ptr_len(
            buf,
            rhs,
            rhs_ty,
            right_ptr_local,
            right_len_tmp_local,
        )?;

        // If lengths differ -> result = 0; exit compare_done.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::LocalGet(right_len_tmp_local));
        emit_instruction(buf, Op::I32Ne);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(1));
        emit_instruction(buf, Op::End);

        // If length == 0 -> result = 1; exit compare_done.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(1));
        emit_instruction(buf, Op::End);

        // Default to equal unless we find a mismatch.
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));

        // loop_exit:
        emit_instruction(buf, Op::Block);
        emit_instruction(buf, Op::Loop);

        // Compare current bytes; on mismatch set result=0 and exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_ptr_local));
        emit_instruction(buf, Op::I32Load8U(0));
        emit_instruction(buf, Op::LocalGet(right_ptr_local));
        emit_instruction(buf, Op::I32Load8U(0));
        emit_instruction(buf, Op::I32Ne);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(2));
        emit_instruction(buf, Op::End);

        // Advance pointers and decrement remaining length.
        emit_instruction(buf, Op::LocalGet(left_ptr_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(left_ptr_local));
        emit_instruction(buf, Op::LocalGet(right_ptr_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(right_ptr_local));

        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::LocalSet(remaining_len_local));

        // If remaining == 0 then exit loop_exit; otherwise continue loop.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::Br(2));
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::Br(0));

        // end loop / loop_exit / compare_done
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::End);

        emit_instruction(buf, Op::LocalGet(result_local));
        if matches!(op, crate::mir::BinOp::Ne) {
            emit_instruction(buf, Op::I32Eqz);
        }
        Ok(ValueType::I32)
    }

    pub(super) fn emit_string_like_slice_ptr_len(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        operand_ty: Option<&Ty>,
        out_ptr_local: u32,
        out_len_local: u32,
    ) -> Result<(), Error> {
        let ty = operand_ty
            .cloned()
            .or_else(|| self.operand_ty(operand))
            .or_else(|| match operand {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Str { .. } => Some(Ty::Str),
                    ConstValue::Null => Some(Ty::String),
                    _ => None,
                },
                _ => None,
            })
            .ok_or_else(|| {
                Error::Codegen(
                    "unable to determine operand type for string equality in WASM backend".into(),
                )
            })?;

        match ty {
            Ty::String => {
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string equality")?;
                emit_instruction(buf, Op::LocalSet(out_ptr_local));
                emit_instruction(buf, Op::LocalGet(out_ptr_local));
                let hook = self.runtime_hook_index(RuntimeHook::StringAsSlice)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::LocalSet(out_len_local));
                emit_instruction(buf, Op::LocalSet(out_ptr_local));
                Ok(())
            }
            Ty::Str => {
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I64, "str equality")?;
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::LocalSet(out_ptr_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(32));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::LocalSet(out_len_local));
                Ok(())
            }
            other => Err(Error::Codegen(format!(
                "unsupported operand type {:?} for string equality in WASM backend",
                other
            ))),
        }
    }
}
