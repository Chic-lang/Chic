use crate::frontend::ast::items::{
    BindingModifier, ClassDecl, ClassMember, DelegateDecl, EnumDecl, InterfaceDecl,
    InterfaceMember, StructDecl, TraitDecl, TraitMember, UnionDecl, UnionMember, Visibility,
};
use crate::frontend::metadata::reflection::query::helpers::{
    attribute_descriptors, generic_handles, layout_descriptor_from_hints, namespace_for_scope,
    qualify, type_handle,
};
use crate::frontend::metadata::reflection::query::members::{
    append_const_members, associated_type_member, constructor_member, field_member, method_member,
    property_member, union_field_member, union_view_member,
};
use crate::frontend::metadata::{
    MemberDescriptor, MemberKind, MethodDescriptor, ParameterDescriptor, ParameterMode,
    TypeDescriptor, TypeKind, VisibilityDescriptor,
};

pub(super) fn struct_descriptor(decl: &StructDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: if decl.is_record {
            TypeKind::Record
        } else {
            TypeKind::Struct
        },
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: Vec::new(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: decl.layout.as_ref().map(layout_descriptor_from_hints),
        readonly: decl.is_readonly,
    };

    for field in &decl.fields {
        if field.visibility == Visibility::Public {
            descriptor
                .members
                .push(field_member(field, MemberKind::Field, &owner));
        }
    }
    for property in &decl.properties {
        if property.visibility == Visibility::Public {
            descriptor.members.push(property_member(property, &owner));
        }
    }

    for konst in &decl.consts {
        append_const_members(&mut descriptor.members, konst, &owner);
    }

    for ctor in &decl.constructors {
        if ctor.visibility == Visibility::Public {
            descriptor.members.push(constructor_member(ctor, &owner));
        }
    }

    for method in &decl.methods {
        if method.visibility == Visibility::Public {
            descriptor
                .members
                .push(method_member(method, MemberKind::Method, &owner));
        }
    }

    Some(descriptor)
}

pub(super) fn delegate_descriptor(decl: &DelegateDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Delegate,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: Vec::new(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    let parameters = decl
        .signature
        .parameters
        .iter()
        .map(|param| ParameterDescriptor {
            name: param.name.clone(),
            parameter_type: type_handle(&param.ty.name),
            mode: match param.binding {
                BindingModifier::In => ParameterMode::In,
                BindingModifier::Ref => ParameterMode::Ref,
                BindingModifier::Out => ParameterMode::Out,
                BindingModifier::Value => ParameterMode::Value,
            },
            has_default: false,
            default_value: None,
            attributes: attribute_descriptors(&param.attributes),
        })
        .collect();

    let invoke_member = MemberDescriptor {
        name: "Invoke".to_string(),
        kind: MemberKind::Method,
        visibility: VisibilityDescriptor::Public,
        declaring_type: owner.clone(),
        attributes: Vec::new(),
        field: None,
        property: None,
        method: Some(MethodDescriptor {
            return_type: type_handle(&decl.signature.return_type.name),
            parameters,
            is_static: false,
            is_virtual: false,
            is_override: false,
            is_abstract: false,
            is_async: false,
            throws: Vec::new(),
            extern_abi: None,
        }),
        constructor: None,
        children: Vec::new(),
    };
    descriptor.members.push(invoke_member);

    Some(descriptor)
}

pub(super) fn union_descriptor(decl: &UnionDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Union,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
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
            UnionMember::Field(field) if field.visibility == Visibility::Public => {
                descriptor.members.push(union_field_member(field, &owner));
            }
            UnionMember::View(view) if view.visibility == Visibility::Public => {
                descriptor.members.push(union_view_member(view, &owner));
            }
            _ => {}
        }
    }

    Some(descriptor)
}

pub(super) fn enum_descriptor(decl: &EnumDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let underlying = decl
        .underlying_type
        .as_ref()
        .map(|ty| type_handle(&ty.name))
        .unwrap_or_else(|| type_handle("int"));
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Enum,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: Vec::new(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: Some(underlying),
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for variant in &decl.variants {
        let mut member = MemberDescriptor {
            name: variant.name.clone(),
            kind: MemberKind::EnumVariant,
            visibility: VisibilityDescriptor::Public,
            declaring_type: owner.clone(),
            attributes: Vec::new(),
            field: None,
            property: None,
            method: None,
            constructor: None,
            children: Vec::new(),
        };
        for field in &variant.fields {
            if field.visibility == Visibility::Public {
                member
                    .children
                    .push(field_member(field, MemberKind::Field, &owner));
            }
        }
        descriptor.members.push(member);
    }

    Some(descriptor)
}

pub(super) fn class_descriptor(decl: &ClassDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Class,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: decl
            .bases
            .iter()
            .map(|base| type_handle(&base.name))
            .collect(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for member in &decl.members {
        match member {
            ClassMember::Field(field) if field.visibility == Visibility::Public => {
                descriptor
                    .members
                    .push(field_member(field, MemberKind::Field, &owner));
            }
            ClassMember::Method(method) if method.visibility == Visibility::Public => {
                descriptor
                    .members
                    .push(method_member(method, MemberKind::Method, &owner));
            }
            ClassMember::Property(property) if property.visibility == Visibility::Public => {
                descriptor.members.push(property_member(property, &owner));
            }
            ClassMember::Constructor(ctor) if ctor.visibility == Visibility::Public => {
                descriptor.members.push(constructor_member(ctor, &owner));
            }
            ClassMember::Const(const_member) => {
                append_const_members(&mut descriptor.members, const_member, &owner);
            }
            _ => {}
        }
    }

    Some(descriptor)
}

pub(super) fn interface_descriptor(
    decl: &InterfaceDecl,
    scope: &[String],
) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Interface,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: decl
            .bases
            .iter()
            .map(|base| type_handle(&base.name))
            .collect(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for member in &decl.members {
        match member {
            InterfaceMember::Method(method) if method.visibility == Visibility::Public => {
                descriptor
                    .members
                    .push(method_member(method, MemberKind::TraitMethod, &owner));
            }
            InterfaceMember::Property(property) if property.visibility == Visibility::Public => {
                descriptor.members.push(property_member(property, &owner));
            }
            InterfaceMember::Const(const_member) => {
                append_const_members(&mut descriptor.members, const_member, &owner);
            }
            InterfaceMember::AssociatedType(assoc) => {
                descriptor
                    .members
                    .push(associated_type_member(assoc, &owner));
            }
            _ => {}
        }
    }

    Some(descriptor)
}

pub(super) fn trait_descriptor(decl: &TraitDecl, scope: &[String]) -> Option<TypeDescriptor> {
    if decl.visibility != Visibility::Public {
        return None;
    }

    let full_name = qualify(scope, &decl.name);
    let namespace = namespace_for_scope(scope);
    let owner = type_handle(&full_name);
    let generic_arguments = generic_handles(decl.generics.as_ref());
    let mut descriptor = TypeDescriptor {
        namespace,
        name: full_name.clone(),
        full_name,
        type_id: owner.type_id,
        kind: TypeKind::Trait,
        visibility: VisibilityDescriptor::from(decl.visibility),
        is_generic: !generic_arguments.is_empty(),
        generic_arguments,
        bases: decl
            .super_traits
            .iter()
            .map(|ty| type_handle(&ty.name))
            .collect(),
        attributes: attribute_descriptors(&decl.attributes),
        underlying_type: None,
        members: Vec::new(),
        layout: None,
        layout_hints: None,
        readonly: false,
    };

    for member in &decl.members {
        match member {
            TraitMember::Method(method) if method.visibility == Visibility::Public => {
                descriptor
                    .members
                    .push(method_member(method, MemberKind::TraitMethod, &owner));
            }
            TraitMember::AssociatedType(assoc) => {
                descriptor
                    .members
                    .push(associated_type_member(assoc, &owner));
            }
            TraitMember::Const(const_member) => {
                append_const_members(&mut descriptor.members, const_member, &owner);
            }
            TraitMember::Method(_) => {}
        }
    }

    Some(descriptor)
}
