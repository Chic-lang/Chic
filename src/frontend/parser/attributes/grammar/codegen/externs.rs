use super::*;
use crate::frontend::ast::ExternBinding;
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Token, TokenKind};
use crate::frontend::parser::attributes::ParsedAttributeValue;
use crate::frontend::parser::attributes::flags::ParsedExternSpec;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_extern_attribute(
        &mut self,
        attr_span: Option<Span>,
    ) -> ParsedExternSpec {
        let mut spec = ParsedExternSpec {
            span: attr_span,
            ..Default::default()
        };

        if !self.consume_punctuation('(') {
            if let Some((value, span)) = self.expect_attribute_string_literal("extern") {
                if self.validate_abi(&value, Some(span)) {
                    spec.convention = Some(value);
                }
            }
            return spec;
        }

        if self.consume_punctuation(')') {
            return spec;
        }

        if matches!(self.peek(), Some(Token { kind: TokenKind::StringLiteral(_), .. })) {
            match self.expect_attribute_string_literal("extern") {
                Some((value, span)) => {
                    if self.validate_abi(&value, Some(span)) {
                        spec.convention = Some(value);
                    }
                }
                None => {
                    self.skip_balanced('(', ')');
                    return spec;
                }
            }
            if self.consume_punctuation(')') {
                return spec;
            }
            if !self.consume_punctuation(',') {
                self.push_error(
                    "expected ',' or ')' after convention in `@extern` attribute",
                    self.peek().map(|t| t.span),
                );
                self.skip_balanced('(', ')');
                return spec;
            }
            if self.consume_punctuation(')') {
                return spec;
            }
        }

        loop {
            let Some((key, key_span)) = self.parse_attribute_name() else {
                self.skip_balanced('(', ')');
                return spec;
            };
            if !self.consume_operator("=") {
                self.push_error(
                    format!("expected '=' after `{key}` in `@extern` attribute"),
                    self.peek().map(|t| t.span).or(Some(key_span)),
                );
                self.skip_balanced('(', ')');
                return spec;
            }
            let key_lower = key.to_ascii_lowercase();
            let Some((value, value_span)) =
                self.parse_attribute_value("extern", &key_lower)
            else {
                self.skip_balanced('(', ')');
                return spec;
            };
            let span = Span::in_file(key_span.file_id, key_span.start, value_span.end);
            self.apply_extern_argument(&mut spec, &key_lower, value, span);

            if self.consume_punctuation(')') {
                break;
            }
            if !self.consume_punctuation(',') {
                self.push_error(
                    "expected ',' or ')' in `@extern` attribute",
                    self.peek().map(|t| t.span),
                );
                self.skip_balanced('(', ')');
                break;
            }
        }

        spec
    }

    pub(in crate::frontend::parser) fn validate_abi(&mut self, abi: &str, span: Option<Span>) -> bool {
        const SUPPORTED: &[&str] = &[
            "c",
            "cdecl",
            "sysv64",
            "aapcs",
            "system",
            "stdcall",
            "fastcall",
            "vectorcall",
        ];
        if SUPPORTED
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(abi))
        {
            true
        } else {
            self.push_error(
                format!(
                    "unsupported ABI `{abi}`; supported ABIs: C, cdecl, sysv64, aapcs, system, stdcall, fastcall, vectorcall"
                ),
                span,
            );
            false
        }
    }

    pub(in crate::frontend::parser) fn apply_extern_argument(
        &mut self,
        spec: &mut ParsedExternSpec,
        key: &str,
        value: ParsedAttributeValue,
        span: Span,
    ) {
        match key {
            "convention" => match value {
                ParsedAttributeValue::Str(text) => {
                    if self.validate_abi(&text, Some(span)) {
                        if spec.convention.is_some() {
                            self.push_error(
                                "duplicate `convention` argument in `@extern` attribute",
                                Some(span),
                            );
                        } else {
                            spec.convention = Some(text);
                        }
                    }
                }
                _ => self.push_error(
                    "`convention` argument for `@extern` expects a string literal",
                    Some(span),
                ),
            },
            "library" => match value {
                ParsedAttributeValue::Str(text) => {
                    if spec.library.is_some() {
                        self.push_error(
                            "duplicate `library` argument in `@extern` attribute",
                            Some(span),
                        );
                    } else {
                        spec.library = Some(text);
                    }
                }
                _ => self.push_error(
                    "`library` argument for `@extern` expects a string literal",
                    Some(span),
                ),
            },
            "alias" => match value {
                ParsedAttributeValue::Str(text) => {
                    if spec.alias.is_some() {
                        self.push_error(
                            "duplicate `alias` argument in `@extern` attribute",
                            Some(span),
                        );
                    } else {
                        spec.alias = Some(text);
                    }
                }
                _ => self.push_error(
                    "`alias` argument for `@extern` expects a string literal",
                    Some(span),
                ),
            },
            "binding" => match value {
                ParsedAttributeValue::Str(text) => {
                    if let Some(mode) = self.parse_extern_binding(&text, span) {
                        if spec.binding.is_some() {
                            self.push_error(
                                "duplicate `binding` argument in `@extern` attribute",
                                Some(span),
                            );
                        } else {
                            spec.binding = Some(mode);
                        }
                    }
                }
                _ => self.push_error(
                    "`binding` argument for `@extern` expects a string literal",
                    Some(span),
                ),
            },
            "optional" => match value {
                ParsedAttributeValue::Bool(value) => spec.optional = Some(value),
                ParsedAttributeValue::Str(text) => {
                    if let Some(value) = parse_bool_literal(&text) {
                        spec.optional = Some(value);
                    } else {
                        self.push_error(
                            "`optional` argument for `@extern` expects `true` or `false`",
                            Some(span),
                        );
                    }
                }
                ParsedAttributeValue::Int(_) => self.push_error(
                    "`optional` argument for `@extern` expects `true` or `false`",
                    Some(span),
                ),
            },
            "charset" => match value {
                ParsedAttributeValue::Str(text) => {
                    if spec.charset.is_some() {
                        self.push_error(
                            "duplicate `charset` argument in `@extern` attribute",
                            Some(span),
                        );
                    } else {
                        spec.charset = Some(text);
                    }
                }
                _ => self.push_error(
                    "`charset` argument for `@extern` expects a string literal",
                    Some(span),
                ),
            },
            other => self.push_error(
                format!("unknown argument `{other}` in `@extern` attribute"),
                Some(span),
            ),
        }
    }

    fn parse_extern_binding(&mut self, text: &str, span: Span) -> Option<ExternBinding> {
        let lowered = text.to_ascii_lowercase();
        let binding = match lowered.as_str() {
            "lazy" => ExternBinding::Lazy,
            "eager" => ExternBinding::Eager,
            "static" => ExternBinding::Static,
            _ => {
                self.push_error(
                    "binding must be `lazy`, `eager`, or `static` in `@extern` attribute",
                    Some(span),
                );
                return None;
            }
        };
        Some(binding)
    }
}

fn parse_bool_literal(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn parse_extern_attribute_reports_unknown_argument() {
        let mut parser = parser_for("(foo = \"bar\")");
        let _ = parser.parse_extern_attribute(None);
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("unknown argument")),
            "expected unknown argument diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_extern_attribute_validates_abi_names() {
        let mut parser = parser_for("(\"invalid\")");
        let _ = parser.parse_extern_attribute(None);
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("unsupported ABI")),
            "expected unsupported ABI diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_extern_attribute_rejects_duplicate_convention() {
        let mut parser = parser_for("(convention = \"c\", convention = \"c\")");
        let _ = parser.parse_extern_attribute(None);
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `convention`")),
            "expected duplicate convention diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_extern_attribute_handles_shorthand_string() {
        let mut parser = parser_for("\"C\"");
        let spec = parser.parse_extern_attribute(None);
        assert_eq!(spec.convention.as_deref(), Some("C"));
    }

    #[test]
    fn parse_extern_attribute_reports_missing_comma() {
        let mut parser = parser_for("(\"C\" \"extra\")");
        let _ = parser.parse_extern_attribute(None);
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expected ','")),
            "expected missing comma diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn parse_extern_attribute_parses_key_value_arguments() {
        let mut parser = parser_for(
            "(binding = \"lazy\", optional = \"true\", charset = \"utf8\", alias = \"foo\")",
        );
        let spec = parser.parse_extern_attribute(None);
        assert!(matches!(spec.binding, Some(ExternBinding::Lazy)));
        assert_eq!(spec.optional, Some(true));
        assert_eq!(spec.charset.as_deref(), Some("utf8"));
        assert_eq!(spec.alias.as_deref(), Some("foo"));
    }

    #[test]
    fn parse_extern_attribute_reports_invalid_binding() {
        let mut parser = parser_for("(binding = \"invalid\")");
        let _ = parser.parse_extern_attribute(None);
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics.iter().any(|diag| diag
                .message
                .contains("binding must be `lazy`, `eager`, or `static`")),
            "expected binding diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_extern_attribute_handles_bool_literal_optional() {
        let mut parser = parser_for("(optional = false)");
        let spec = parser.parse_extern_attribute(None);
        assert_eq!(spec.optional, Some(false));
    }

    #[test]
    fn apply_extern_argument_updates_spec_for_all_keys() {
        let mut parser = parser_for("");
        let mut spec = ParsedExternSpec::default();
        let span = Span::new(0, 1);

        parser.apply_extern_argument(
            &mut spec,
            "convention",
            ParsedAttributeValue::Str("c".into()),
            span,
        );
        parser.apply_extern_argument(
            &mut spec,
            "library",
            ParsedAttributeValue::Str("m".into()),
            span,
        );
        parser.apply_extern_argument(
            &mut spec,
            "alias",
            ParsedAttributeValue::Str("alias".into()),
            span,
        );
        parser.apply_extern_argument(
            &mut spec,
            "binding",
            ParsedAttributeValue::Str("lazy".into()),
            span,
        );
        parser.apply_extern_argument(
            &mut spec,
            "optional",
            ParsedAttributeValue::Bool(true),
            span,
        );
        parser.apply_extern_argument(
            &mut spec,
            "charset",
            ParsedAttributeValue::Str("utf8".into()),
            span,
        );

        assert_eq!(spec.convention.as_deref(), Some("c"));
        assert_eq!(spec.library.as_deref(), Some("m"));
        assert_eq!(spec.alias.as_deref(), Some("alias"));
        assert!(matches!(spec.binding, Some(ExternBinding::Lazy)));
        assert_eq!(spec.optional, Some(true));
        assert_eq!(spec.charset.as_deref(), Some("utf8"));
    }
}
