//! Builtin `@conditional` attribute marker.

use super::super::CollectedAttributes;
use super::*;
use crate::frontend::diagnostics::Span;

pub(super) fn handle_conditional_attribute(
    parser: &mut Parser,
    lowered: &str,
    span: Span,
    _attrs: &mut CollectedAttributes,
) -> bool {
    if lowered != "conditional" {
        return false;
    }
    let _ = parser.parse_attribute_string_argument("conditional", true);
    if parser.consume_punctuation('(') {
        parser.skip_balanced('(', ')');
    }
    let _ = span;
    true
}
