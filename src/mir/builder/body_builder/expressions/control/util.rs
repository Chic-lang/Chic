use super::*;
use crate::mir::data::{Pattern, VariantPatternFields};
use crate::mir::layout::{EnumLayout, EnumVariantLayout};

impl<'a> BodyBuilder<'a> {
    pub(crate) fn bin_op_for_assign(op: AssignOp) -> Option<BinOp> {
        match op {
            AssignOp::AddAssign => Some(BinOp::Add),
            AssignOp::SubAssign => Some(BinOp::Sub),
            AssignOp::MulAssign => Some(BinOp::Mul),
            AssignOp::DivAssign => Some(BinOp::Div),
            AssignOp::RemAssign => Some(BinOp::Rem),
            AssignOp::BitAndAssign => Some(BinOp::BitAnd),
            AssignOp::BitOrAssign => Some(BinOp::BitOr),
            AssignOp::BitXorAssign => Some(BinOp::BitXor),
            AssignOp::ShlAssign => Some(BinOp::Shl),
            AssignOp::ShrAssign => Some(BinOp::Shr),
            AssignOp::Assign => None,
            AssignOp::NullCoalesceAssign => None,
        }
    }

    pub(crate) fn mmio_operand_for_place(&self, place: &Place) -> Option<MmioOperand> {
        if place.projection.len() != 1 {
            return None;
        }

        let base_decl = self.locals.get(place.local.0)?;
        let base_name = self.resolve_ty_name(&base_decl.ty)?;
        let layout = self.lookup_struct_layout_by_name(&base_name)?;
        let mmio_struct = layout.mmio.as_ref()?;

        let field_layout = match &place.projection[0] {
            ProjectionElem::Field(index) => layout.fields.iter().find(|f| f.index == *index)?,
            ProjectionElem::FieldNamed(name) => layout.fields.iter().find(|f| f.name == *name)?,
            _ => return None,
        };

        let field_mmio = field_layout.mmio.as_ref()?;

        Some(MmioOperand {
            base_address: mmio_struct.base_address,
            offset: field_mmio.offset,
            width_bits: field_mmio.width_bits,
            access: field_mmio.access,
            endianness: mmio_struct.endianness,
            address_space: crate::mmio::AddressSpaceId::from_optional(
                mmio_struct.address_space.as_deref(),
            ),
            requires_unsafe: mmio_struct.requires_unsafe,
            ty: field_layout.ty.clone(),
            name: Some(field_layout.name.clone()),
        })
    }

    pub(crate) fn is_self_operand(&self, operand: &Operand) -> bool {
        let Some(self_local) = self.lookup_name("self") else {
            return false;
        };
        match operand {
            Operand::Copy(place) | Operand::Move(place) => place.local == self_local,
            Operand::Borrow(borrow) => borrow.place.local == self_local,
            _ => false,
        }
    }

    pub(crate) fn validate_mmio_access(
        &mut self,
        spec: &MmioOperand,
        intent: MmioIntent,
        span: Option<Span>,
    ) -> bool {
        let register_name = spec
            .name
            .clone()
            .unwrap_or_else(|| format!("0x{:08X}", spec.offset));

        if spec.requires_unsafe && self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "MMIO register `{register_name}` requires an `unsafe` block to access"
                ),
                span,
            });
        }

        if matches!(intent, MmioIntent::Read | MmioIntent::ReadWrite)
            && matches!(spec.access, MmioAccess::WriteOnly)
        {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "MMIO register `{register_name}` is write-only and cannot be read"
                ),
                span,
            });
        }

        if matches!(intent, MmioIntent::Write | MmioIntent::ReadWrite)
            && matches!(spec.access, MmioAccess::ReadOnly)
        {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("MMIO register `{register_name}` is read-only"),
                span,
            });
            return false;
        }

        true
    }

    pub(crate) fn find_enum_layout(&self, type_name: &str) -> Option<EnumLayout> {
        if let Some(layout) = self.lookup_enum_layout(type_name) {
            return Some(layout.clone());
        }
        if let Some(index) = type_name.find('<') {
            let base = type_name[..index].trim_end();
            if let Some(layout) = self.lookup_enum_layout(base) {
                return Some(layout.clone());
            }
        }
        None
    }

    pub(crate) fn find_result_variant(
        layout: &EnumLayout,
        candidates: &[&str],
    ) -> Option<EnumVariantLayout> {
        layout
            .variants
            .iter()
            .find(|variant| {
                candidates
                    .iter()
                    .any(|name| variant.name.eq_ignore_ascii_case(name))
            })
            .cloned()
    }

    pub(crate) fn result_variant_pattern(
        layout: &EnumLayout,
        variant: &EnumVariantLayout,
    ) -> Pattern {
        let path = layout
            .name
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>();
        let fields = if variant.fields.is_empty() {
            VariantPatternFields::Unit
        } else {
            VariantPatternFields::Tuple(vec![Pattern::Wildcard; variant.fields.len()])
        };
        Pattern::Enum {
            path,
            variant: variant.name.clone(),
            fields,
        }
    }

    pub(crate) fn type_names_equivalent(a: &Ty, b: &Ty) -> bool {
        a.canonical_name() == b.canonical_name()
    }

    pub(crate) fn convert_error_operand(
        &mut self,
        operand: Operand,
        source_ty: &Ty,
        target_ty: &Ty,
        span: Option<Span>,
    ) -> Option<(Operand, Option<LocalId>)> {
        if Self::type_names_equivalent(source_ty, target_ty) {
            return Some((operand, None));
        }

        let target_name = self
            .resolve_ty_name(target_ty)
            .unwrap_or_else(|| target_ty.canonical_name());
        let source_name = self
            .resolve_ty_name(source_ty)
            .unwrap_or_else(|| source_ty.canonical_name());
        let function_name = format!("{target_name}::from");

        let candidates = self
            .symbol_index
            .resolve_function(self.namespace.as_deref(), &function_name);
        let selected = candidates.iter().find(|symbol| {
            symbol.signature.params.len() == 1
                && Self::type_names_equivalent(&symbol.signature.params[0], source_ty)
                && Self::type_names_equivalent(&symbol.signature.ret, target_ty)
        });

        let Some(symbol) = selected else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "cannot convert error type `{source_name}` to `{target_name}`; implement `{target_name}::from({source_name})`"
                ),
                span,
            });
            return None;
        };

        let mut arg_operand = operand.clone();
        if let Operand::Copy(place) = arg_operand {
            arg_operand = Operand::Move(place);
        }

        let is_nullable = matches!(target_ty, Ty::Nullable(_));
        let result = self.emit_property_call(
            &symbol.qualified,
            vec![arg_operand],
            Some((target_ty.clone(), is_nullable)),
            span,
        );

        let Some(result_operand) = result else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("failed to lower `{target_name}::from` while propagating error"),
                span,
            });
            return None;
        };

        let temp_local = match &result_operand {
            Operand::Copy(place) | Operand::Move(place) => Some(place.local),
            Operand::Borrow(borrow) => Some(borrow.place.local),
            Operand::Const(_) | Operand::Mmio(_) | Operand::Pending(_) => None,
        };

        Some((result_operand, temp_local))
    }
}
