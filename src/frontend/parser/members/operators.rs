use super::*;

#[derive(Clone, Copy)]
pub(super) enum OperatorTokenKind {
    Unary(UnaryOperator),
    Binary(BinaryOperator),
    UnaryOrBinary {
        unary: UnaryOperator,
        binary: BinaryOperator,
    },
}

pub(super) fn operator_token_kind(token: &Token) -> Option<OperatorTokenKind> {
    match token.kind {
        TokenKind::Operator(op) => match op {
            "+" => Some(OperatorTokenKind::UnaryOrBinary {
                unary: UnaryOperator::UnaryPlus,
                binary: BinaryOperator::Add,
            }),
            "-" => Some(OperatorTokenKind::UnaryOrBinary {
                unary: UnaryOperator::Negate,
                binary: BinaryOperator::Subtract,
            }),
            "*" => Some(OperatorTokenKind::Binary(BinaryOperator::Multiply)),
            "/" => Some(OperatorTokenKind::Binary(BinaryOperator::Divide)),
            "%" => Some(OperatorTokenKind::Binary(BinaryOperator::Remainder)),
            "&" => Some(OperatorTokenKind::Binary(BinaryOperator::BitAnd)),
            "|" => Some(OperatorTokenKind::Binary(BinaryOperator::BitOr)),
            "^" => Some(OperatorTokenKind::Binary(BinaryOperator::BitXor)),
            "<<" => Some(OperatorTokenKind::Binary(BinaryOperator::ShiftLeft)),
            ">>" => Some(OperatorTokenKind::Binary(BinaryOperator::ShiftRight)),
            "==" => Some(OperatorTokenKind::Binary(BinaryOperator::Equal)),
            "!=" => Some(OperatorTokenKind::Binary(BinaryOperator::NotEqual)),
            "<" => Some(OperatorTokenKind::Binary(BinaryOperator::LessThan)),
            "<=" => Some(OperatorTokenKind::Binary(BinaryOperator::LessThanOrEqual)),
            ">" => Some(OperatorTokenKind::Binary(BinaryOperator::GreaterThan)),
            ">=" => Some(OperatorTokenKind::Binary(
                BinaryOperator::GreaterThanOrEqual,
            )),
            "!" => Some(OperatorTokenKind::Unary(UnaryOperator::LogicalNot)),
            "~" => Some(OperatorTokenKind::Unary(UnaryOperator::OnesComplement)),
            "++" => Some(OperatorTokenKind::Unary(UnaryOperator::Increment)),
            "--" => Some(OperatorTokenKind::Unary(UnaryOperator::Decrement)),
            _ => None,
        },
        TokenKind::Punctuation('<') => Some(OperatorTokenKind::Binary(BinaryOperator::LessThan)),
        TokenKind::Punctuation('>') => Some(OperatorTokenKind::Binary(BinaryOperator::GreaterThan)),
        _ => None,
    }
}

pub(super) fn canonical_operator_name(kind: &OperatorKind, return_type: &TypeExpr) -> String {
    match kind {
        OperatorKind::Unary(op) => canonical_unary_name(*op).to_string(),
        OperatorKind::Binary(op) => canonical_binary_name(*op).to_string(),
        OperatorKind::Conversion(conv) => canonical_conversion_name(*conv, &return_type.name),
    }
}

pub(super) fn canonical_conversion_name(kind: ConversionKind, target: &str) -> String {
    let fragment = sanitize_type_fragment(target);
    match kind {
        ConversionKind::Implicit => format!("op_Implicit_{fragment}"),
        ConversionKind::Explicit => format!("op_Explicit_{fragment}"),
    }
}

fn canonical_unary_name(op: UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Negate => "op_UnaryNegation",
        UnaryOperator::UnaryPlus => "op_UnaryPlus",
        UnaryOperator::LogicalNot => "op_LogicalNot",
        UnaryOperator::OnesComplement => "op_OnesComplement",
        UnaryOperator::Increment => "op_Increment",
        UnaryOperator::Decrement => "op_Decrement",
    }
}

fn canonical_binary_name(op: BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Add => "op_Addition",
        BinaryOperator::Subtract => "op_Subtraction",
        BinaryOperator::Multiply => "op_Multiply",
        BinaryOperator::Divide => "op_Division",
        BinaryOperator::Remainder => "op_Modulus",
        BinaryOperator::BitAnd => "op_BitwiseAnd",
        BinaryOperator::BitOr => "op_BitwiseOr",
        BinaryOperator::BitXor => "op_ExclusiveOr",
        BinaryOperator::ShiftLeft => "op_LeftShift",
        BinaryOperator::ShiftRight => "op_RightShift",
        BinaryOperator::Equal => "op_Equality",
        BinaryOperator::NotEqual => "op_Inequality",
        BinaryOperator::LessThan => "op_LessThan",
        BinaryOperator::LessThanOrEqual => "op_LessThanOrEqual",
        BinaryOperator::GreaterThan => "op_GreaterThan",
        BinaryOperator::GreaterThanOrEqual => "op_GreaterThanOrEqual",
    }
}

fn sanitize_type_fragment(name: &str) -> String {
    let mut fragment = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            fragment.push(ch);
        } else {
            fragment.push('_');
        }
    }
    if fragment.is_empty() {
        "_".to_string()
    } else {
        fragment
    }
}
