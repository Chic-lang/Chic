use crate::frontend::ast::{GenericParamKind, GenericParams, MemberDispatch};
use crate::syntax::expr::ExprNode;
use std::convert::TryFrom;

pub(crate) fn expect_u32_index(index: usize, context: &str) -> u32 {
    u32::try_from(index).unwrap_or_else(|_| panic!("{context} exceeds u32 range"))
}

pub(crate) fn expr_path_segments(node: &ExprNode) -> Result<Vec<String>, String> {
    match node {
        ExprNode::Identifier(name) => Ok(vec![name.clone()]),
        ExprNode::Member { base, member, .. } => {
            let mut segments = expr_path_segments(base)?;
            segments.push(member.clone());
            Ok(segments)
        }
        ExprNode::Parenthesized(inner) => expr_path_segments(inner),
        _ => Err("expression is not a simple path".to_string()),
    }
}

pub(crate) fn is_power_of_two(value: u128) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

pub(crate) fn collect_type_param_names(generics: Option<&GenericParams>) -> Vec<String> {
    generics
        .map(|params| {
            params
                .params
                .iter()
                .filter_map(|param| {
                    if matches!(param.kind, GenericParamKind::Type(_)) {
                        Some(param.name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) const fn dispatch_participates(dispatch: MemberDispatch) -> bool {
    dispatch.is_virtual || dispatch.is_override || dispatch.is_abstract || dispatch.is_sealed
}
