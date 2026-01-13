use crate::frontend::ast::items::{
    ExtensionDecl, ExtensionMember, ImplDecl, ImplMember, Visibility,
};
use crate::frontend::metadata::reflection::query::helpers::{
    attribute_descriptors, generic_handles, namespace_for_scope, qualify, type_handle,
};
use crate::frontend::metadata::reflection::query::members::{
    append_const_members, associated_type_member, method_member,
};
use crate::frontend::metadata::{MemberKind, TypeDescriptor, TypeKind, VisibilityDescriptor};

pub(super) fn impl_descriptor(decl: &ImplDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let name = if let Some(trait_ref) = &decl.trait_ref {
        format!(
            "{} for {}",
            trait_ref.name.clone(),
            qualify(scope, &decl.target.name)
        )
    } else {
        qualify(scope, &decl.target.name)
    };
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&name);
    let generics = generic_handles(decl.generics.as_ref());

    let mut descriptor = TypeDescriptor {
        namespace,
        name: name.clone(),
        full_name: name,
        type_id: owner.type_id,
        kind: TypeKind::Impl,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generics.is_empty(),
        generic_arguments: generics,
        bases: Vec::new(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for member in &decl.members {
        match member {
            ImplMember::Method(method) if method.visibility == Visibility::Public => {
                descriptor
                    .members
                    .push(method_member(method, MemberKind::Method, &owner));
            }
            ImplMember::AssociatedType(assoc) => {
                descriptor
                    .members
                    .push(associated_type_member(assoc, &owner));
            }
            ImplMember::Const(const_member) => {
                append_const_members(&mut descriptor.members, const_member, &owner);
            }
            ImplMember::Method(_) => {}
        }
    }

    Some(descriptor)
}

pub(super) fn extension_descriptor(
    decl: &ExtensionDecl,
    scope: &[String],
) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let name = qualify(scope, &format!("Extension<{}>", decl.target.name));
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&name);
    let generics = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: name.clone(),
        full_name: name,
        type_id: owner.type_id,
        kind: TypeKind::Extension,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generics.is_empty(),
        generic_arguments: generics,
        bases: Vec::new(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for member in &decl.members {
        let ExtensionMember::Method(method) = member;
        if method.function.visibility == Visibility::Public {
            descriptor.members.push(method_member(
                &method.function,
                MemberKind::ExtensionMethod,
                &owner,
            ));
        }
    }

    Some(descriptor)
}
