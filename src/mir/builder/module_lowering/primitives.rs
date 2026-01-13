use super::driver::{LoweringDiagnostic, ModuleLowering};
use crate::frontend::ast::Attribute;
use crate::primitives::{
    PrimitiveAttributeError, PrimitiveDescriptor, PrimitiveRegistrationError, normalize_name,
    parse_primitive_attribute,
};

impl ModuleLowering {
    pub(super) fn register_primitive_attribute(
        &mut self,
        qualified_name: &str,
        attributes: &[Attribute],
    ) {
        for attr in attributes {
            if !attr.name.eq_ignore_ascii_case("primitive") {
                continue;
            }
            let (maybe_desc, errors) = parse_primitive_attribute(attr, qualified_name);
            self.diagnostics.extend(diagnostics_from_attribute(errors));
            let Some(mut desc) = maybe_desc else {
                continue;
            };
            desc.std_wrapper_type
                .get_or_insert_with(|| qualified_name.to_string());
            desc.aliases.push(qualified_name.to_string());
            let dotted = qualified_name.replace("::", ".");
            if dotted != qualified_name {
                desc.aliases.push(dotted);
            }
            self.register_primitive_descriptor(desc);
        }
    }

    pub(crate) fn push_primitive_error(&mut self, err: PrimitiveRegistrationError) {
        self.diagnostics.push(LoweringDiagnostic {
            message: err.message,
            span: err.span.or(err.conflicting_span),
        });
    }

    pub(super) fn register_primitive_descriptor(&mut self, desc: PrimitiveDescriptor) {
        if let Some(primary) = normalize_name(&desc.primitive_name) {
            let aliases = Self::normalized_aliases(&desc, &primary);
            if self.prune_conflicting_extras(&aliases, desc.span) {
                self.rebuild_primitive_registries();
            }
        }
        if let Ok(registered) = self.register_descriptor_pair(desc) {
            self.registered_primitives.push(registered);
        }
    }
}

fn diagnostics_from_attribute(errors: Vec<PrimitiveAttributeError>) -> Vec<LoweringDiagnostic> {
    errors
        .into_iter()
        .map(|err| LoweringDiagnostic {
            message: err.message,
            span: err.span,
        })
        .collect()
}
