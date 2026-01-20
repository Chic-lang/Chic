use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_decimal_constant_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Decimal128,
    ) -> Result<(), Error> {
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        let parts = value.to_bits();
        for (index, part) in parts.iter().enumerate() {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            let offset = (index * 4) as i32;
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::I32Const(*part as i32));
            emit_instruction(buf, Op::I32Store(0));
        }
        Ok(())
    }

    pub(super) fn emit_decimal_intrinsic_assign(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        decimal: &DecimalIntrinsic,
    ) -> Result<(), Error> {
        let result_access = self.resolve_memory_access(place)?;
        // stash vector hint for reuse
        let vector_ty = self.emit_decimal_enum_operand(buf, &decimal.vectorize)?;
        if !matches!(vector_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "decimal vectorize operand must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::Drop);

        self.emit_decimal_runtime_call(buf, &result_access, decimal, false)?;
        self.emit_decimal_runtime_call(buf, &result_access, decimal, true)?;

        self.store_decimal_intrinsic_variant(buf, &result_access)?;
        Ok(())
    }

    pub(super) fn emit_decimal_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        result_access: &MemoryAccess,
        decimal: &DecimalIntrinsic,
        vectorized: bool,
    ) -> Result<(), Error> {
        let base_hook = match decimal.kind {
            DecimalIntrinsicKind::Add => RuntimeHook::DecimalAdd,
            DecimalIntrinsicKind::Sub => RuntimeHook::DecimalSub,
            DecimalIntrinsicKind::Mul => RuntimeHook::DecimalMul,
            DecimalIntrinsicKind::Div => RuntimeHook::DecimalDiv,
            DecimalIntrinsicKind::Rem => RuntimeHook::DecimalRem,
            DecimalIntrinsicKind::Fma => RuntimeHook::DecimalFma,
        };
        let hook = if vectorized {
            match base_hook {
                RuntimeHook::DecimalAdd => RuntimeHook::DecimalAddSimd,
                RuntimeHook::DecimalSub => RuntimeHook::DecimalSubSimd,
                RuntimeHook::DecimalMul => RuntimeHook::DecimalMulSimd,
                RuntimeHook::DecimalDiv => RuntimeHook::DecimalDivSimd,
                RuntimeHook::DecimalRem => RuntimeHook::DecimalRemSimd,
                RuntimeHook::DecimalFma => RuntimeHook::DecimalFmaSimd,
                other => other,
            }
        } else {
            base_hook
        };

        self.emit_pointer_expression(buf, result_access)?;
        self.emit_decimal_operand_pointer(buf, &decimal.lhs)?;
        self.emit_decimal_operand_pointer(buf, &decimal.rhs)?;
        if decimal.kind == DecimalIntrinsicKind::Fma {
            let addend = decimal.addend.as_ref().ok_or_else(|| {
                Error::Codegen("decimal intrinsic `Fma` missing addend operand".into())
            })?;
            self.emit_decimal_operand_pointer(buf, addend)?;
        }

        let rounding_ty = self.emit_decimal_enum_operand(buf, &decimal.rounding)?;
        if !matches!(rounding_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "decimal rounding operand must lower to i32 in WASM backend".into(),
            ));
        }
        let flags = if vectorized {
            DECIMAL_FLAG_VECTORIZE
        } else {
            0
        };
        emit_instruction(buf, Op::I32Const(flags as i32));
        let hook_index = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::Call(hook_index));
        Ok(())
    }

    pub(super) fn emit_decimal_operand_pointer(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<(), Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(())
            }
            Operand::Const(constant) => {
                if let ConstValue::Decimal(value) = &constant.value {
                    self.allocate_decimal_temp(buf, value, self.stack_temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    Ok(())
                } else {
                    Err(Error::Codegen(
                        "decimal intrinsic operands must be addressable in the WASM backend".into(),
                    ))
                }
            }
            other => Err(Error::Codegen(format!(
                "decimal intrinsic operand {other:?} is not supported in WASM backend"
            ))),
        }
    }

    pub(super) fn store_decimal_intrinsic_variant(
        &mut self,
        buf: &mut Vec<u8>,
        result_access: &MemoryAccess,
    ) -> Result<(), Error> {
        let ty = Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult");
        let layout = self.lookup_struct_layout(&ty).ok_or_else(|| {
            Error::Codegen("missing `DecimalIntrinsicResult` layout for WASM backend".into())
        })?;
        let field = layout
            .fields
            .iter()
            .find(|field| field.name == "Variant")
            .ok_or_else(|| {
                Error::Codegen("`DecimalIntrinsicResult` layout missing Variant field".into())
            })?;
        let offset = field.offset.ok_or_else(|| {
            Error::Codegen("Variant field missing offset for WASM lowering".into())
        })?;

        self.emit_pointer_expression(buf, result_access)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        if offset != 0 {
            emit_instruction(buf, Op::I32Const(offset as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::I32Store(0));
        Ok(())
    }

    pub(super) fn emit_decimal_enum_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        match operand {
            Operand::Const(constant) => {
                if let ConstValue::Enum { discriminant, .. } = &constant.value {
                    let value = i32::try_from(*discriminant).map_err(|_| {
                        Error::Codegen(
                            "enum discriminant exceeds 32-bit range in WASM backend".into(),
                        )
                    })?;
                    emit_instruction(buf, Op::I32Const(value));
                    Ok(ValueType::I32)
                } else {
                    Err(Error::Codegen(format!(
                        "unsupported constant operand {:?} for decimal enum",
                        constant.value
                    )))
                }
            }
            _ => self.emit_operand(buf, operand),
        }
    }

    pub(super) fn allocate_decimal_temp(
        &mut self,
        buf: &mut Vec<u8>,
        value: &Decimal128,
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

        for (index, part) in value.to_bits().iter().enumerate() {
            emit_instruction(buf, Op::LocalGet(target_local));
            let offset = (index * 4) as i32;
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::I32Const(*part as i32));
            emit_instruction(buf, Op::I32Store(0));
        }

        Ok(())
    }
}
