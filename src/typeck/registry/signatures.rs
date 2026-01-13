use super::*;
use crate::frontend::ast::{Parameter, Signature, ThrowsClause, TypeExpr};
use crate::frontend::diagnostics::Span;
use crate::frontend::type_utils::{substitute_self_type, type_expr_surface};

pub(crate) fn signature_from(
    signature: &Signature,
    name: String,
    span: Option<Span>,
) -> FunctionSignature {
    FunctionSignature {
        name,
        param_types: signature
            .parameters
            .iter()
            .map(|param| type_expr_surface(&param.ty))
            .collect(),
        return_type: type_expr_surface(&signature.return_type),
        span,
    }
}

pub(crate) fn signature_from_extension(
    signature: &Signature,
    name: String,
    span: Option<Span>,
    self_ty: &TypeExpr,
) -> FunctionSignature {
    let substituted = Signature {
        parameters: signature
            .parameters
            .iter()
            .map(|param| Parameter {
                ty: substitute_self_type(&param.ty, self_ty),
                ..param.clone()
            })
            .collect(),
        return_type: substitute_self_type(&signature.return_type, self_ty),
        lends_to_return: signature.lends_to_return.clone(),
        variadic: signature.variadic,
        throws: signature.throws.as_ref().map(|clause| ThrowsClause {
            types: clause
                .types
                .iter()
                .map(|ty| substitute_self_type(ty, self_ty))
                .collect(),
            span: clause.span,
        }),
    };
    signature_from(&substituted, name, span)
}

pub(crate) fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}
