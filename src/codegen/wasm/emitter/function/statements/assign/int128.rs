use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn int128_signed(&self, ty: &Ty) -> Option<bool> {
        let canonical = ty.canonical_name().to_ascii_lowercase();
        match canonical.as_str() {
            "i128" | "int128" | "std::int128" | "system::int128" => Some(true),
            "u128" | "uint128" | "std::uint128" | "system::uint128" => Some(false),
            _ => None,
        }
    }

    pub(super) fn emit_int128_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        signed: bool,
    ) -> Result<(), Error> {
        let dest_access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &dest_access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let pointer_size = self.pointer_width_bits() / 8;
        let int_info_for = |emitter: &Self, name: &str| -> Option<IntInfo> {
            if let Some(info) = int_info(&emitter.layouts.primitive_registry, name, pointer_size) {
                return Some(info);
            }
            let layout = emitter.layouts.layout_for_name(name)?;
            match layout {
                TypeLayout::Enum(enum_layout) => {
                    if let Some(info) = enum_layout.underlying_info {
                        return Some(info);
                    }
                    let bits = enum_layout.size.map(|size| size.saturating_mul(8) as u16)?;
                    if bits == 0 {
                        return None;
                    }
                    Some(IntInfo {
                        bits,
                        signed: !enum_layout.is_flags,
                    })
                }
                _ => None,
            }
        };

        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                let src_access = self.resolve_memory_access(src)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.copy_int128(buf, self.temp_local, self.block_local);
                return Ok(());
            }
            Rvalue::Use(Operand::Pending(_)) => {
                self.store_int128_parts(buf, self.temp_local, 0, 0);
                return Ok(());
            }
            Rvalue::Use(Operand::Const(constant)) => {
                let (lo, hi) = self.int128_const_parts(&constant.value, signed)?;
                self.store_int128_parts(buf, self.temp_local, lo, hi);
                return Ok(());
            }
            Rvalue::Cast {
                kind: CastKind::IntToInt,
                operand,
                source,
                target,
                ..
            } => {
                let source_name = source.canonical_name();
                let target_name = target.canonical_name();
                let source_info = int_info_for(self, &source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let target_info = int_info_for(self, &target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                if target_info.bits != 128 {
                    return Err(Error::Codegen(format!(
                        "unsupported int-to-int cast to {}-bit target in WASM backend",
                        target_info.bits
                    )));
                }
                if source_info.bits == 128 {
                    self.materialize_int128_operand(
                        buf,
                        operand,
                        source_info.signed,
                        self.block_local,
                    )?;
                    self.copy_int128(buf, self.temp_local, self.block_local);
                    return Ok(());
                }
                if source_info.bits == 0 || source_info.bits > 128 {
                    return Err(Error::Codegen(format!(
                        "source integer width {} is not supported in WASM backend",
                        source_info.bits
                    )));
                }
                let source_bits = u32::from(source_info.bits);
                self.emit_numeric_operand_as(
                    buf,
                    operand,
                    ValueType::I64,
                    source_bits,
                    source_info.signed,
                )?;
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Store(0));
                if source_info.signed {
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Const(63));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    emit_instruction(buf, Op::I64Const(0));
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::I64Store(8));
                return Ok(());
            }
            Rvalue::Cast {
                kind: CastKind::FloatToInt,
                operand,
                target,
                ..
            } => {
                let target_name = target.canonical_name();
                let target_info = int_info_for(self, &target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                if target_info.bits != 128 {
                    return Err(Error::Codegen(format!(
                        "unsupported float-to-int cast to {}-bit target in WASM backend",
                        target_info.bits
                    )));
                }
                let value_ty = self.emit_operand(buf, operand)?;
                let convert_op = match value_ty {
                    ValueType::F32 => {
                        if signed {
                            Op::I64TruncF32S
                        } else {
                            Op::I64TruncF32U
                        }
                    }
                    ValueType::F64 => {
                        if signed {
                            Op::I64TruncF64S
                        } else {
                            Op::I64TruncF64U
                        }
                    }
                    other => {
                        return Err(Error::Codegen(format!(
                            "expected floating-point operand for int128 cast, found {:?}",
                            other
                        )));
                    }
                };
                emit_instruction(buf, convert_op);
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Store(0));
                if signed {
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Const(63));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    emit_instruction(buf, Op::I64Const(0));
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local_hi));
                emit_instruction(buf, Op::I64Store(8));
                return Ok(());
            }
            Rvalue::Unary { op, operand, .. } => match op {
                UnOp::UnaryPlus => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    self.copy_int128(buf, self.temp_local, self.block_local);
                    return Ok(());
                }
                UnOp::Neg if signed => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    let call = self.runtime_hook_index(RuntimeHook::I128Neg)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                UnOp::BitNot => {
                    self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Not
                    } else {
                        RuntimeHook::U128Not
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                _ => {}
            },
            Rvalue::Binary { op, lhs, rhs, .. } => match op {
                crate::mir::BinOp::Add => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Add
                    } else {
                        RuntimeHook::U128Add
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Sub => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Sub
                    } else {
                        RuntimeHook::U128Sub
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Mul => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Mul
                    } else {
                        RuntimeHook::U128Mul
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Div => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Div
                    } else {
                        RuntimeHook::U128Div
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Rem => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Rem
                    } else {
                        RuntimeHook::U128Rem
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitAnd => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128And
                    } else {
                        RuntimeHook::U128And
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitOr => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Or
                    } else {
                        RuntimeHook::U128Or
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::BitXor => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                    let hook = if signed {
                        RuntimeHook::I128Xor
                    } else {
                        RuntimeHook::U128Xor
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Shl => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    let amount_ty = self.emit_operand(buf, rhs)?;
                    Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    let hook = if signed {
                        RuntimeHook::I128Shl
                    } else {
                        RuntimeHook::U128Shl
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                crate::mir::BinOp::Shr => {
                    self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                    let amount_ty = self.emit_operand(buf, rhs)?;
                    Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    let hook = if signed {
                        RuntimeHook::I128Shr
                    } else {
                        RuntimeHook::U128Shr
                    };
                    let call = self.runtime_hook_index(hook)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::Call(call));
                    return Ok(());
                }
                _ => {}
            },
            _ => {}
        }

        Err(Error::Codegen(format!(
            "unsupported int128 assignment for rvalue {:?} in WASM backend",
            value
        )))
    }

    pub(crate) fn materialize_int128_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        signed: bool,
        scratch_local: u32,
    ) -> Result<(), Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::LocalSet(scratch_local));
                Ok(())
            }
            Operand::Const(constant) => {
                let (lo, hi) = self.int128_const_parts(&constant.value, signed)?;
                self.allocate_int128_temp(buf, lo, hi, scratch_local)
            }
            _ => Err(Error::Codegen(
                "unsupported operand for int128 lowering in WASM backend".into(),
            )),
        }
    }

    pub(crate) fn allocate_int128_temp(
        &mut self,
        buf: &mut Vec<u8>,
        lo: u64,
        hi: i64,
        target_local: u32,
    ) -> Result<(), Error> {
        let size = 16i32;
        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::I32Const(size));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::LocalTee(target_local));
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::LocalGet(target_local));
        emit_instruction(buf, Op::I64Const(lo as i64));
        emit_instruction(buf, Op::I64Store(0));
        emit_instruction(buf, Op::LocalGet(target_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Const(hi));
        emit_instruction(buf, Op::I64Store(0));
        Ok(())
    }

    pub(super) fn store_int128_parts(&self, buf: &mut Vec<u8>, dest_local: u32, lo: u64, hi: i64) {
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I64Const(lo as i64));
        emit_instruction(buf, Op::I64Store(0));
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Const(hi));
        emit_instruction(buf, Op::I64Store(0));
    }

    pub(crate) fn int128_const_parts(
        &self,
        value: &ConstValue,
        signed: bool,
    ) -> Result<(u64, i64), Error> {
        if signed {
            let raw: i128 = match value {
                ConstValue::Int(v) | ConstValue::Int32(v) => *v,
                ConstValue::UInt(v) => *v as i128,
                ConstValue::Bool(b) => i128::from(*b),
                ConstValue::Decimal(decimal) => decimal.to_encoding() as i128,
                _ => {
                    return Err(Error::Codegen(format!(
                        "unsupported int128 constant kind for WASM backend: {:?}",
                        value
                    )));
                }
            };
            let lo = raw as u128 as u64;
            let hi = (raw >> 64) as i64;
            Ok((lo, hi))
        } else {
            let raw: u128 = match value {
                ConstValue::UInt(v) => *v,
                ConstValue::Int(v) | ConstValue::Int32(v) if *v >= 0 => *v as u128,
                ConstValue::Bool(b) => u128::from(*b),
                ConstValue::Decimal(decimal) => decimal.to_encoding(),
                _ => {
                    return Err(Error::Codegen(format!(
                        "unsupported uint128 constant kind for WASM backend: {:?}",
                        value
                    )));
                }
            };
            let lo = raw as u64;
            let hi = (raw >> 64) as u64;
            Ok((lo, hi as i64))
        }
    }

    pub(super) fn copy_int128(&self, buf: &mut Vec<u8>, dest_local: u32, src_local: u32) {
        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::LocalGet(src_local));
        emit_instruction(buf, Op::I64Load(0));
        emit_instruction(buf, Op::I64Store(0));

        emit_instruction(buf, Op::LocalGet(dest_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalGet(src_local));
        emit_instruction(buf, Op::I32Const(8));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I64Load(0));
        emit_instruction(buf, Op::I64Store(0));
    }
}
