use super::*;
use crate::frontend::ast::Expression;
use crate::mir::ConstValue;
use crate::syntax::expr::ExprNode;
use crate::syntax::expr::builders::LiteralConst;
use crate::syntax::numeric::{IntegerWidth, NumericLiteralMetadata, NumericLiteralType};

impl<'a> TypeChecker<'a> {
    pub(crate) fn check_numeric_literal_expression(
        &mut self,
        expr: &Expression,
        declared_type: Option<&TypeExpr>,
    ) {
        if let Some(node) = expr.node.as_ref() {
            if let ExprNode::Literal(literal) = node {
                self.check_numeric_literal(literal, declared_type, expr.span);
            }
        }
    }

    pub(crate) fn check_numeric_literal(
        &mut self,
        literal: &LiteralConst,
        declared_type: Option<&TypeExpr>,
        span: Option<Span>,
    ) {
        let Some(meta) = literal.numeric.as_ref() else {
            return;
        };
        if !meta.explicit_suffix {
            return;
        }

        match (&literal.value, meta.literal_type) {
            (ConstValue::Int(value), NumericLiteralType::Signed(width)) => {
                if let Some((min, max)) = Self::signed_bounds(width) {
                    if *value < min || *value > max {
                        let suffix = Self::literal_suffix(meta);
                        let type_name = Self::literal_type_name(meta.literal_type);
                        self.emit_error(
                            codes::NUMERIC_LITERAL_SUFFIX_OVERFLOW,
                            span,
                            format!(
                                "literal with suffix `{suffix}` has value `{value}`, which exceeds the range of `{type_name}`"
                            ),
                        );
                        return;
                    }
                }
            }
            (ConstValue::UInt(value), NumericLiteralType::Unsigned(width)) => {
                if let Some(max) = Self::unsigned_max(width) {
                    if *value > max {
                        let suffix = Self::literal_suffix(meta);
                        let type_name = Self::literal_type_name(meta.literal_type);
                        self.emit_error(
                            codes::NUMERIC_LITERAL_SUFFIX_OVERFLOW,
                            span,
                            format!(
                                "literal with suffix `{suffix}` has value `{value}`, which exceeds the range of `{type_name}`"
                            ),
                        );
                        return;
                    }
                }
            }
            _ => {}
        }

        if let Some(target) = declared_type {
            if !self.numeric_type_matches(target, meta.literal_type) {
                let suffix = Self::literal_suffix(meta);
                let literal_type = Self::literal_type_name(meta.literal_type);
                let target_name = self.canonical_type_name(target);
                self.emit_error(
                    codes::NUMERIC_LITERAL_SUFFIX_MISMATCH,
                    span,
                    format!(
                        "literal with suffix `{suffix}` has type `{literal_type}` but is used where `{target_name}` is expected"
                    ),
                );
            }
        }
    }

    fn numeric_type_matches(&self, ty: &TypeExpr, literal_type: NumericLiteralType) -> bool {
        match literal_type {
            NumericLiteralType::Signed(width) => {
                self.type_matches_aliases(ty, Self::signed_aliases(width))
            }
            NumericLiteralType::Unsigned(width) => {
                self.type_matches_aliases(ty, Self::unsigned_aliases(width))
            }
            NumericLiteralType::Float32 => {
                self.type_matches_aliases(ty, &["float", "single", "system::single"])
            }
            NumericLiteralType::Float64 => {
                self.type_matches_aliases(ty, &["double", "system::double"])
            }
            NumericLiteralType::Float16 => {
                self.type_matches_aliases(ty, &["float16", "half", "f16", "system::float16"])
            }
            NumericLiteralType::Float128 => {
                self.type_matches_aliases(ty, &["float128", "quad", "f128", "system::float128"])
            }
            NumericLiteralType::Decimal => {
                self.type_matches_aliases(ty, &["decimal", "system::decimal"])
            }
        }
    }

    fn type_matches_aliases(&self, ty: &TypeExpr, aliases: &[&str]) -> bool {
        let canonical = self.canonical_type_name(ty);
        let canonical_lower = canonical.to_ascii_lowercase();
        let simple_lower = diagnostics::simple_name(&canonical).to_ascii_lowercase();
        aliases.iter().any(|alias| {
            let alias_lower = alias.to_ascii_lowercase();
            alias_lower == canonical_lower || alias_lower == simple_lower
        })
    }

    pub(crate) fn literal_suffix(meta: &NumericLiteralMetadata) -> String {
        meta.suffix_text
            .clone()
            .unwrap_or_else(|| Self::literal_type_name(meta.literal_type).to_string())
    }

    pub(crate) fn literal_type_name(literal_type: NumericLiteralType) -> &'static str {
        match literal_type {
            NumericLiteralType::Signed(width) => match width {
                IntegerWidth::W8 => "i8",
                IntegerWidth::W16 => "i16",
                IntegerWidth::W32 => "i32",
                IntegerWidth::W64 => "i64",
                IntegerWidth::W128 => "i128",
                IntegerWidth::Size => "isize",
            },
            NumericLiteralType::Unsigned(width) => match width {
                IntegerWidth::W8 => "u8",
                IntegerWidth::W16 => "u16",
                IntegerWidth::W32 => "u32",
                IntegerWidth::W64 => "u64",
                IntegerWidth::W128 => "u128",
                IntegerWidth::Size => "usize",
            },
            NumericLiteralType::Float16 => "float16",
            NumericLiteralType::Float32 => "float",
            NumericLiteralType::Float64 => "double",
            NumericLiteralType::Float128 => "float128",
            NumericLiteralType::Decimal => "decimal",
        }
    }

    fn signed_bounds(width: IntegerWidth) -> Option<(i128, i128)> {
        match width {
            IntegerWidth::W8 => Some((i8::MIN as i128, i8::MAX as i128)),
            IntegerWidth::W16 => Some((i16::MIN as i128, i16::MAX as i128)),
            IntegerWidth::W32 => Some((i32::MIN as i128, i32::MAX as i128)),
            IntegerWidth::W64 => Some((i64::MIN as i128, i64::MAX as i128)),
            IntegerWidth::W128 => Some((i128::MIN, i128::MAX)),
            IntegerWidth::Size => None,
        }
    }

    fn unsigned_max(width: IntegerWidth) -> Option<u128> {
        match width {
            IntegerWidth::W8 => Some(u8::MAX as u128),
            IntegerWidth::W16 => Some(u16::MAX as u128),
            IntegerWidth::W32 => Some(u32::MAX as u128),
            IntegerWidth::W64 => Some(u64::MAX as u128),
            IntegerWidth::W128 => Some(u128::MAX),
            IntegerWidth::Size => None,
        }
    }

    fn signed_aliases(width: IntegerWidth) -> &'static [&'static str] {
        match width {
            IntegerWidth::W8 => &["sbyte", "i8", "system::sbyte"],
            IntegerWidth::W16 => &["short", "i16", "int16", "system::int16"],
            IntegerWidth::W32 => &["int", "i32", "int32", "system::int32"],
            IntegerWidth::W64 => &["long", "i64", "int64", "system::int64"],
            IntegerWidth::W128 => &["i128", "int128", "system::int128"],
            IntegerWidth::Size => &["isize", "nint", "intptr", "system::intptr"],
        }
    }

    fn unsigned_aliases(width: IntegerWidth) -> &'static [&'static str] {
        match width {
            IntegerWidth::W8 => &["byte", "u8", "system::byte"],
            IntegerWidth::W16 => &["ushort", "u16", "uint16", "system::uint16"],
            IntegerWidth::W32 => &["uint", "u32", "uint32", "system::uint32"],
            IntegerWidth::W64 => &["ulong", "u64", "uint64", "system::uint64"],
            IntegerWidth::W128 => &["u128", "uint128", "system::uint128"],
            IntegerWidth::Size => &["usize", "nuint", "uintptr", "system::uintptr"],
        }
    }
}
