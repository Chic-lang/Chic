use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn store_call_result(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<(), Error> {
        let representation = self
            .representations
            .get(place.local.0)
            .copied()
            .unwrap_or(LocalRepresentation::Scalar);
        if place.projection.is_empty() && matches!(representation, LocalRepresentation::Scalar) {
            if let Some(index) = self.local_index(place.local) {
                emit_instruction(buf, Op::LocalSet(index));
            } else {
                emit_instruction(buf, Op::Drop);
            }
            return Ok(());
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        let access = self.resolve_memory_access(place)?;
        if access.load_pointer_from_slot && place.projection.is_empty() {
            emit_instruction(buf, Op::LocalGet(access.pointer_local));
            if access.offset != 0 {
                emit_instruction(buf, Op::I32Const(access.offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            let value_ty = map_type(&access.value_ty);
            self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
            return Ok(());
        }
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        let value_ty = map_type(&access.value_ty);
        self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
        Ok(())
    }

    pub(super) fn store_multi_call_result(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        results: &[ValueType],
    ) -> Result<(), Error> {
        if results.len() != 2 || results.iter().any(|ty| *ty != ValueType::I32) {
            return Err(Error::Codegen(
                "multi-value call results are only supported for i32 pointer/length pairs".into(),
            ));
        }
        let dest_ty = self.mir_place_ty(place)?;
        if matches!(dest_ty, Ty::Str) {
            // `str` is a packed scalar in wasm locals/values (len << 32 | ptr). Pack the
            // `(ptr, len)` multivalue return into the scalar representation before storing.
            emit_instruction(buf, Op::LocalSet(self.temp_local)); // len
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local)); // ptr

            emit_instruction(buf, Op::LocalGet(self.temp_local));
            emit_instruction(buf, Op::I64ExtendI32U);
            emit_instruction(buf, Op::I64Const(32));
            emit_instruction(buf, Op::I64Shl);
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I64ExtendI32U);
            emit_instruction(buf, Op::I64Or);

            self.store_value_into_place(buf, place, ValueType::I64)?;
            return Ok(());
        }
        let (ptr_offset, len_offset) = self.ptr_len_field_offsets(&dest_ty)?;

        emit_instruction(buf, Op::LocalSet(self.temp_local));
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Store(ptr_offset));

        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Store(len_offset));

        Ok(())
    }
}
