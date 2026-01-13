use super::{
    helpers::{attribute_descriptors, generic_handles, namespace_for_scope, qualify, type_handle},
    members::{parameter_descriptor, throws_list},
};
use crate::frontend::ast::items::{FunctionDecl, Visibility};
use crate::frontend::metadata::{
    MemberDescriptor, MemberKind, MethodDescriptor, ReflectionTables, TypeDescriptor, TypeKind,
    VisibilityDescriptor,
};

pub(super) fn push_function(tables: &mut ReflectionTables, func: &FunctionDecl, scope: &[String]) {
    if func.visibility != Visibility::Public {
        return;
    }

    let full_name = qualify(scope, &func.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(func.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Function,
        visibility: VisibilityDescriptor::from(func.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: Vec::new(),
        attributes: attribute_descriptors(&func.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    let member = MemberDescriptor {
        name: func.name.clone(),
        kind: MemberKind::Method,
        visibility: VisibilityDescriptor::from(func.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&func.attributes),
        field: None,
        property: None,
        method: Some(MethodDescriptor {
            return_type: type_handle(&func.signature.return_type.name),
            parameters: func
                .signature
                .parameters
                .iter()
                .map(parameter_descriptor)
                .collect(),
            is_static: func.modifiers.iter().any(|m| m == "static"),
            is_virtual: false,
            is_override: false,
            is_abstract: false,
            is_async: func.is_async,
            throws: throws_list(func),
            extern_abi: func.extern_abi.clone(),
        }),
        constructor: None,
        children: Vec::new(),
    };

    descriptor.members.push(member);

    tables.types.push(descriptor);
}
