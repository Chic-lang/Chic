use super::helpers::{namespace_for_scope, qualify, type_handle};
use crate::frontend::ast::items::{ConstItemDecl, Visibility};
use crate::frontend::metadata::{TypeDescriptor, TypeKind, VisibilityDescriptor};

pub(super) fn const_descriptors(decl: &ConstItemDecl, scope: &[String]) -> Vec<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return Vec::new();
    }

    let mut result = Vec::new();
    let namespace = namespace_for_scope(scope);
    for declarator in &decl.declaration.declarators {
        let name = qualify(scope, &declarator.name);
        let descriptor = TypeDescriptor {
            namespace: namespace.clone(),
            name: name.clone(),
            full_name: name.clone(),
            type_id: type_handle(&name).type_id,
            kind: TypeKind::Const,
            visibility: VisibilityDescriptor::from(decl.visibility),
            is_generic: false,
            generic_arguments: Vec::new(),
            bases: Vec::new(),
            attributes: Vec::new(),
            underlying_type: Some(type_handle(&decl.declaration.ty.name)),
            members: Vec::new(),
            layout: None,
            layout_hints: None,
            readonly: true,
        };
        result.push(descriptor);
    }
    result
}
