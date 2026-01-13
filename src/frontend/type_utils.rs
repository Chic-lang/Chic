use crate::frontend::ast::{
    FnTypeExpr, FunctionDecl, GenericArgument, Parameter, RefKind, Signature, ThrowsClause,
    TypeExpr, TypeSuffix,
};
use crate::frontend::diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceKind {
    Array,
    Vec,
    Span,
    ReadOnlySpan,
}

#[derive(Debug, Clone, Copy)]
pub struct SequenceDescriptor<'a> {
    pub kind: SequenceKind,
    pub element: &'a TypeExpr,
    pub rank: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct VectorDescriptor<'a> {
    pub element: &'a TypeExpr,
    pub lanes: &'a GenericArgument,
    pub lanes_span: Option<Span>,
}

pub fn sequence_descriptor<'a>(ty: &'a TypeExpr) -> Option<SequenceDescriptor<'a>> {
    let base_name = ty.base.last()?.as_str();
    let mut generic_args: Option<&[GenericArgument]> = None;
    let mut rank: Option<usize> = None;

    for suffix in &ty.suffixes {
        match suffix {
            TypeSuffix::GenericArgs(args) => generic_args = Some(args.as_slice()),
            TypeSuffix::Array(spec) => {
                if rank.is_none() {
                    rank = Some(spec.dimensions);
                }
            }
            TypeSuffix::Qualifier(_) => return None,
            _ => {}
        }
    }

    match base_name {
        "Array" => {
            let args = generic_args?;
            if args.len() != 1 {
                return None;
            }
            let element = args[0].ty()?;
            Some(SequenceDescriptor {
                kind: SequenceKind::Array,
                element,
                rank: rank.unwrap_or(1),
            })
        }
        "Vec" => {
            let args = generic_args?;
            if args.len() != 1 {
                return None;
            }
            let element = args[0].ty()?;
            Some(SequenceDescriptor {
                kind: SequenceKind::Vec,
                element,
                rank: 1,
            })
        }
        "Span" => {
            let args = generic_args?;
            if args.len() != 1 {
                return None;
            }
            let element = args[0].ty()?;
            Some(SequenceDescriptor {
                kind: SequenceKind::Span,
                element,
                rank: 1,
            })
        }
        "ReadOnlySpan" => {
            let args = generic_args?;
            if args.len() != 1 {
                return None;
            }
            let element = args[0].ty()?;
            Some(SequenceDescriptor {
                kind: SequenceKind::ReadOnlySpan,
                element,
                rank: 1,
            })
        }
        _ => None,
    }
}

/// Detect a `vector<T, N>` SIMD type expression and surface its components.
#[must_use]
pub fn vector_descriptor<'a>(ty: &'a TypeExpr) -> Option<VectorDescriptor<'a>> {
    let base_name = ty.base.last()?;
    if base_name != "vector" {
        return None;
    }

    let args = ty.generic_arguments()?;
    if args.len() != 2 {
        return None;
    }
    let element = args[0].ty()?;
    let lanes = &args[1];
    let lanes_span = lanes.expression().span.or(ty.span);
    Some(VectorDescriptor {
        element,
        lanes,
        lanes_span,
    })
}

#[must_use]
pub fn base_identifier(ty: &TypeExpr) -> Option<&str> {
    ty.base.last().map(String::as_str)
}

#[must_use]
pub fn qualify_name(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) if !ns.is_empty() => format!("{}::{}", ns.replace('.', "::"), name),
        _ => name.to_string(),
    }
}

#[must_use]
pub fn type_expr_surface(expr: &TypeExpr) -> String {
    let name = expr.name.trim();
    let Some(kind) = expr.ref_kind else {
        return name.to_string();
    };

    let starts_with_ref = name.starts_with("ref ")
        || name.starts_with("ref readonly ")
        || name.starts_with("ref\t")
        || name.starts_with("ref\r")
        || name.starts_with("ref\n");
    if starts_with_ref {
        return name.to_string();
    }

    match kind {
        RefKind::Ref => format!("ref {name}"),
        RefKind::ReadOnly => format!("ref readonly {name}"),
    }
}

fn is_self_type(expr: &TypeExpr) -> bool {
    expr.base.len() == 1 && expr.base[0] == "Self"
}

fn substitute_suffixes(suffixes: &[TypeSuffix], replacement: &TypeExpr) -> Vec<TypeSuffix> {
    suffixes
        .iter()
        .map(|suffix| match suffix {
            TypeSuffix::GenericArgs(args) => TypeSuffix::GenericArgs(
                args.iter()
                    .map(|arg| {
                        let substituted_ty = arg
                            .ty()
                            .map(|inner| substitute_self_type(inner, replacement));
                        GenericArgument::new(substituted_ty, arg.expression().clone())
                    })
                    .collect(),
            ),
            TypeSuffix::Array(spec) => TypeSuffix::Array(*spec),
            TypeSuffix::Nullable => TypeSuffix::Nullable,
            TypeSuffix::Pointer { mutable, modifiers } => TypeSuffix::Pointer {
                mutable: *mutable,
                modifiers: modifiers.clone(),
            },
            TypeSuffix::Qualifier(qualifier) => TypeSuffix::Qualifier(qualifier.clone()),
        })
        .collect()
}

fn substitute_fn_signature(signature: &FnTypeExpr, replacement: &TypeExpr) -> FnTypeExpr {
    FnTypeExpr {
        abi: signature.abi.clone(),
        params: signature
            .params
            .iter()
            .map(|param| substitute_self_type(param, replacement))
            .collect(),
        return_type: Box::new(substitute_self_type(&signature.return_type, replacement)),
        variadic: signature.variadic,
    }
}

fn substitute_trait_object(bounds: &[TypeExpr], replacement: &TypeExpr) -> Vec<TypeExpr> {
    bounds
        .iter()
        .map(|bound| substitute_self_type(bound, replacement))
        .collect()
}

#[must_use]
pub fn substitute_self_type(expr: &TypeExpr, replacement: &TypeExpr) -> TypeExpr {
    if is_self_type(expr) {
        let mut substituted = replacement.clone();
        substituted
            .suffixes
            .extend(substitute_suffixes(&expr.suffixes, replacement));
        if let Some(elements) = &substituted.tuple_elements {
            substituted.tuple_elements = Some(
                elements
                    .iter()
                    .map(|element| substitute_self_type(element, replacement))
                    .collect(),
            );
        }
        if let Some(signature) = &substituted.fn_signature {
            substituted.fn_signature = Some(substitute_fn_signature(signature, replacement));
        }
        if let Some(object) = &mut substituted.trait_object {
            object.bounds = substitute_trait_object(&object.bounds, replacement);
        }
        substituted
    } else {
        let mut cloned = expr.clone();
        cloned.suffixes = substitute_suffixes(&expr.suffixes, replacement);
        if let Some(elements) = &expr.tuple_elements {
            cloned.tuple_elements = Some(
                elements
                    .iter()
                    .map(|element| substitute_self_type(element, replacement))
                    .collect(),
            );
        }
        if let Some(signature) = &expr.fn_signature {
            cloned.fn_signature = Some(substitute_fn_signature(signature, replacement));
        }
        if let Some(object) = &mut cloned.trait_object {
            object.bounds = substitute_trait_object(&object.bounds, replacement);
        }
        cloned
    }
}

#[must_use]
pub fn instantiate_extension_method(method: &FunctionDecl, target: &TypeExpr) -> FunctionDecl {
    let mut cloned = method.clone();
    cloned.signature = Signature {
        parameters: method
            .signature
            .parameters
            .iter()
            .map(|param| Parameter {
                ty: substitute_self_type(&param.ty, target),
                ..param.clone()
            })
            .collect(),
        return_type: substitute_self_type(&method.signature.return_type, target),
        lends_to_return: method.signature.lends_to_return.clone(),
        variadic: method.signature.variadic,
        throws: method.signature.throws.as_ref().map(|clause| ThrowsClause {
            types: clause
                .types
                .iter()
                .map(|ty| substitute_self_type(ty, target))
                .collect(),
            span: clause.span,
        }),
    };
    cloned
}

#[must_use]
pub fn extension_method_symbol(
    target_name: &str,
    namespace: Option<&str>,
    method_name: &str,
    is_default: bool,
) -> String {
    if !is_default {
        return format!("{target_name}::{method_name}");
    }
    let scope = namespace.unwrap_or("Global");
    let sanitized = scope.replace("::", "_");
    format!("{target_name}::__default__{sanitized}::{method_name}")
}
