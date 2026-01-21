use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_tuple_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        tuple_ty: &TupleTy,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                self.copy_tuple_fields(buf, place, src, tuple_ty)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn emit_aggregate_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        kind: &AggregateKind,
        fields: &[Operand],
    ) -> Result<(), Error> {
        match kind {
            AggregateKind::Tuple | AggregateKind::Array => {
                for (index, field) in fields.iter().enumerate() {
                    let mut field_place = place.clone();
                    field_place
                        .projection
                        .push(ProjectionElem::Field(index as u32));
                    let field_rvalue = Rvalue::Use(field.clone());
                    self.emit_assign(buf, &field_place, &field_rvalue)?;
                }
                Ok(())
            }
            AggregateKind::Adt { name, variant } => {
                if variant.is_some() {
                    return Err(Error::Codegen(
                        "enum variant aggregate assignment is not yet supported in WASM backend"
                            .into(),
                    ));
                }
                let Some(layout) = self.layouts.types.get(name.as_str()) else {
                    return Err(Error::Codegen(format!(
                        "missing layout for aggregate `{}` in WASM backend",
                        name
                    )));
                };
                let struct_layout = match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data,
                    other => {
                        return Err(Error::Codegen(format!(
                            "aggregate assignment only supports struct/class layouts in WASM backend; found {other:?}"
                        )));
                    }
                };
                if struct_layout.fields.len() != fields.len() {
                    return Err(Error::Codegen(format!(
                        "aggregate for `{}` provided {} fields but layout expects {}",
                        name,
                        fields.len(),
                        struct_layout.fields.len()
                    )));
                }

                for (field, value) in struct_layout.fields.iter().zip(fields.iter()) {
                    let mut field_place = place.clone();
                    field_place
                        .projection
                        .push(ProjectionElem::Field(field.index));
                    let field_rvalue = Rvalue::Use(value.clone());
                    self.emit_assign(buf, &field_place, &field_rvalue)?;
                }
                Ok(())
            }
        }
    }

    pub(super) fn emit_named_aggregate_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
        ty: &Ty,
    ) -> Result<bool, Error> {
        let Ty::Named(named) = ty else {
            return Ok(false);
        };
        if is_builtin_primitive(&self.layouts.primitive_registry, named.as_str()) {
            return Ok(false);
        }
        // Named aggregates that require memory are always copied as raw bytes in the WASM backend.
        // Field-by-field copies are both slower and can miss padding/ABI details (e.g., ValuePtr
        // handles that must roundtrip through runtime shims).
        if local_requires_memory(ty, self.layouts) {
            return Ok(false);
        }
        if env::var_os("CHIC_DEBUG_WASM_AGG").is_some() {
            let repr = self
                .representations
                .get(place.local.0)
                .copied()
                .unwrap_or(LocalRepresentation::Scalar);
            eprintln!(
                "[wasm-agg-assign] func={} local={} repr={:?} ty={} proj={:?}",
                self.function.name,
                place.local.0,
                repr,
                ty.canonical_name(),
                place.projection
            );
        }
        if matches!(
            self.representations
                .get(place.local.0)
                .copied()
                .unwrap_or(LocalRepresentation::Scalar),
            LocalRepresentation::Scalar
        ) {
            // Scalar locals for single-field structs are handled as plain values.
            return Ok(false);
        }
        if self.ty_is_reference(ty) {
            return Ok(false);
        }
        let layout = match self.lookup_struct_layout(ty).cloned() {
            Some(layout) => layout,
            None => return Ok(false),
        };
        match value {
            Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) => {
                self.copy_named_fields(buf, place, src, &layout)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn copy_named_fields(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        for field in &layout.fields {
            let mut dest_field = dest.clone();
            dest_field
                .projection
                .push(ProjectionElem::FieldNamed(field.name.clone()));
            let mut src_field = src.clone();
            src_field
                .projection
                .push(ProjectionElem::FieldNamed(field.name.clone()));
            let operand = Operand::Copy(src_field);
            let field_rvalue = Rvalue::Use(operand);
            self.emit_assign(buf, &dest_field, &field_rvalue)?;
        }
        Ok(())
    }
}
