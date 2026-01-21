use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_numeric_intrinsic_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
    ) -> Result<(), Error> {
        let bits = numeric_width_bits(intrinsic.width);
        let value_ty = numeric_value_ty(intrinsic.width);
        match intrinsic.kind {
            NumericIntrinsicKind::TryAdd
            | NumericIntrinsicKind::TrySub
            | NumericIntrinsicKind::TryMul
            | NumericIntrinsicKind::TryNeg => {
                self.emit_numeric_checked_arith(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::LeadingZeroCount
            | NumericIntrinsicKind::TrailingZeroCount
            | NumericIntrinsicKind::PopCount => {
                self.emit_numeric_count_intrinsic(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::RotateLeft | NumericIntrinsicKind::RotateRight => {
                self.emit_numeric_rotate_intrinsic(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::ReverseEndianness => {
                self.emit_numeric_reverse_endianness(buf, place, intrinsic, bits, value_ty)?
            }
            NumericIntrinsicKind::IsPowerOfTwo => {
                self.emit_numeric_is_power_of_two(buf, place, intrinsic, bits, value_ty)?
            }
        }
        Ok(())
    }

    pub(super) fn emit_numeric_checked_arith(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        let result_local = match value_ty {
            ValueType::I64 => self.wide_temp_local,
            _ => self.temp_local,
        };
        match intrinsic.kind {
            NumericIntrinsicKind::TryNeg => {
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[0],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                emit_instruction(buf, Op::LocalSet(result_local));
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I32Const(0)),
                    ValueType::I64 => emit_instruction(buf, Op::I64Const(0)),
                    _ => unreachable!("numeric intrinsic uses integer operands"),
                }
                emit_instruction(buf, Op::LocalGet(result_local));
                emit_instruction(buf, self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub));
                self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
                emit_instruction(buf, Op::LocalSet(result_local));
            }
            NumericIntrinsicKind::TryAdd
            | NumericIntrinsicKind::TrySub
            | NumericIntrinsicKind::TryMul => {
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[0],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                self.emit_numeric_operand_as(
                    buf,
                    &intrinsic.operands[1],
                    value_ty,
                    bits,
                    intrinsic.signed,
                )?;
                let op = match intrinsic.kind {
                    NumericIntrinsicKind::TryAdd => {
                        self.op_for_int(value_ty, Op::I32Add, Op::I64Add)
                    }
                    NumericIntrinsicKind::TrySub => {
                        self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub)
                    }
                    NumericIntrinsicKind::TryMul => {
                        self.op_for_int(value_ty, Op::I32Mul, Op::I64Mul)
                    }
                    _ => unreachable!(),
                };
                emit_instruction(buf, op);
                self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
                emit_instruction(buf, Op::LocalSet(result_local));
            }
            _ => unreachable!("checked arithmetic helper used for Try* intrinsics only"),
        }

        self.emit_numeric_overflow_flag(buf, intrinsic, bits, value_ty, result_local)?;
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        if let Some(out_place) = &intrinsic.out {
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::If);
            emit_instruction(buf, Op::LocalGet(result_local));
            self.store_value_into_place(buf, out_place, value_ty)?;
            emit_instruction(buf, Op::End);
        }

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    pub(super) fn emit_numeric_overflow_flag(
        &mut self,
        buf: &mut Vec<u8>,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
        result_local: u32,
    ) -> Result<(), Error> {
        match intrinsic.kind {
            NumericIntrinsicKind::TryAdd => {
                if intrinsic.signed {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor));
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
                    emit_instruction(
                        buf,
                        self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
                    );
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtS, Op::I64LtS));
                } else {
                    emit_instruction(buf, Op::LocalGet(result_local));
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        false,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtU, Op::I64LtU));
                }
            }
            NumericIntrinsicKind::TrySub => {
                if intrinsic.signed {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor)); // lhs ^ rhs
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        true,
                    )?;
                    emit_instruction(buf, Op::LocalGet(result_local));
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32Xor, Op::I64Xor)); // lhs ^ result
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
                    emit_instruction(
                        buf,
                        self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
                    );
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtS, Op::I64LtS));
                } else {
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[0],
                        value_ty,
                        bits,
                        false,
                    )?;
                    self.emit_numeric_operand_as(
                        buf,
                        &intrinsic.operands[1],
                        value_ty,
                        bits,
                        false,
                    )?;
                    emit_instruction(buf, self.op_for_int(value_ty, Op::I32LtU, Op::I64LtU));
                }
            }
            NumericIntrinsicKind::TryNeg => {
                let min_value = min_int_value(bits);
                self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I32Const(min_value as i32)),
                    ValueType::I64 => emit_instruction(buf, Op::I64Const(min_value)),
                    _ => unreachable!(),
                }
                emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eq, Op::I64Eq));
            }
            NumericIntrinsicKind::TryMul => {
                self.emit_numeric_mul_overflow_flag(buf, intrinsic, bits, value_ty, result_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unexpected numeric intrinsic for overflow flag".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn emit_numeric_mul_overflow_flag(
        &mut self,
        buf: &mut Vec<u8>,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
        result_local: u32,
    ) -> Result<(), Error> {
        if bits < 64 {
            self.emit_widened_operand(buf, &intrinsic.operands[0], bits, intrinsic.signed)?;
            self.emit_widened_operand(buf, &intrinsic.operands[1], bits, intrinsic.signed)?;
            emit_instruction(buf, Op::I64Mul);
            emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

            if intrinsic.signed {
                let min = min_int_value(bits);
                let max = max_int_value(bits);
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(min));
                emit_instruction(buf, Op::I64LtS);
                emit_instruction(buf, Op::LocalSet(self.block_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(max));
                emit_instruction(buf, Op::I64GtS);
                emit_instruction(buf, Op::LocalSet(self.temp_local));

                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Or);
            } else {
                let max = max_uint_value(bits);
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(max as i64));
                emit_instruction(buf, Op::I64GtU);
            }
            return Ok(());
        }

        // 64-bit path: guard divide-by-zero and MIN * -1 (signed).
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[0],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[1],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        emit_instruction(buf, Op::I32Or);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::Else);
        if intrinsic.signed {
            let min = min_int_value(bits);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Const(min));
            emit_instruction(buf, Op::I64Eq);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Const(-1));
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32And);
            emit_instruction(buf, Op::If);
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::Else);
            emit_instruction(buf, Op::LocalGet(result_local));
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64DivS);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, true)?;
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32Eqz);
            emit_instruction(buf, Op::End);
            emit_instruction(buf, Op::End);
        } else {
            emit_instruction(buf, Op::LocalGet(result_local));
            self.emit_numeric_operand_as(buf, &intrinsic.operands[1], value_ty, bits, false)?;
            emit_instruction(buf, Op::I64DivU);
            self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
            emit_instruction(buf, Op::I64Eq);
            emit_instruction(buf, Op::I32Eqz);
            emit_instruction(buf, Op::End);
        }
        Ok(())
    }

    pub(super) fn emit_numeric_count_intrinsic(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        let op = match (intrinsic.kind, value_ty) {
            (NumericIntrinsicKind::LeadingZeroCount, ValueType::I32) => Op::I32Clz,
            (NumericIntrinsicKind::LeadingZeroCount, ValueType::I64) => Op::I64Clz,
            (NumericIntrinsicKind::TrailingZeroCount, ValueType::I32) => Op::I32Ctz,
            (NumericIntrinsicKind::TrailingZeroCount, ValueType::I64) => Op::I64Ctz,
            (NumericIntrinsicKind::PopCount, ValueType::I32) => Op::I32Popcnt,
            (NumericIntrinsicKind::PopCount, ValueType::I64) => Op::I64Popcnt,
            _ => {
                return Err(Error::Codegen(format!(
                    "unsupported numeric intrinsic {:?} in WASM backend",
                    intrinsic.kind
                )));
            }
        };
        emit_instruction(buf, op);
        let type_bits = match value_ty {
            ValueType::I64 => 64,
            _ => 32,
        };
        if bits < type_bits
            && matches!(
                intrinsic.kind,
                NumericIntrinsicKind::LeadingZeroCount | NumericIntrinsicKind::TrailingZeroCount
            )
        {
            emit_instruction(buf, Op::I32Const((type_bits - bits) as i32));
            emit_instruction(buf, Op::I32Sub);
        }
        if matches!(value_ty, ValueType::I64) {
            emit_instruction(buf, Op::I32WrapI64);
        }
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    pub(super) fn emit_numeric_rotate_intrinsic(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        if value_ty == ValueType::I64 && bits != 64 {
            return Err(Error::Codegen(
                "64-bit rotate with non-64 bit width is not supported in WASM backend".into(),
            ));
        }
        if matches!(value_ty, ValueType::I32) && bits < 32 {
            self.emit_small_rotate(buf, place, intrinsic, bits)?;
            return Ok(());
        }

        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        self.emit_numeric_operand_as(buf, &intrinsic.operands[1], ValueType::I32, bits, false)?;
        let mask = (bits - 1) as i32;
        emit_instruction(buf, Op::I32Const(mask));
        emit_instruction(buf, Op::I32And);
        let op = match (intrinsic.kind, value_ty) {
            (NumericIntrinsicKind::RotateLeft, ValueType::I32) => Op::I32Rotl,
            (NumericIntrinsicKind::RotateRight, ValueType::I32) => Op::I32Rotr,
            (NumericIntrinsicKind::RotateLeft, ValueType::I64) => Op::I64Rotl,
            (NumericIntrinsicKind::RotateRight, ValueType::I64) => Op::I64Rotr,
            _ => unreachable!(),
        };
        emit_instruction(buf, op);
        self.canonicalize_int_value(buf, value_ty, bits, false);
        self.store_value_into_place(buf, place, value_ty)?;
        Ok(())
    }

    pub(super) fn emit_small_rotate(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], ValueType::I32, bits, false)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        self.emit_numeric_operand_as(buf, &intrinsic.operands[1], ValueType::I32, bits, false)?;
        emit_instruction(buf, Op::I32Const((bits - 1) as i32));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        match intrinsic.kind {
            NumericIntrinsicKind::RotateLeft => emit_instruction(buf, Op::I32Shl),
            NumericIntrinsicKind::RotateRight => emit_instruction(buf, Op::I32ShrU),
            _ => unreachable!(),
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Const(bits as i32));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Sub);
        match intrinsic.kind {
            NumericIntrinsicKind::RotateLeft => emit_instruction(buf, Op::I32ShrU),
            NumericIntrinsicKind::RotateRight => emit_instruction(buf, Op::I32Shl),
            _ => unreachable!(),
        }

        emit_instruction(buf, Op::I32Or);
        self.canonicalize_int_value(buf, ValueType::I32, bits, false);
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    pub(super) fn emit_numeric_reverse_endianness(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, &intrinsic.operands[0], value_ty, bits, false)?;
        match bits {
            8 => {}
            16 => {
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Or);
                self.canonicalize_int_value(buf, ValueType::I32, bits, false);
            }
            32 => {
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(24));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Const(0xFF));
                emit_instruction(buf, Op::I32And);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32ShrU);
                emit_instruction(buf, Op::I32Const(0xFF00));
                emit_instruction(buf, Op::I32And);
                emit_instruction(buf, Op::I32Or);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(8));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::I32Const(0x00FF0000));
                emit_instruction(buf, Op::I32And);
                emit_instruction(buf, Op::I32Or);

                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::I32Const(24));
                emit_instruction(buf, Op::I32Shl);
                emit_instruction(buf, Op::I32Or);
            }
            64 => {
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                // byte 0 -> byte 7
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(56));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF));
                emit_instruction(buf, Op::I64And);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(40));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF00));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(24));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF0000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(8));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I64Const(0xFF000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(8));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF00000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(24));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF0000000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(40));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Const(0xFF000000000000));
                emit_instruction(buf, Op::I64And);
                emit_instruction(buf, Op::I64Or);

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(56));
                emit_instruction(buf, Op::I64Shl);
                emit_instruction(buf, Op::I64Or);
            }
            _ => {
                return Err(Error::Codegen(format!(
                    "unsupported reverse endianness width {}",
                    bits
                )));
            }
        }
        self.canonicalize_int_value(buf, value_ty, bits, intrinsic.signed);
        self.store_value_into_place(buf, place, value_ty)?;
        Ok(())
    }

    pub(super) fn emit_numeric_is_power_of_two(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        intrinsic: &NumericIntrinsic,
        bits: u32,
        value_ty: ValueType,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(
            buf,
            &intrinsic.operands[0],
            value_ty,
            bits,
            intrinsic.signed,
        )?;
        let value_local = if value_ty == ValueType::I64 {
            self.wide_temp_local
        } else {
            self.temp_local
        };
        emit_instruction(buf, Op::LocalTee(value_local));

        // value != 0 (and > 0 for signed)
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        if intrinsic.signed {
            emit_instruction(buf, Op::LocalGet(value_local));
            emit_instruction(
                buf,
                self.op_for_int(value_ty, Op::I32Const(0), Op::I64Const(0)),
            );
            emit_instruction(buf, self.op_for_int(value_ty, Op::I32GtS, Op::I64GtS));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }

        emit_instruction(buf, Op::LocalGet(value_local));
        emit_instruction(
            buf,
            self.op_for_int(value_ty, Op::I32Const(1), Op::I64Const(1)),
        );
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Sub, Op::I64Sub));
        self.canonicalize_int_value(buf, value_ty, bits, false);
        emit_instruction(buf, Op::LocalGet(value_local));
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32And, Op::I64And));
        emit_instruction(buf, self.op_for_int(value_ty, Op::I32Eqz, Op::I64Eqz));

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32And);
        self.store_value_into_place(buf, place, ValueType::I32)?;
        Ok(())
    }

    pub(super) fn emit_numeric_operand_as(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        value_ty: ValueType,
        bits: u32,
        signed: bool,
    ) -> Result<(), Error> {
        let operand_ty = self.emit_operand(buf, operand)?;
        match (operand_ty, value_ty) {
            (ValueType::I32, ValueType::I64) => emit_instruction(
                buf,
                if signed {
                    Op::I64ExtendI32S
                } else {
                    Op::I64ExtendI32U
                },
            ),
            (ValueType::I64, ValueType::I32) => emit_instruction(buf, Op::I32WrapI64),
            _ => {}
        }
        self.canonicalize_int_value(buf, value_ty, bits, signed);
        Ok(())
    }

    pub(super) fn emit_widened_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        bits: u32,
        signed: bool,
    ) -> Result<(), Error> {
        self.emit_numeric_operand_as(buf, operand, ValueType::I64, bits, signed)
    }

    pub(super) fn canonicalize_int_value(
        &self,
        buf: &mut Vec<u8>,
        value_ty: ValueType,
        bits: u32,
        signed: bool,
    ) {
        if bits == 0
            || bits
                >= match value_ty {
                    ValueType::I64 => 64,
                    _ => 32,
                }
        {
            return;
        }
        match value_ty {
            ValueType::I64 => {
                if signed {
                    let shift = 64 - bits;
                    emit_instruction(buf, Op::I64Const(shift as i64));
                    emit_instruction(buf, Op::I64Shl);
                    emit_instruction(buf, Op::I64Const(shift as i64));
                    emit_instruction(buf, Op::I64ShrS);
                } else {
                    let mask = numeric_mask(bits);
                    emit_instruction(buf, Op::I64Const(mask as i64));
                    emit_instruction(buf, Op::I64And);
                }
            }
            _ => {
                if signed {
                    let shift = 32 - bits;
                    emit_instruction(buf, Op::I32Const(shift as i32));
                    emit_instruction(buf, Op::I32Shl);
                    emit_instruction(buf, Op::I32Const(shift as i32));
                    emit_instruction(buf, Op::I32ShrS);
                } else {
                    let mask = (numeric_mask(bits) & 0xFFFF_FFFF) as i32;
                    emit_instruction(buf, Op::I32Const(mask));
                    emit_instruction(buf, Op::I32And);
                }
            }
        }
    }

    pub(super) fn op_for_int(&self, value_ty: ValueType, i32_op: Op, i64_op: Op) -> Op {
        match value_ty {
            ValueType::I64 => i64_op,
            _ => i32_op,
        }
    }
}

fn numeric_width_bits(width: NumericWidth) -> u32 {
    match width {
        NumericWidth::W8 => 8,
        NumericWidth::W16 => 16,
        NumericWidth::W32 => 32,
        NumericWidth::W64 => 64,
        NumericWidth::W128 => 128,
        NumericWidth::Pointer => 32,
    }
}
fn numeric_value_ty(width: NumericWidth) -> ValueType {
    match width {
        NumericWidth::W64 => ValueType::I64,
        NumericWidth::W128 => ValueType::I64,
        NumericWidth::Pointer => ValueType::I32,
        _ => ValueType::I32,
    }
}

fn numeric_mask(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u128 << bits) as u64 - 1
    }
}

fn min_int_value(bits: u32) -> i64 {
    if bits == 64 {
        i64::MIN
    } else {
        -(1i64 << (bits - 1))
    }
}

fn max_int_value(bits: u32) -> i64 {
    if bits == 64 {
        i64::MAX
    } else {
        (1i64 << (bits - 1)) - 1
    }
}

fn max_uint_value(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        numeric_mask(bits)
    }
}
