use crate::drop_glue::drop_type_identity;
use crate::frontend::ast::items::{
    Attribute, AutoTraitConstraint, GenericConstraintKind, GenericParams,
};
use crate::frontend::attributes::LayoutHints;
use crate::frontend::metadata::{
    AttributeArgument, AttributeDescriptor, LayoutDescriptor, TypeHandle,
};

pub(super) fn layout_descriptor_from_hints(hints: &LayoutHints) -> LayoutDescriptor {
    LayoutDescriptor {
        repr_c: hints.repr_c,
        pack: hints.packing.and_then(|hint| hint.value),
        align: hints.align.map(|hint| hint.value),
    }
}

pub(super) fn generic_handles(params: Option<&GenericParams>) -> Vec<TypeHandle> {
    params
        .map(|params| {
            params
                .params
                .iter()
                .map(|param| {
                    let name = if let Some(data) = param.as_type() {
                        let mut descriptor = String::new();
                        if let Some(keyword) = data.variance.keyword() {
                            descriptor.push_str(keyword);
                            descriptor.push(' ');
                        }
                        descriptor.push_str(&param.name);
                        if !data.constraints.is_empty() {
                            descriptor.push_str(" : ");
                            let mut first = true;
                            for constraint in &data.constraints {
                                if !first {
                                    descriptor.push_str(", ");
                                }
                                descriptor.push_str(&describe_constraint(&constraint.kind));
                                first = false;
                            }
                        }
                        descriptor
                    } else if let Some(data) = param.as_const() {
                        format!("const {} : {}", param.name, data.ty.name)
                    } else {
                        param.name.clone()
                    };
                    TypeHandle {
                        name,
                        type_id: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn describe_constraint(kind: &GenericConstraintKind) -> String {
    match kind {
        GenericConstraintKind::Type(expr) => expr.name.clone(),
        GenericConstraintKind::Struct => "struct".into(),
        GenericConstraintKind::Class => "class".into(),
        GenericConstraintKind::NotNull => "notnull".into(),
        GenericConstraintKind::DefaultConstructor => "new()".into(),
        GenericConstraintKind::AutoTrait(trait_kind) => match trait_kind {
            AutoTraitConstraint::ThreadSafe => "@thread_safe".into(),
            AutoTraitConstraint::Shareable => "@shareable".into(),
        },
    }
}

pub(super) fn attribute_descriptors(attributes: &[Attribute]) -> Vec<AttributeDescriptor> {
    attributes
        .iter()
        .map(|attr| {
            let mut positional_args = Vec::new();
            let mut named_args = Vec::new();
            for arg in &attr.arguments {
                let descriptor = AttributeArgument {
                    name: arg.name.clone(),
                    value: arg.value.clone(),
                };
                if arg.name.is_some() {
                    named_args.push(descriptor);
                } else {
                    positional_args.push(descriptor);
                }
            }
            AttributeDescriptor {
                name: attr.name.clone(),
                positional_args,
                named_args,
            }
        })
        .collect()
}

pub(super) fn extend_scope(scope: &mut Vec<String>, path: &str) -> usize {
    let parts = split_path(path);
    let mut prefix = 0usize;
    while prefix < parts.len() && prefix < scope.len() && scope[prefix] == parts[prefix] {
        prefix += 1;
    }
    for part in parts.iter().skip(prefix) {
        scope.push(part.clone());
    }
    parts.len().saturating_sub(prefix)
}

pub(super) fn split_path(path: &str) -> Vec<String> {
    path.replace("::", ".")
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

pub(super) fn qualify(scope: &[String], name: &str) -> String {
    let mut parts = scope.to_vec();
    parts.extend(split_path(name));
    parts.join("::")
}

pub(super) fn namespace_for_scope(scope: &[String]) -> Option<String> {
    if scope.is_empty() {
        None
    } else {
        Some(scope.join("::"))
    }
}

pub(super) fn type_handle(name: &str) -> TypeHandle {
    TypeHandle {
        name: name.to_string(),
        type_id: Some(drop_type_identity(name)),
    }
}
