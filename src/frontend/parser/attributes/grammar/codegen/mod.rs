//! Code generation attributes (`@extern`, `@link`, `@inline`, `@vectorize`,
//! etc.) and their helpers.

use super::super::CollectedAttributes;
use super::*;
use crate::frontend::diagnostics::Span;

mod externs;
mod inline_attr;
mod strings;
mod vectorize;

pub(super) fn handle_codegen_attribute(
    parser: &mut Parser,
    lowered: &str,
    span: Span,
    attrs: &mut CollectedAttributes,
) -> bool {
    match lowered {
        "hot" | "cold" | "always_inline" | "alwaysinline" | "never_inline" | "neverinline" => {
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        "extern" => {
            if attrs.builtin.extern_attr {
                parser.push_error("duplicate `@extern` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else {
                let spec = parser.parse_extern_attribute(Some(span));
                attrs.builtin.record_extern(spec, Some(span));
            }
            true
        }
        "link" => {
            if attrs.builtin.link_library.is_some() {
                parser.push_error("duplicate `@link` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else if let Some((library, maybe_span)) =
                parser.parse_attribute_string_argument("link", true)
            {
                if library.is_empty() {
                    parser.push_error(
                        "`@link` attribute requires a non-empty library name",
                        maybe_span.or(Some(span)),
                    );
                } else {
                    attrs
                        .builtin
                        .record_link(library, maybe_span.or(Some(span)));
                }
            }
            true
        }
        "cimport" => {
            if let Some((header, span_override)) =
                parser.parse_attribute_string_argument("cimport", true)
            {
                attrs
                    .builtin
                    .push_c_import(header, span_override.or(Some(span)));
            }
            true
        }
        "friend" => {
            if let Some((prefix, span_override)) =
                parser.parse_attribute_string_argument("friend", true)
            {
                let directive_span = span_override
                    .map(|s| Span {
                        file_id: s.file_id,
                        start: span.start,
                        end: s.end.saturating_add(1),
                    })
                    .or(Some(span));
                attrs.builtin.push_friend_namespace(prefix, directive_span);
            }
            true
        }
        "package" => {
            if let Some((name, span_override)) =
                parser.parse_attribute_string_argument("package", true)
            {
                let directive_span = span_override
                    .map(|s| Span {
                        file_id: s.file_id,
                        start: span.start,
                        end: s.end.saturating_add(1),
                    })
                    .or(Some(span));
                attrs.builtin.push_package_import(name, directive_span);
            }
            true
        }
        "export" => {
            let _ = parser.parse_attribute_string_argument("export", true);
            true
        }
        "no_std" | "nostd" => {
            parser.push_error(
                "`@no_std` is not supported; use the crate attribute `#![no_std]`",
                Some(span),
            );
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        "global_allocator" | "globalallocator" => {
            parser.push_error(
                "`@global_allocator` is not supported; configure allocators via the runtime/manifest",
                Some(span),
            );
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        "inline" => {
            if attrs.builtin.inline_attr.is_some() {
                parser.push_error("duplicate `@inline` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else if let Some(attr) = parser.parse_inline_attribute(Some(span)) {
                attrs.builtin.record_inline(attr, Some(span));
            }
            true
        }
        "intrinsic" => {
            if attrs.builtin.intrinsic {
                parser.push_error("duplicate `@Intrinsic` attribute", Some(span));
                if parser.consume_punctuation('(') {
                    parser.skip_balanced('(', ')');
                }
            } else {
                if parser.consume_punctuation('(') && !parser.expect_punctuation(')') {
                    parser.skip_balanced('(', ')');
                }
                attrs.builtin.record_intrinsic(Some(span));
            }
            true
        }
        "primitive" => {
            // `@primitive(...)` is a built-in attribute consumed later during header/lowering, but
            // it must not be treated as a macro-expansion attribute.
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        "vectorize" => {
            let hint = parser.parse_vectorize_attribute(Some(span));
            if attrs.builtin.vectorize_hint.is_some() {
                parser.push_error("duplicate `@vectorize` attribute", Some(span));
            } else if let Some(hint) = hint {
                attrs.builtin.record_vectorize(hint, Some(span));
            }
            true
        }
        "weak" | "weak_import" | "weakimport" => {
            if parser.consume_punctuation('(') {
                parser.skip_balanced('(', ')');
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Span;
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::attributes::CollectedAttributes;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn handle_codegen_attribute_sets_link_library() {
        let mut attrs = CollectedAttributes::default();
        let mut parser = parser_for("(\"math\")");
        assert!(handle_codegen_attribute(
            &mut parser,
            "link",
            Span::new(0, 0),
            &mut attrs
        ));
        assert_eq!(attrs.builtin.link_library.as_deref(), Some("math"));
    }

    #[test]
    fn handle_codegen_attribute_reports_duplicate_extern() {
        let mut attrs = CollectedAttributes::default();
        let mut parser = parser_for("(\"C\")");
        assert!(handle_codegen_attribute(
            &mut parser,
            "extern",
            Span::new(0, 0),
            &mut attrs
        ));
        let mut parser = parser_for("(\"C\")");
        assert!(handle_codegen_attribute(
            &mut parser,
            "extern",
            Span::new(0, 0),
            &mut attrs
        ));
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `@extern` attribute")),
            "expected duplicate extern diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn handle_codegen_attribute_reports_duplicate_link() {
        let mut attrs = CollectedAttributes::default();
        attrs
            .builtin
            .record_link("math".into(), Some(Span::new(0, 0)));
        let mut parser = parser_for("(\"m\")");
        assert!(handle_codegen_attribute(
            &mut parser,
            "link",
            Span::new(0, 0),
            &mut attrs
        ));
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `@link` attribute")),
            "expected duplicate link diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn handle_codegen_attribute_reports_duplicate_inline() {
        let mut attrs = CollectedAttributes::default();
        let mut parser = parser_for("(local)");
        assert!(handle_codegen_attribute(
            &mut parser,
            "inline",
            Span::new(0, 0),
            &mut attrs
        ));
        let mut parser = parser_for("(local)");
        assert!(handle_codegen_attribute(
            &mut parser,
            "inline",
            Span::new(0, 0),
            &mut attrs
        ));
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `@inline` attribute")),
            "expected duplicate inline diagnostic, found {diagnostics:?}"
        );
    }
}
