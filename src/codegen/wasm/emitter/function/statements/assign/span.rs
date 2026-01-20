use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_span_stack_alloc(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        element: &Ty,
        length: &Operand,
        source: Option<&Operand>,
    ) -> Result<(), Error> {
        if !place.projection.is_empty() {
            return Err(Error::Codegen(
                "span stack allocation requires assigning to a local binding".into(),
            ));
        }

        let (elem_size, elem_align) =
            self.layouts.size_and_align_for_ty(element).ok_or_else(|| {
                Error::Codegen(format!(
                    "span stack allocation element `{}` is not sized",
                    element.canonical_name()
                ))
            })?;
        let elem_size_i32 = i32::try_from(elem_size).map_err(|_| {
            Error::Codegen(format!(
                "span element `{}` size `{elem_size}` exceeds wasm i32 range",
                element.canonical_name()
            ))
        })?;
        let elem_align_i32 = i32::try_from(elem_align).map_err(|_| {
            Error::Codegen(format!(
                "span element `{}` alignment `{elem_align}` exceeds wasm i32 range",
                element.canonical_name()
            ))
        })?;
        let offsets = self.span_ptr_offsets(false)?;

        let dest_ptr = self.pointer_local_index(place.local)?;
        let len_ty = self.emit_operand(buf, length)?;
        Self::ensure_operand_type(len_ty, ValueType::I32, "span stack allocation length")?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        if elem_size == 0 {
            emit_instruction(buf, Op::I32Const(0));
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        } else {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            if elem_size != 1 {
                emit_instruction(buf, Op::I32Const(elem_size_i32));
                emit_instruction(buf, Op::I32Mul);
            }
            if elem_align > 1 {
                emit_instruction(buf, Op::I32Const(elem_align_i32 - 1));
                emit_instruction(buf, Op::I32Add);
                emit_instruction(buf, Op::I32Const(elem_align_i32));
                emit_instruction(buf, Op::I32DivS);
                emit_instruction(buf, Op::I32Const(elem_align_i32));
                emit_instruction(buf, Op::I32Mul);
            }
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
            emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I32Add);
            emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
            emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I32Sub);
            emit_instruction(buf, Op::LocalTee(self.stack_temp_local));
            emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
        }

        if elem_size == 0 {
            emit_instruction(buf, Op::I32Const(1));
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        }

        // ValueMutPtr.Data.Pointer
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Store(0));

        // ValueMutPtr.Data.Size
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_size != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_size_i32));
        emit_instruction(buf, Op::I32Store(0));

        // ValueMutPtr.Data.Alignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.data_align != 0 {
            emit_instruction(buf, Op::I32Const(offsets.data_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_align_i32));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.Length
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.ElementSize
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_size_i32));
        emit_instruction(buf, Op::I32Store(0));

        // SpanPtr.ElementAlignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Const(elem_align_i32));
        emit_instruction(buf, Op::I32Store(0));
        if let Some(source) = source {
            self.emit_span_copy_from_source(buf, place, source)?;
        }
        Ok(())
    }

    pub(super) fn emit_span_copy_from_source(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        source: &Operand,
    ) -> Result<(), Error> {
        let source_place = match source {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => {
                return Err(Error::Codegen(
                    "span stack allocation source must be addressable in WASM backend".into(),
                ));
            }
        };
        if !source_place.projection.is_empty() {
            return Err(Error::Codegen(
                "projected span stack allocation sources are not yet supported in WASM backend"
                    .into(),
            ));
        }
        let source_ptr = self.pointer_local_index(source_place.local)?;
        let dest_ptr = self.pointer_local_index(dest.local)?;
        let source_ty = self
            .local_tys
            .get(source_place.local.0)
            .cloned()
            .map(|ty| self.resolve_self_ty(&ty))
            .unwrap_or_else(|| Ty::named("Std::Span::SpanPtr"));
        let source_readonly = matches!(source_ty, Ty::ReadOnlySpan(_));
        let source_offsets = self.span_ptr_offsets(source_readonly)?;
        let dest_offsets = self.span_ptr_offsets(false)?;

        // source pointer
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source length
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source element size
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // source element alignment
        emit_instruction(buf, Op::LocalGet(source_ptr));
        if source_offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(source_offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));

        // dest pointer
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.data_ptr != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.data_ptr as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest length
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.len != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.len as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest element size
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.elem_size != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.elem_size as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));
        // dest element alignment
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        if dest_offsets.elem_align != 0 {
            emit_instruction(buf, Op::I32Const(dest_offsets.elem_align as i32));
            emit_instruction(buf, Op::I32Add);
        }
        emit_instruction(buf, Op::I32Load(0));

        let hook = self.runtime_hook_index(RuntimeHook::SpanCopyTo)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn span_ptr_offsets(&self, readonly: bool) -> Result<SpanOffsets, Error> {
        let raw_ty = if readonly {
            Ty::named("Std::Span::ReadOnlySpanPtr")
        } else {
            Ty::named("Std::Span::SpanPtr")
        };
        let (data_field, data_offset) = self.resolve_field_by_name(&raw_ty, None, "Data")?;
        let (_ptr_field, ptr_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Pointer")?;
        let (_size_field, size_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Size")?;
        let (_align_field, align_offset) =
            self.resolve_field_by_name(&data_field.ty, None, "Alignment")?;
        let (_, len_offset) = self.resolve_field_by_name(&raw_ty, None, "Length")?;
        let (_, elem_size_offset) = self.resolve_field_by_name(&raw_ty, None, "ElementSize")?;
        let (_, elem_align_offset) =
            self.resolve_field_by_name(&raw_ty, None, "ElementAlignment")?;

        let data_ptr = data_offset
            .checked_add(ptr_offset)
            .ok_or_else(|| Error::Codegen("span data pointer offset overflowed".into()))?;
        let data_size = data_offset
            .checked_add(size_offset)
            .ok_or_else(|| Error::Codegen("span data size offset overflowed".into()))?;
        let data_align = data_offset
            .checked_add(align_offset)
            .ok_or_else(|| Error::Codegen("span data alignment offset overflowed".into()))?;

        Ok(SpanOffsets {
            data_ptr: ensure_u32(
                data_ptr,
                "span data pointer offset exceeds 32-bit range in WASM backend",
            )?,
            data_size: ensure_u32(
                data_size,
                "span data size offset exceeds 32-bit range in WASM backend",
            )?,
            data_align: ensure_u32(
                data_align,
                "span data alignment offset exceeds 32-bit range in WASM backend",
            )?,
            len: ensure_u32(
                len_offset,
                "span length offset exceeds 32-bit range in WASM backend",
            )?,
            elem_size: ensure_u32(
                elem_size_offset,
                "span element size offset exceeds 32-bit range in WASM backend",
            )?,
            elem_align: ensure_u32(
                elem_align_offset,
                "span element alignment offset exceeds 32-bit range in WASM backend",
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct SpanOffsets {
    data_ptr: u32,
    data_size: u32,
    data_align: u32,
    len: u32,
    elem_size: u32,
    elem_align: u32,
}
