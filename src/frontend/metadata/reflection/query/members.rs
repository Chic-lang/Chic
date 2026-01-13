use super::helpers::{attribute_descriptors, type_handle};
use crate::frontend::ast::items::{
    ConstDeclarator, ConstMemberDecl, ConstructorDecl, ConstructorKind, FieldDecl, FunctionDecl,
    Parameter, PropertyAccessorKind, PropertyDecl, TraitAssociatedType, UnionField, UnionViewDecl,
};
use crate::frontend::metadata::{
    AttributeDescriptor, ConstructorDescriptor, FieldDescriptor, MemberDescriptor, MemberKind,
    MethodDescriptor, ParameterDescriptor, ParameterMode, PropertyDescriptor, TypeHandle,
    VisibilityDescriptor,
};

pub(super) fn field_member(
    field: &FieldDecl,
    kind: MemberKind,
    owner: &TypeHandle,
) -> MemberDescriptor {
    MemberDescriptor {
        name: field.name.clone(),
        kind,
        visibility: VisibilityDescriptor::from(field.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&field.attributes),
        field: Some(FieldDescriptor {
            field_type: type_handle(&field.ty.name),
            is_static: field.is_static,
            is_readonly: field.is_readonly,
            offset: None,
        }),
        property: None,
        method: None,
        constructor: None,
        children: Vec::new(),
    }
}

pub(super) fn union_field_member(field: &UnionField, owner: &TypeHandle) -> MemberDescriptor {
    MemberDescriptor {
        name: field.name.clone(),
        kind: MemberKind::UnionField,
        visibility: VisibilityDescriptor::from(field.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&field.attributes),
        field: Some(FieldDescriptor {
            field_type: type_handle(&field.ty.name),
            is_static: false,
            is_readonly: field.is_readonly,
            offset: None,
        }),
        property: None,
        method: None,
        constructor: None,
        children: Vec::new(),
    }
}

pub(super) fn union_view_member(view: &UnionViewDecl, owner: &TypeHandle) -> MemberDescriptor {
    let mut member = MemberDescriptor {
        name: view.name.clone(),
        kind: MemberKind::UnionView,
        visibility: VisibilityDescriptor::from(view.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&view.attributes),
        field: None,
        property: None,
        method: None,
        constructor: None,
        children: Vec::new(),
    };

    for field in &view.fields {
        member
            .children
            .push(field_member(field, MemberKind::UnionField, owner));
    }

    member
}

pub(super) fn property_member(property: &PropertyDecl, owner: &TypeHandle) -> MemberDescriptor {
    let mut descriptor = MemberDescriptor {
        name: property.name.clone(),
        kind: MemberKind::Property,
        visibility: VisibilityDescriptor::from(property.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&property.attributes),
        field: None,
        property: Some(PropertyDescriptor {
            property_type: type_handle(&property.ty.name),
            has_getter: false,
            has_setter: false,
            has_init: false,
            parameters: property
                .parameters
                .iter()
                .map(parameter_descriptor)
                .collect(),
            getter: None,
            setter: None,
            init: None,
        }),
        method: None,
        constructor: None,
        children: Vec::new(),
    };

    if let Some(prop) = descriptor.property.as_mut() {
        for accessor in &property.accessors {
            match accessor.kind {
                PropertyAccessorKind::Get => prop.has_getter = true,
                PropertyAccessorKind::Set => prop.has_setter = true,
                PropertyAccessorKind::Init => prop.has_init = true,
            }
        }
    }

    descriptor
}

pub(super) fn constructor_member(ctor: &ConstructorDecl, owner: &TypeHandle) -> MemberDescriptor {
    MemberDescriptor {
        name: "Constructor".to_string(),
        kind: MemberKind::Constructor,
        visibility: VisibilityDescriptor::from(ctor.visibility),
        declaring_type: owner.clone(),
        attributes: attribute_descriptors(&ctor.attributes),
        field: None,
        property: None,
        method: None,
        constructor: Some(ConstructorDescriptor {
            parameters: ctor.parameters.iter().map(parameter_descriptor).collect(),
            is_designated: matches!(ctor.kind, ConstructorKind::Designated),
            is_convenience: matches!(ctor.kind, ConstructorKind::Convenience),
        }),
        children: Vec::new(),
    }
}

pub(super) fn method_member(
    function: &FunctionDecl,
    default_kind: MemberKind,
    owner: &TypeHandle,
) -> MemberDescriptor {
    let mut attributes = attribute_descriptors(&function.attributes);
    if let Some(hint) = function.vectorize_hint {
        attributes.push(AttributeDescriptor {
            name: format!("vectorize={}", hint.as_str()),
            positional_args: Vec::new(),
            named_args: Vec::new(),
        });
    }
    let descriptor = MemberDescriptor {
        name: function.name.clone(),
        kind: default_kind,
        visibility: VisibilityDescriptor::from(function.visibility),
        declaring_type: owner.clone(),
        attributes,
        field: None,
        property: None,
        method: Some(MethodDescriptor {
            return_type: type_handle(&function.signature.return_type.name),
            parameters: function
                .signature
                .parameters
                .iter()
                .map(parameter_descriptor)
                .collect(),
            is_static: function.modifiers.iter().any(|m| m == "static"),
            is_virtual: function.modifiers.iter().any(|m| m == "virtual"),
            is_override: function.modifiers.iter().any(|m| m == "override"),
            is_abstract: function.modifiers.iter().any(|m| m == "abstract"),
            is_async: function.is_async,
            throws: throws_list(function),
            extern_abi: function.extern_abi.clone(),
        }),
        constructor: None,
        children: Vec::new(),
    };

    descriptor
}

pub(super) fn associated_type_member(
    assoc: &TraitAssociatedType,
    owner: &TypeHandle,
) -> MemberDescriptor {
    let mut descriptor = MemberDescriptor {
        name: assoc.name.clone(),
        kind: MemberKind::AssociatedType,
        visibility: VisibilityDescriptor::Public,
        declaring_type: owner.clone(),
        attributes: Vec::new(),
        field: None,
        property: None,
        method: None,
        constructor: None,
        children: Vec::new(),
    };

    if let Some(default) = assoc.default.as_ref() {
        descriptor.field = Some(FieldDescriptor {
            field_type: type_handle(&default.name),
            is_static: false,
            is_readonly: true,
            offset: None,
        });
    }

    descriptor
}

pub(super) fn append_const_members(
    target: &mut Vec<MemberDescriptor>,
    const_decl: &ConstMemberDecl,
    owner: &TypeHandle,
) {
    for declarator in &const_decl.declaration.declarators {
        target.push(const_member(
            declarator,
            &const_decl.declaration.ty.name,
            owner,
        ));
    }
}

pub(super) fn const_member(
    declarator: &ConstDeclarator,
    type_name: &str,
    owner: &TypeHandle,
) -> MemberDescriptor {
    MemberDescriptor {
        name: declarator.name.clone(),
        kind: MemberKind::Const,
        visibility: VisibilityDescriptor::Public,
        declaring_type: owner.clone(),
        attributes: Vec::new(),
        field: Some(FieldDescriptor {
            field_type: type_handle(type_name),
            is_static: true,
            is_readonly: true,
            offset: None,
        }),
        property: None,
        method: None,
        constructor: None,
        children: Vec::new(),
    }
}

pub(super) fn throws_list(function: &FunctionDecl) -> Vec<String> {
    function
        .signature
        .throws
        .as_ref()
        .map(|throws| throws.types.iter().map(|ty| ty.name.clone()).collect())
        .unwrap_or_default()
}

pub(super) fn parameter_descriptor(parameter: &Parameter) -> ParameterDescriptor {
    ParameterDescriptor {
        name: parameter.name.clone(),
        parameter_type: type_handle(&parameter.ty.name),
        mode: ParameterMode::from(parameter.binding),
        has_default: parameter.default.is_some(),
        default_value: parameter.default.as_ref().map(|expr| expr.text.clone()),
        attributes: attribute_descriptors(&parameter.attributes),
    }
}
