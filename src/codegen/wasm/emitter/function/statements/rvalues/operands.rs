use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        wasm_debug!("        emit_operand {:?}", operand);
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        if let Some(ty) = self.local_tys.get(place.local.0) {
                            let base_ty = self.resolve_self_ty(ty);
                            if local_requires_memory(&base_ty, self.layouts) {
                                let access = self.resolve_memory_access(place)?;
                                self.emit_pointer_expression(buf, &access)?;
                                return Ok(ValueType::I32);
                            }
                        }
                    }
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        if let Some(ty) = self.local_tys.get(place.local.0) {
                            if matches!(
                                ty,
                                Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                            ) || self.ty_is_trait_object_like(ty)
                            {
                                if let Some(index) = self.local_index(place.local) {
                                    if std::env::var_os("CHIC_DEBUG_WASM_FN_OPERAND").is_some() {
                                        eprintln!(
                                            "[wasm-fn-op] func={} local={} wasm_index={}",
                                            self.function.name, place.local.0, index
                                        );
                                    }
                                    emit_instruction(buf, Op::LocalGet(index));
                                    return Ok(ValueType::I32);
                                }
                            }
                            if self.ty_is_reference(ty) {
                                let access = self.resolve_memory_access(place)?;
                                self.emit_pointer_expression(buf, &access)?;
                                if matches!(
                                    self.representations[place.local.0],
                                    LocalRepresentation::FrameAllocated
                                ) && !access.load_pointer_from_slot
                                {
                                    emit_instruction(buf, Op::I32Load(0));
                                }
                                return Ok(ValueType::I32);
                            }
                        }
                        return self.emit_load_from_place(buf, place);
                    }
                    if let Some(index) = self.local_index(place.local) {
                        emit_instruction(buf, Op::LocalGet(index));
                        Ok(map_type(&self.local_tys[place.local.0]))
                    } else {
                        emit_instruction(buf, Op::I32Const(0));
                        Ok(ValueType::I32)
                    }
                } else {
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(LocalRepresentation::Scalar)
                    ) {
                        let base_ty = self.resolve_self_ty(&self.local_tys[place.local.0]);
                        if matches!(base_ty, Ty::Str) && place.projection.len() == 1 {
                            if let Some(index) = self.local_index(place.local) {
                                let field = &place.projection[0];
                                let key = match field {
                                    ProjectionElem::Field(0) => Some("ptr"),
                                    ProjectionElem::Field(1) => Some("len"),
                                    ProjectionElem::FieldNamed(name) => {
                                        let lowered = name.to_ascii_lowercase();
                                        if lowered == "ptr" || lowered == "pointer" {
                                            Some("ptr")
                                        } else if lowered == "len" || lowered == "length" {
                                            Some("len")
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };

                                if let Some(key) = key {
                                    emit_instruction(buf, Op::LocalGet(index));
                                    match key {
                                        "ptr" => {
                                            emit_instruction(buf, Op::I32WrapI64);
                                        }
                                        "len" => {
                                            emit_instruction(buf, Op::I64Const(32));
                                            emit_instruction(buf, Op::I64ShrU);
                                            emit_instruction(buf, Op::I32WrapI64);
                                        }
                                        _ => {}
                                    }
                                    return Ok(ValueType::I32);
                                }
                            }
                        }
                    }
                    if matches!(
                        self.representations[place.local.0],
                        LocalRepresentation::Scalar
                    ) && !self.ty_is_reference(&self.local_tys[place.local.0])
                    {
                        let has_deref = place
                            .projection
                            .iter()
                            .any(|elem| matches!(elem, ProjectionElem::Deref));
                        if !has_deref {
                            if let Some(index) = self.local_index(place.local) {
                                let base_ty = self.resolve_self_ty(&self.local_tys[place.local.0]);
                                if let Ok(plan) =
                                    self.compute_projection_offset(&base_ty, &place.projection)
                                {
                                    if plan.vec_index.is_none() {
                                        emit_instruction(buf, Op::LocalGet(index));
                                        return Ok(map_type(&plan.value_ty));
                                    }
                                }
                            }
                        }
                    }
                    self.emit_load_from_place(buf, place)
                }
            }
            Operand::Const(constant) => self.emit_const_operand(buf, constant),
            Operand::Borrow(borrow) => {
                let access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(ValueType::I32)
            }
            Operand::Mmio(spec) => self.emit_mmio_read(buf, spec),
            Operand::Pending(PendingOperand { repr, .. }) => {
                if env::var_os("CHIC_DEBUG_PENDING").is_some() {
                    eprintln!(
                        "[pending-operand] repr=`{repr}` func={}",
                        self.function.name
                    );
                }
                emit_instruction(buf, Op::I32Const(0));
                Ok(ValueType::I32)
            }
        }
    }

    pub(crate) fn operand_ty(&self, operand: &Operand) -> Option<Ty> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let base_ty = self.local_tys.get(place.local.0)?.clone();
                Some(
                    self.compute_projection_offset(&base_ty, &place.projection)
                        .ok()?
                        .value_ty,
                )
            }
            Operand::Borrow(borrow) => {
                let base_ty = self.local_tys.get(borrow.place.local.0)?.clone();
                let ty = self
                    .compute_projection_offset(&base_ty, &borrow.place.projection)
                    .ok()?
                    .value_ty;
                let mutable = borrow.kind != BorrowKind::Shared;
                Some(Ty::Pointer(Box::new(PointerTy::new(ty, mutable))))
            }
            Operand::Mmio(spec) => Some(spec.ty.clone()),
            Operand::Const(_) | Operand::Pending(_) => None,
        }
    }
}
