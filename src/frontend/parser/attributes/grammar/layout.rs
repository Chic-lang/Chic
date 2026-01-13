//! Parsers for layout-facing attributes (`@StructLayout`, `@mmio`, `@repr`,
//! and `@align`) plus supporting helpers for numeric arguments and layout kinds.

use super::super::CollectedAttributes;
use super::*;
use crate::frontend::attributes::{AlignHint, LayoutHints, PackingHint};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::TokenKind;

pub(super) fn handle_layout_attribute(
    parser: &mut Parser,
    lowered: &str,
    span: Span,
    attrs: &mut CollectedAttributes,
) -> bool {
    match lowered {
        "repr" | "align" => {
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        "mmio" => {
            if attrs.builtin.mmio_struct.is_some() {
                parser.push_error("duplicate `@mmio` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else if let Some(attr) = parser.parse_mmio_struct_attribute(Some(span)) {
                attrs.builtin.record_mmio_struct(attr, Some(span));
            }
            true
        }
        "structlayout" => {
            if attrs.builtin.struct_layout.is_some() {
                parser.push_error("duplicate `@StructLayout` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else if let Some(layout) = parser.parse_struct_layout_attribute(Some(span)) {
                attrs.builtin.record_struct_layout(layout, Some(span));
            }
            true
        }
        _ => false,
    }
}

parser_impl! {
    pub(in crate::frontend::parser) fn parse_struct_layout_attribute(
        &mut self,
        attr_span: Option<Span>,
    ) -> Option<LayoutHints> {
        if !self.consume_punctuation('(') {
            self.push_error(
                "`@StructLayout` attribute expects parentheses with arguments",
                attr_span,
            );
            return None;
        }

        let Some((kind_name, kind_span)) = self.parse_struct_layout_kind_value() else {
            self.skip_balanced('(', ')');
            return None;
        };

        let mut hints = LayoutHints::default();
        hints.repr_c = self.apply_struct_layout_kind(&kind_name, kind_span.or(attr_span));

        if self.consume_punctuation(')') {
            return Some(hints);
        }

        if !self.expect_punctuation(',') {
            self.skip_balanced('(', ')');
            return None;
        }

        loop {
            if self.consume_punctuation(')') {
                break;
            }

            let Some((key, key_span)) = self.parse_attribute_name() else {
                self.skip_balanced('(', ')');
                return None;
            };

            if !self.consume_operator("=") {
                self.push_error(
                    format!("expected '=' after `{key}` in `@StructLayout` attribute"),
                    self.peek().map(|t| t.span).or(Some(key_span)),
                );
                self.skip_balanced('(', ')');
                return None;
            }

            match key.to_ascii_lowercase().as_str() {
                "kind" => {
                    let Some((kind_override, span)) = self.parse_struct_layout_kind_value() else {
                        self.skip_balanced('(', ')');
                        return None;
                    };
                    hints.repr_c = self
                        .apply_struct_layout_kind(&kind_override, span.or(Some(key_span)));
                }
                "pack" => {
                    let Some((value, span)) =
                        self.parse_struct_layout_numeric_argument("pack")
                    else {
                        self.skip_balanced('(', ')');
                        return None;
                    };
                    if hints.packing.is_some() {
                        self.push_error(
                            "duplicate `Pack` argument in `@StructLayout` attribute",
                            Some(span),
                        );
                    } else {
                        hints.packing = Some(PackingHint {
                            value: Some(value),
                            span: Some(span),
                        });
                    }
                }
                "align" => {
                    let Some((value, span)) =
                        self.parse_struct_layout_numeric_argument("align")
                    else {
                        self.skip_balanced('(', ')');
                        return None;
                    };
                    if hints.align.is_some() {
                        self.push_error(
                            "duplicate `Align` argument in `@StructLayout` attribute",
                            Some(span),
                        );
                    } else {
                        hints.align = Some(AlignHint {
                            value,
                            span: Some(span),
                        });
                    }
                }
                other => {
                    self.push_error(
                        format!(
                            "unknown argument `{other}` in `@StructLayout` attribute"
                        ),
                        Some(key_span),
                    );
                    self.skip_balanced('(', ')');
                    return None;
                }
            }

            if self.consume_punctuation(')') {
                break;
            }

            if !self.expect_punctuation(',') {
                self.skip_balanced('(', ')');
                return None;
            }
        }

        Some(hints)
    }

    fn parse_struct_layout_kind_value(&mut self) -> Option<(String, Option<Span>)> {
        let start = self.index;
        let Some(token) = self.peek().cloned() else {
            self.push_error(
                "`@StructLayout` attribute requires a layout kind such as `LayoutKind.Sequential`",
                None,
            );
            return None;
        };
        if !matches!(token.kind, TokenKind::Identifier) {
            self.push_error(
                "expected layout kind identifier in `@StructLayout` attribute",
                Some(token.span),
            );
            return None;
        }
        let mut segments = vec![token.lexeme.clone()];
        self.advance();

        loop {
            let mut consumed = false;
            if self.consume_punctuation('.') {
                consumed = true;
            } else if self.consume_operator("::") || self.consume_double_colon() {
                consumed = true;
            }
            if !consumed {
                break;
            }

            let Some(next) = self.peek().cloned() else {
                self.push_error(
                    "expected identifier after layout kind separator",
                    None,
                );
                return None;
            };
            if !matches!(next.kind, TokenKind::Identifier) {
                self.push_error(
                    "expected identifier after layout kind separator",
                    Some(next.span),
                );
                return None;
            }
            segments.push(next.lexeme.clone());
            self.advance();
        }

        let span = self
            .span_from_indices(start, self.index)
            .or(Some(token.span));
        Some((segments.join("::"), span))
    }

    fn consume_double_colon(&mut self) -> bool {
        let Some(first) = self.peek() else { return false };
        if !matches!(first.kind, TokenKind::Punctuation(':')) {
            return false;
        }
        let Some(second) = self.tokens.get(self.index + 1) else {
            return false;
        };
        if !matches!(second.kind, TokenKind::Punctuation(':')) {
            return false;
        }
        self.advance();
        self.advance();
        true
    }

    fn parse_struct_layout_numeric_argument(
        &mut self,
        name: &str,
    ) -> Option<(u32, Span)> {
        let Some(token) = self.peek().cloned() else {
            self.push_error(
                format!(
                    "expected integer literal for `{name}` in `@StructLayout` attribute"
                ),
                None,
            );
            return None;
        };

        match token.kind {
            TokenKind::NumberLiteral(_) => {
                let normalized: String =
                    token.lexeme.chars().filter(|ch| *ch != '_').collect();
                match normalized.parse::<u32>() {
                    Ok(value) => {
                        self.advance();
                        Some((value, token.span))
                    }
                    Err(_) => {
                        self.push_error(
                            format!(
                                "`{name}` value `{}` in `@StructLayout` attribute is not a valid `u32`",
                                token.lexeme
                            ),
                            Some(token.span),
                        );
                        self.advance();
                        None
                    }
                }
            }
            _ => {
                self.push_error(
                    format!(
                        "expected integer literal for `{name}` in `@StructLayout` attribute"
                    ),
                    Some(token.span),
                );
                None
            }
        }
    }

    fn apply_struct_layout_kind(&mut self, name: &str, span: Option<Span>) -> bool {
        let simple = name
            .rsplit("::")
            .next()
            .unwrap_or(name)
            .to_ascii_lowercase();
        match simple.as_str() {
            "sequential" => true,
            _other => {
                self.push_error(
                    format!(
                        "unsupported `@StructLayout` kind `{}`; only `LayoutKind.Sequential` is supported",
                        name
                    ),
                    span,
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::handle_layout_attribute;
    use crate::frontend::diagnostics::Span;
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::attributes::CollectedAttributes;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn struct_layout_requires_parentheses() {
        let mut parser = parser_for(" ");
        assert!(
            parser.parse_struct_layout_attribute(None).is_none(),
            "expected parsing to fail without parentheses"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expects parentheses")),
            "expected missing parentheses diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn struct_layout_reports_unknown_argument() {
        let mut parser = parser_for("(LayoutKind.Sequential, Foo = 1)");
        assert!(
            parser.parse_struct_layout_attribute(None).is_none(),
            "expected parsing to fail for unknown argument"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("unknown argument `foo`")),
            "expected unknown argument diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn struct_layout_reports_duplicate_pack() {
        let mut parser = parser_for("(LayoutKind.Sequential, Pack = 4, Pack = 8)");
        let result = parser
            .parse_struct_layout_attribute(None)
            .expect("expected layout result");
        assert!(result.packing.is_some(), "expected packing hint");
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `Pack` argument")),
            "expected duplicate pack diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn handle_layout_attribute_records_mmio_struct() {
        let mut parser = parser_for("(base = 0, size = 4)");
        let mut attrs = CollectedAttributes::default();
        assert!(handle_layout_attribute(
            &mut parser,
            "mmio",
            Span::new(0, 0),
            &mut attrs
        ));
        assert!(attrs.builtin.mmio_struct.is_some(), "expected mmio struct");
    }

    #[test]
    fn handle_layout_attribute_handles_align() {
        let mut parser = parser_for("(16)");
        let mut attrs = CollectedAttributes::default();
        assert!(handle_layout_attribute(
            &mut parser,
            "align",
            Span::new(0, 0),
            &mut attrs
        ));
        assert!(attrs.builtin.struct_layout.is_none());
    }

    #[test]
    fn struct_layout_pack_requires_integer_literal() {
        let mut parser = parser_for("(LayoutKind.Sequential, Pack = foo)");
        assert!(
            parser.parse_struct_layout_attribute(None).is_none(),
            "expected invalid pack to fail"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expected integer literal for `pack`")),
            "expected pack diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn struct_layout_align_requires_integer_literal() {
        let mut parser = parser_for("(LayoutKind.Sequential, Align = foo)");
        assert!(
            parser.parse_struct_layout_attribute(None).is_none(),
            "expected invalid align to fail"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics.iter().any(|diag| diag
                .message
                .contains("expected integer literal for `align`")),
            "expected align diagnostic: {diagnostics:?}"
        );
    }
}
