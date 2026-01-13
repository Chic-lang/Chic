use super::*;

mod borrow;
mod operators;
mod place;

#[derive(Clone)]
pub(crate) enum OperatorResolution {
    Handled(OperatorOverload),
    Skip,
    Error,
}

pub(crate) fn unary_operator_symbol(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::UnaryPlus => "+",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
        UnOp::Increment => "++",
        UnOp::Decrement => "--",
        UnOp::Deref => "*",
        UnOp::AddrOf => "&",
        UnOp::AddrOfMut => "&mut",
    }
}

pub(crate) fn binary_operator_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::NullCoalesce => "??",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
    }
}

pub(crate) fn collect_path_segments(node: &ExprNode) -> Option<Vec<String>> {
    match node {
        ExprNode::Identifier(name) => {
            let segments = vec![name.clone()];
            if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok()
                && segments.iter().any(|seg| seg.contains("ReadOnlySpan"))
            {
                eprintln!(
                    "[owner-args-debug] collect_path Identifier segments={segments:?} node={node:?}"
                );
            }
            Some(segments)
        }
        ExprNode::Member { base, member, .. } => {
            let mut segments = collect_path_segments(base)?;
            segments.push(member.clone());
            if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok()
                && segments.iter().any(|seg| seg.contains("ReadOnlySpan"))
            {
                eprintln!(
                    "[owner-args-debug] collect_path Member segments={segments:?} node={node:?}"
                );
            }
            Some(segments)
        }
        ExprNode::Parenthesized(inner) => collect_path_segments(inner),
        _ => None,
    }
}

body_builder_impl! {}
