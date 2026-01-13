use super::{StringLiteralContents, StringLiteralKind, StringSegment, parse_string_literal};
use crate::frontend::literals::{LiteralError, LiteralErrorKind};

fn assert_no_errors(errors: &[LiteralError]) {
    if !errors.is_empty() {
        panic!("unexpected literal errors: {errors:?}");
    }
}

#[test]
fn parses_regular_unicode_escape() {
    let (literal, errors) = parse_string_literal("\\u0041", StringLiteralKind::Regular);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Simple(text) => assert_eq!(text, "A"),
        other => panic!("expected simple contents, found {other:?}"),
    }
}

#[test]
fn normalizes_regular_string_literal_to_nfc() {
    let (literal, errors) = parse_string_literal("A\u{030A}", StringLiteralKind::Regular);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Simple(text) => assert_eq!(text, "Å"),
        other => panic!("expected simple contents, found {other:?}"),
    }
}

#[test]
fn parses_verbatim_with_newline() {
    let input = "Line1\nLine2";
    let (literal, errors) = parse_string_literal(input, StringLiteralKind::Verbatim);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Simple(text) => assert_eq!(text, "Line1\nLine2"),
        _ => panic!("expected simple contents"),
    }
}

#[test]
fn parses_interpolated_segments() {
    let (literal, errors) = parse_string_literal("Hello {name}!", StringLiteralKind::Interpolated);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Interpolated(segments) => {
            assert_eq!(segments.len(), 3);
            match &segments[0] {
                StringSegment::Text(text) => assert_eq!(text, "Hello "),
                other => panic!("expected text segment, found {other:?}"),
            }
            match &segments[1] {
                StringSegment::Interpolation(segment) => {
                    assert_eq!(segment.expression, "name");
                    assert_eq!(segment.alignment, None);
                    assert_eq!(segment.format, None);
                }
                other => panic!("expected interpolation segment, found {other:?}"),
            }
            match &segments[2] {
                StringSegment::Text(text) => assert_eq!(text, "!"),
                other => panic!("expected text segment, found {other:?}"),
            }
        }
        other => panic!("expected interpolated contents, found {other:?}"),
    }
}

#[test]
fn normalizes_interpolated_text_segments() {
    let (literal, errors) =
        parse_string_literal("Hello A\u{030A} {name}", StringLiteralKind::Interpolated);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Interpolated(segments) => match &segments[0] {
            StringSegment::Text(text) => assert_eq!(text, "Hello Å "),
            other => panic!("expected text segment, found {other:?}"),
        },
        other => panic!("expected interpolated contents, found {other:?}"),
    }
}

#[test]
fn parses_interpolated_alignment_and_format() {
    let (literal, errors) = parse_string_literal("{value,10:00}", StringLiteralKind::Interpolated);
    assert_no_errors(&errors);
    match literal.contents {
        StringLiteralContents::Interpolated(segments) => {
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                StringSegment::Interpolation(segment) => {
                    assert_eq!(segment.expression, "value");
                    assert_eq!(segment.alignment, Some(10));
                    assert_eq!(segment.format.as_deref(), Some("00"));
                }
                other => panic!("expected interpolation segment, found {other:?}"),
            }
        }
        _ => panic!("expected interpolated contents"),
    }
}

#[test]
fn reports_unmatched_closing_brace() {
    let (_, errors) = parse_string_literal("Text }", StringLiteralKind::Interpolated);
    assert!(
        errors
            .iter()
            .any(|err| matches!(err.kind, LiteralErrorKind::UnmatchedClosingBrace))
    );
}

#[test]
fn reports_unterminated_interpolation() {
    let (_, errors) = parse_string_literal("{value", StringLiteralKind::Interpolated);
    assert!(
        errors
            .iter()
            .any(|err| matches!(err.kind, LiteralErrorKind::UnterminatedInterpolation))
    );
}
