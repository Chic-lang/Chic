#![allow(dead_code)]

use crate::frontend::lexer::{
    NumericBase, NumericLiteral, NumericLiteralKind, NumericLiteralSuffix,
};

/// Parsed representation of an integer literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerLiteral {
    pub value: u128,
    pub is_unsigned: bool,
    pub width: Option<IntegerWidth>,
    pub suffix: Option<NumericLiteralSuffix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerWidth {
    W8,
    W16,
    W32,
    W64,
    W128,
    Size,
}

impl IntegerWidth {
    #[must_use]
    pub fn bit_width(self, pointer_bits: u32) -> u32 {
        match self {
            Self::W8 => 8,
            Self::W16 => 16,
            Self::W32 => 32,
            Self::W64 => 64,
            Self::W128 => 128,
            Self::Size => pointer_bits,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericLiteralType {
    Signed(IntegerWidth),
    Unsigned(IntegerWidth),
    Float16,
    Float32,
    Float64,
    Float128,
    Decimal,
}

#[derive(Debug, Clone)]
pub struct NumericLiteralMetadata {
    pub literal_type: NumericLiteralType,
    pub suffix_text: Option<String>,
    pub explicit_suffix: bool,
}

impl NumericLiteralMetadata {
    #[must_use]
    pub fn suffix_str(&self) -> Option<&str> {
        self.suffix_text.as_deref()
    }
}

/// Interpret a numeric literal as an integer constant when possible.
/// Returns `None` if the literal contains fractional/exponent parts, targets decimal/float suffixes,
/// or overflows `u128`.
pub fn parse_integer_literal(literal: &NumericLiteral) -> Option<IntegerLiteral> {
    if literal.has_errors() || literal.kind != NumericLiteralKind::Integer {
        return None;
    }

    if literal.integer.is_empty() {
        return None;
    }

    let base = match literal.base {
        NumericBase::Binary => 2,
        NumericBase::Decimal => 10,
        NumericBase::Hexadecimal => 16,
    };

    let value = u128::from_str_radix(&literal.integer, base).ok()?;
    let (is_unsigned, width) = classify_suffix(literal.suffix)?;

    Some(IntegerLiteral {
        value,
        is_unsigned,
        width,
        suffix: literal.suffix,
    })
}

#[must_use]
pub fn numeric_literal_metadata(literal: &NumericLiteral) -> Option<NumericLiteralMetadata> {
    match literal.kind {
        NumericLiteralKind::Integer => {
            let parsed = parse_integer_literal(literal)?;
            let suffix = parsed.suffix.map(|suffix| suffix.label().to_string());
            let width = parsed.width?;
            let literal_type = if parsed.is_unsigned {
                NumericLiteralType::Unsigned(width)
            } else {
                NumericLiteralType::Signed(width)
            };
            Some(NumericLiteralMetadata {
                literal_type,
                suffix_text: suffix,
                explicit_suffix: true,
            })
        }
        NumericLiteralKind::Float => {
            let suffix = literal.suffix?;
            match suffix {
                NumericLiteralSuffix::F16 => Some(NumericLiteralMetadata {
                    literal_type: NumericLiteralType::Float16,
                    suffix_text: Some(suffix.label().to_string()),
                    explicit_suffix: true,
                }),
                NumericLiteralSuffix::F32 => Some(NumericLiteralMetadata {
                    literal_type: NumericLiteralType::Float32,
                    suffix_text: Some(suffix.label().to_string()),
                    explicit_suffix: true,
                }),
                NumericLiteralSuffix::F64 => Some(NumericLiteralMetadata {
                    literal_type: NumericLiteralType::Float64,
                    suffix_text: Some(suffix.label().to_string()),
                    explicit_suffix: true,
                }),
                NumericLiteralSuffix::F128 => Some(NumericLiteralMetadata {
                    literal_type: NumericLiteralType::Float128,
                    suffix_text: Some(suffix.label().to_string()),
                    explicit_suffix: true,
                }),
                _ => None,
            }
        }
        NumericLiteralKind::Decimal => Some(NumericLiteralMetadata {
            literal_type: NumericLiteralType::Decimal,
            suffix_text: Some("m".to_string()),
            explicit_suffix: true,
        }),
    }
}

fn classify_suffix(suffix: Option<NumericLiteralSuffix>) -> Option<(bool, Option<IntegerWidth>)> {
    match suffix {
        None => Some((false, None)),
        Some(s) => match s {
            NumericLiteralSuffix::I8 => Some((false, Some(IntegerWidth::W8))),
            NumericLiteralSuffix::I16 => Some((false, Some(IntegerWidth::W16))),
            NumericLiteralSuffix::I32 => Some((false, Some(IntegerWidth::W32))),
            NumericLiteralSuffix::I64 => Some((false, Some(IntegerWidth::W64))),
            NumericLiteralSuffix::I128 => Some((false, Some(IntegerWidth::W128))),
            NumericLiteralSuffix::Isize => Some((false, Some(IntegerWidth::Size))),
            NumericLiteralSuffix::U8 => Some((true, Some(IntegerWidth::W8))),
            NumericLiteralSuffix::U16 => Some((true, Some(IntegerWidth::W16))),
            NumericLiteralSuffix::U32 => Some((true, Some(IntegerWidth::W32))),
            NumericLiteralSuffix::U64 => Some((true, Some(IntegerWidth::W64))),
            NumericLiteralSuffix::U128 => Some((true, Some(IntegerWidth::W128))),
            NumericLiteralSuffix::Usize => Some((true, Some(IntegerWidth::Size))),
            NumericLiteralSuffix::Decimal
            | NumericLiteralSuffix::F16
            | NumericLiteralSuffix::F32
            | NumericLiteralSuffix::F64
            | NumericLiteralSuffix::F128 => None,
        },
    }
}
