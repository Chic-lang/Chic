use super::super::*;
use crate::frontend::metadata::reflection::TypeKind;

body_builder_impl! {
    pub(crate) fn validate_assignment_target(&mut self, place: &Place, span: Option<Span>) -> bool {
        if let Some((owner_name, field_layout)) = self.readonly_field_for_place(place) {
            if field_layout.is_readonly {
                let in_owner_ctor = self.function_kind == FunctionKind::Constructor
                    && self.is_self_place(place)
                    && self
                        .current_self_type_name()
                        .as_deref()
                        .map(|ty| ty == owner_name)
                        .unwrap_or(false);
                let is_record_temp_assignment = self
                    .symbol_index
                    .reflection_descriptor(&owner_name)
                    .map(|descriptor| matches!(descriptor.kind, TypeKind::Record))
                    .unwrap_or(false)
                    && self.is_temp_place(place);
                if !in_owner_ctor && !is_record_temp_assignment {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "readonly field `{owner_name}::{}` can only be assigned within its constructors",
                            field_layout.name
                        ),
                        span,
                    });
                    return false;
                }
            }
        }

        let Some(owner_name) = self.place_owner_type_name(place) else {
            return true;
        };
        if self.symbol_index.is_readonly_struct(&owner_name) {
            if self.function_kind == FunctionKind::Constructor && self.is_self_place(place) {
                return true;
            }
            let is_record_temp_assignment = self
                .symbol_index
                .reflection_descriptor(&owner_name)
                .map(|descriptor| matches!(descriptor.kind, TypeKind::Record))
                .unwrap_or(false)
                && self.is_temp_place(place);
            if !is_record_temp_assignment {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "readonly struct `{owner_name}` fields can only be assigned within its constructors"
                    ),
                    span,
                });
                return false;
            }
        }
        true
    }

    pub(crate) fn readonly_field_for_place(&self, place: &Place) -> Option<(String, &FieldLayout)> {
        let selector = place.projection.last()?;
        match selector {
            ProjectionElem::Field(_) | ProjectionElem::FieldNamed(_) => {}
            _ => return None,
        }
        let owner_name = self.place_owner_type_name(place)?;
        let layout = self
            .type_layouts
            .types
            .get(&owner_name)
            .and_then(|layout| match layout {
                TypeLayout::Struct(layout) | TypeLayout::Class(layout) => Some(layout),
                _ => None,
            })?;
        let field_layout = match selector {
            ProjectionElem::Field(index) => layout.fields.iter().find(|field| field.index == *index),
            ProjectionElem::FieldNamed(name) => layout
                .fields
                .iter()
                .find(|field| field.matches_name(name)),
            _ => None,
        }?;
        Some((owner_name, field_layout))
    }

    pub(crate) fn is_self_place(&self, place: &Place) -> bool {
        self.locals
            .get(place.local.0)
            .map(|decl| {
                matches!(decl.kind, LocalKind::Arg(_))
                    && decl
                        .name
                        .as_deref()
                        .map(|name| name.eq_ignore_ascii_case("self") || name.eq_ignore_ascii_case("this"))
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn is_temp_place(&self, place: &Place) -> bool {
        self.locals
            .get(place.local.0)
            .map(|decl| matches!(decl.kind, LocalKind::Temp))
            .unwrap_or(false)
    }
}
