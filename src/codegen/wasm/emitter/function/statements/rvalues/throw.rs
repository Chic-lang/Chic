use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn pointer_width_bits(&self) -> u32 {
        32
    }

    pub(crate) fn emit_throw(
        &mut self,
        buf: &mut Vec<u8>,
        exception: &Option<Operand>,
        ty: &Option<Ty>,
    ) -> Result<(), Error> {
        let treat_as_reference = ty
            .as_ref()
            .map(|throw_ty| self.ty_is_reference(throw_ty) || matches!(throw_ty, Ty::Named(_)))
            .unwrap_or(false);
        if std::env::var_os("CHIC_DEBUG_WASM_THROW_LOWERING").is_some() {
            eprintln!(
                "[wasm-throw-lower] func={} ty={} operand={:?}",
                self.function.name,
                ty.as_ref()
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<none>".to_string()),
                exception
            );
        }
        if let Some(value) = exception {
            let value_ty = if treat_as_reference {
                if let Operand::Copy(place) | Operand::Move(place) = value {
                    let access = self.resolve_memory_access(place)?;
                    let pointer_in_slot = access.vec_index.is_none()
                        && access.offset == 0
                        && (access.load_pointer_from_slot
                            || matches!(access.value_ty, Ty::Named(_))
                            || (self.ty_is_reference(&access.value_ty)
                                && matches!(
                                    self.representations[place.local.0],
                                    LocalRepresentation::PointerParam
                                )));
                    if std::env::var_os("CHIC_DEBUG_WASM_THROW_LOWERING").is_some() {
                        eprintln!(
                            "[wasm-throw-lower] place local={} load_slot={} offset={} vec={:?} ptr_in_slot={}",
                            place.local.0,
                            access.load_pointer_from_slot,
                            access.offset,
                            access.vec_index,
                            pointer_in_slot
                        );
                    }
                    self.emit_pointer_expression(buf, &access)?;
                    if !pointer_in_slot {
                        emit_instruction(buf, Op::I32Load(0));
                    }
                    ValueType::I32
                } else {
                    self.emit_operand(buf, value)?
                }
            } else {
                self.emit_operand(buf, value)?
            };
            match value_ty {
                ValueType::I32 => {}
                ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
                other => {
                    return Err(Error::Codegen(format!(
                        "throw operand type {:?} is not supported by the WASM backend",
                        other
                    )));
                }
            }
        } else {
            emit_instruction(buf, Op::I32Const(0));
        }

        let type_id = ty
            .as_ref()
            .map(|ty| exception_type_identity(&ty.canonical_name()))
            .unwrap_or(0);
        emit_instruction(buf, Op::I64Const(type_id as i64));
        let hook = self.runtime_hook_index(RuntimeHook::Throw)?;
        emit_instruction(buf, Op::Call(hook));
        // Treat throw as non-returning: tear down the current frame and branch to the
        // function epilogue so callers can observe the pending exception state.
        self.emit_frame_teardown(buf);
        emit_instruction(buf, Op::Br(2));
        Ok(())
    }

    pub(crate) fn const_to_op(value: &ConstValue) -> Result<Op, Error> {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => {
                i32::try_from(*v).map(Op::I32Const).map_err(|_| {
                    Error::Codegen("integer literal exceeds 32-bit range in WASM backend".into())
                })
            }
            ConstValue::UInt(v) => i32::try_from(*v).map(Op::I32Const).map_err(|_| {
                Error::Codegen(
                    "unsigned integer literal exceeds 32-bit range in WASM backend".into(),
                )
            }),
            ConstValue::Char(c) => Ok(Op::I32Const(*c as i32)),
            ConstValue::Bool(b) => Ok(Op::I32Const(i32::from(*b))),
            ConstValue::Null => Ok(Op::I32Const(0)),
            _ => Err(Error::Codegen(
                "only integral literals are supported by the WASM backend".into(),
            )),
        }
    }
}
