use super::*;

body_builder_impl! {
        pub(crate) fn lower_identifier_expr(&mut self, name: &str, span: Option<Span>) -> Option<Operand> {
            if let Some(id) = self.lookup_name(name) {
                let mut place = Place::new(id);
                self.normalise_place(&mut place);
                return Some(Operand::Copy(place));
            }

        if let Some(index) = self.lookup_local_function_entry(name) {
            return self.instantiate_local_function(index, span);
        }

        if let Some(value) = self.lookup_const(name) {
            let value = self.normalise_const(value, span);
            return Some(Operand::Const(ConstOperand::new(value)));
        }

        if let Some(self_operand) = self.make_self_operand(span) {
            if let Some((_, property)) = self.property_symbol_from_operand(&self_operand, name) {
                if !property.is_static {
                    if let Some(result) = self.lower_property_member(&self_operand, name, span) {
                        return Some(result);
                    }
                }
            }
        }

        if let Some(place) = self.resolve_self_field_place(name) {
            if let Some(target) = self.mmio_operand_for_place(&place) {
                self.validate_mmio_access(&target, MmioIntent::Read, span);
                return Some(Operand::Mmio(target));
            }
            return Some(Operand::Copy(place));
        }

            if let Some(owner) = self.current_self_type_name() {
                if let Some(field) = self.symbol_index.field_symbol(&owner, name)
                    && field.is_static
                {
                    return self.lower_static_field_value(&owner, name, field, span);
                }

                if let Some(property) = self.symbol_index.property(&owner, name)
                    && property.is_static
                {
                    return self.lower_static_property_value(&owner, None, name, property, span);
                }

                if let Some(symbol) = self.symbol_index.type_const(&owner, name) {
                    if let Some(value) = self.const_symbol_value(symbol, span) {
                        let value = self.normalise_const(value, span);
                        return Some(Operand::Const(ConstOperand::new(value)));
                    }
                    return None;
                }
            }

        if let Some(operand) = self.lower_static_using_identifier(name, span) {
            return Some(operand);
        }

        for ty in self.symbol_index.types() {
            if let Some(field) = self.symbol_index.field_symbol(ty, name)
                && field.is_static
            {
                return self.lower_static_field_value(ty, name, field, span);
            }
            if let Some(property) = self.symbol_index.property(ty, name)
                && property.is_static
            {
                return self.lower_static_property_value(ty, None, name, property, span);
            }
        }

            if let Some(symbol) = self
                .symbol_index
                .namespace_const(self.namespace.as_deref(), name)
            {
            if let Some(value) = self.const_symbol_value(symbol, span) {
                let value = self.normalise_const(value, span);
                return Some(Operand::Const(ConstOperand::new(value)));
            }
            return None;
        }

        if let Some(operand) = self.lower_namespace_static_identifier(name, span) {
            return Some(operand);
        }

        if let Some(function_operand) = self.resolve_function_operand(name, span) {
            return Some(function_operand);
        }

        if Self::identifier_looks_like_type(name) {
            return Some(Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: name.to_string(),
                span,
                info: None,
            }));
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: format!("unknown identifier `{name}` in expression"),
            span,
        });
        Some(Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: name.to_string(),
            span,
            info: None,
        }))
    }
    pub(crate) fn resolve_function_operand(&mut self, name: &str, span: Option<Span>) -> Option<Operand> {
        let namespace = self.namespace.as_deref();
        let matches = self.symbol_index.resolve_function(namespace, name);
        if matches.is_empty() {
            return None;
        }
        if matches.len() == 1 {
            let symbol = matches[0];
            return Some(Operand::Const(ConstOperand::new(ConstValue::Symbol(
                symbol.internal_name.clone(),
            ))));
        }

        let candidates = matches
            .iter()
            .map(|symbol| PendingFunctionCandidate {
                qualified: symbol.qualified.clone(),
                signature: symbol.signature.clone(),
                is_static: symbol.is_static,
            })
            .collect::<Vec<_>>();
        Some(Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: name.to_string(),
            span,
            info: Some(Box::new(PendingOperandInfo::FunctionGroup {
                path: name.to_string(),
                candidates,
                receiver: None,
            })),
        }))
    }
    pub(crate) fn member_chain_unresolved(&self, expr: &ExprNode) -> bool {
        match expr {
            ExprNode::Identifier(name) => {
                if self.lookup_name(name).is_some() {
                    return false;
                }
                if self.self_has_instance_member(name) {
                    return false;
                }
                self.primitive_registry.descriptor_for_name(name).is_some()
                    || Self::identifier_looks_like_type(name)
            }
            ExprNode::Member { base, .. } => self.member_chain_unresolved(base.as_ref()),
            ExprNode::Parenthesized(inner) => self.member_chain_unresolved(inner.as_ref()),
            _ => false,
        }
    }
    fn self_has_instance_member(&self, name: &str) -> bool {
        let Some(owner) = self.current_self_type_name() else {
            return false;
        };
        if self
            .symbol_index
            .field_symbol(&owner, name)
            .is_some_and(|field| !field.is_static)
        {
            return true;
        }
        if self
            .symbol_index
            .property(&owner, name)
            .is_some_and(|property| !property.is_static)
        {
            return true;
        }
        if let Some(layout) = self.lookup_struct_layout_by_name(&owner) {
            if layout.fields.iter().any(|field| field.matches_name(name)) {
                return true;
            }
        }
        false
    }
    pub(crate) fn identifier_looks_like_type(name: &str) -> bool {
        let trimmed = name.trim_start_matches('_');
        trimmed.chars().next().is_some_and(char::is_uppercase)
    }
}
