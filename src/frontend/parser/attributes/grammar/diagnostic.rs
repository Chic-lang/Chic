//! Diagnostic/marker attributes such as thread-safety toggles, copyability, and
//! fallibility markers.

use super::super::CollectedAttributes;
use super::*;
use crate::frontend::diagnostics::Span;

pub(super) fn handle_diagnostic_attribute(
    parser: &mut Parser,
    lowered: &str,
    span: Span,
    attrs: &mut CollectedAttributes,
) -> bool {
    match lowered {
        "pin" => {
            if attrs.builtin.pin {
                parser.push_error("duplicate `@pin` attribute", Some(span));
            }
            attrs.builtin.record_pin(Some(span));
            true
        }
        "thread_safe" | "threadsafe" => {
            record_flag(
                parser,
                span,
                attrs.builtin.thread_safe,
                "`@thread_safe` attribute is repeated",
                "conflicting thread-safety attributes (`@thread_safe` vs `@not_thread_safe`)",
                Some(true),
            );
            if attrs.builtin.thread_safe.is_none() {
                attrs.builtin.record_thread_safe(true, Some(span));
            }
            true
        }
        "not_thread_safe" | "notthreadsafe" => {
            record_flag(
                parser,
                span,
                attrs.builtin.thread_safe,
                "`@not_thread_safe` attribute is repeated",
                "conflicting thread-safety attributes (`@thread_safe` vs `@not_thread_safe`)",
                Some(false),
            );
            if attrs.builtin.thread_safe.is_none() {
                attrs.builtin.record_thread_safe(false, Some(span));
            }
            true
        }
        "shareable" => {
            record_flag(
                parser,
                span,
                attrs.builtin.shareable,
                "`@shareable` attribute is repeated",
                "conflicting shareability attributes (`@shareable` vs `@not_shareable`)",
                Some(true),
            );
            if attrs.builtin.shareable.is_none() {
                attrs.builtin.record_shareable(true, Some(span));
            }
            true
        }
        "not_shareable" | "notshareable" => {
            record_flag(
                parser,
                span,
                attrs.builtin.shareable,
                "`@not_shareable` attribute is repeated",
                "conflicting shareability attributes (`@shareable` vs `@not_shareable`)",
                Some(false),
            );
            if attrs.builtin.shareable.is_none() {
                attrs.builtin.record_shareable(false, Some(span));
            }
            true
        }
        "copy" => {
            record_flag(
                parser,
                span,
                attrs.builtin.copy,
                "`@copy` attribute is repeated",
                "conflicting copy semantics (`@copy` vs `@not_copy`)",
                Some(true),
            );
            if attrs.builtin.copy.is_none() {
                attrs.builtin.record_copy(true, Some(span));
            }
            true
        }
        "not_copy" | "notcopy" => {
            record_flag(
                parser,
                span,
                attrs.builtin.copy,
                "`@not_copy` attribute is repeated",
                "conflicting copy semantics (`@copy` vs `@not_copy`)",
                Some(false),
            );
            if attrs.builtin.copy.is_none() {
                attrs.builtin.record_copy(false, Some(span));
            }
            true
        }
        "flags" => {
            if attrs.builtin.flags {
                parser.push_error("duplicate `@flags` attribute", Some(span));
            }
            attrs.builtin.record_flags(Some(span));
            true
        }
        "fallible" => {
            if attrs.builtin.fallible {
                parser.push_error("duplicate `@fallible` attribute", Some(span));
            }
            if parser.consume_punctuation('(') {
                parser.push_error(
                    "`@fallible` attribute does not accept arguments",
                    Some(span),
                );
                parser.skip_balanced('(', ')');
            }
            attrs.builtin.record_fallible(Some(span));
            true
        }
        _ => false,
    }
}

fn record_flag(
    parser: &mut Parser,
    span: Span,
    existing: Option<bool>,
    duplicate_msg: &str,
    conflict_msg: &str,
    incoming: Option<bool>,
) {
    if let Some(current) = existing {
        if Some(current) == incoming {
            parser.push_error(duplicate_msg, Some(span));
        } else {
            parser.push_error(conflict_msg, Some(span));
        }
    }
}
