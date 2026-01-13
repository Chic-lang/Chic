use crate::frontend::lexer::{NumericBase, NumericLiteral, NumericLiteralKind};

pub(super) fn parse_u64_numeric_literal(literal: &NumericLiteral) -> Option<u64> {
    if literal.has_errors() || literal.kind != NumericLiteralKind::Integer {
        return None;
    }

    let base = match literal.base {
        NumericBase::Binary => 2,
        NumericBase::Decimal => 10,
        NumericBase::Hexadecimal => 16,
    };

    if literal.integer.is_empty() {
        return None;
    }

    u64::from_str_radix(&literal.integer, base).ok()
}

pub(super) fn parse_u64_literal(text: &str) -> Option<u64> {
    let cleaned = text.replace('_', "");
    if cleaned.is_empty() {
        return None;
    }

    let (base, digits) = if let Some(rest) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (16, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (2, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (8, rest)
    } else {
        (10, cleaned.as_str())
    };

    if digits.is_empty() {
        return None;
    }

    u64::from_str_radix(digits, base).ok()
}

pub(super) fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}
