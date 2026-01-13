use super::state::Lexer;
use crate::diagnostics::Suggestion;
use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::frontend::literals::{LiteralError, LiteralErrorKind, StringLiteralKind};
use crate::string_support::diagnostics::{LiteralDiagnosticKind, literal_message};

pub(super) fn report_simple_error(lexer: &mut Lexer<'_>, message: &str, span: Span) {
    lexer
        .diagnostics
        .push(Diagnostic::error(message.to_string(), Some(span)));
}

impl<'a> Lexer<'a> {
    pub(super) fn report_literal_errors(
        &mut self,
        literal_start: usize,
        content_start: usize,
        literal_kind: Option<StringLiteralKind>,
        errors: Vec<LiteralError>,
    ) {
        for error in errors {
            let mut span_start = content_start.saturating_add(error.offset);
            let mut span_end = if error.length == 0 {
                span_start
            } else {
                span_start.saturating_add(error.length)
            };
            if matches!(error.kind, LiteralErrorKind::EmptyCharLiteral) {
                span_start = literal_start;
                span_end = content_start;
            }
            span_end = span_end.min(self.source.len());
            let message =
                literal_message(&to_literal_diagnostic(&error.kind, literal_kind)).into_owned();
            self.diagnostics.push(Diagnostic::error(
                message,
                Some(Span::new(span_start, span_end)),
            ));
        }
    }
}

pub(super) fn report_invalid_identifier_char(
    lexer: &mut Lexer<'_>,
    offset: usize,
    ch: char,
    property: &str,
    suggestion: Option<&str>,
) {
    let span = Span::in_file(lexer.file_id, offset, offset + ch.len_utf8());
    let message = format!(
        "code point U+{ch:04X} is not permitted in identifiers ({property})",
        ch = ch as u32
    );
    let mut diagnostic = Diagnostic::error(message, Some(span));
    if let Some(suggested) = suggestion {
        diagnostic.add_suggestion(Suggestion::new(
            format!("replace with `{suggested}`"),
            Some(span),
            Some(suggested.to_string()),
        ));
    }
    lexer.diagnostics.push(diagnostic);
}

pub(super) fn report_identifier_not_normalized(
    lexer: &mut Lexer<'_>,
    span: Span,
    normalized: &str,
) {
    let mut diagnostic = Diagnostic::error("identifier is not NFC-normalised", Some(span))
        .with_primary_label("identifier must use NFC form");
    diagnostic.add_suggestion(Suggestion::new(
        format!("replace with `{normalized}`"),
        Some(span),
        Some(normalized.to_string()),
    ));
    lexer.diagnostics.push(diagnostic);
}

fn to_literal_diagnostic(
    kind: &LiteralErrorKind,
    literal_kind: Option<StringLiteralKind>,
) -> LiteralDiagnosticKind {
    let verbatim = matches!(
        literal_kind,
        Some(StringLiteralKind::Verbatim | StringLiteralKind::InterpolatedVerbatim)
    );
    match kind {
        LiteralErrorKind::UnexpectedEnd => LiteralDiagnosticKind::UnexpectedEnd,
        LiteralErrorKind::InvalidEscape(ch) => {
            LiteralDiagnosticKind::InvalidEscape { ch: *ch, verbatim }
        }
        LiteralErrorKind::InvalidHexDigit(ch) => LiteralDiagnosticKind::InvalidHexDigit { ch: *ch },
        LiteralErrorKind::InvalidUnicodeLen { expected, actual } => {
            LiteralDiagnosticKind::InvalidUnicodeLength {
                expected: *expected,
                actual: *actual,
            }
        }
        LiteralErrorKind::InvalidCodepoint(cp) => {
            LiteralDiagnosticKind::InvalidCodepoint { codepoint: *cp }
        }
        LiteralErrorKind::MissingHexDigits => LiteralDiagnosticKind::MissingHexDigits,
        LiteralErrorKind::NewlineInLiteral => LiteralDiagnosticKind::NewlineInLiteral,
        LiteralErrorKind::EmptyCharLiteral => LiteralDiagnosticKind::EmptyCharLiteral,
        LiteralErrorKind::TooManyCharacters => LiteralDiagnosticKind::TooManyCharacters,
        LiteralErrorKind::UnmatchedClosingBrace => LiteralDiagnosticKind::UnmatchedClosingBrace,
        LiteralErrorKind::UnterminatedInterpolation => {
            LiteralDiagnosticKind::UnterminatedInterpolation
        }
        LiteralErrorKind::EmptyInterpolation => LiteralDiagnosticKind::EmptyInterpolation,
        LiteralErrorKind::InvalidAlignment => LiteralDiagnosticKind::InvalidAlignment,
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use super::*;
    use crate::frontend::diagnostics::FileId;
    use crate::frontend::literals::{LiteralError, LiteralErrorKind};

    #[test]
    fn reports_invalid_escape_sequence() {
        let mut lexer = Lexer::new(r#""\q""#, FileId(0));
        report_simple_error(&mut lexer, "unterminated string literal", Span::new(0, 0));
        lexer.report_literal_errors(
            0,
            1,
            Some(StringLiteralKind::Regular),
            vec![LiteralError {
                offset: 1,
                length: 1,
                kind: LiteralErrorKind::InvalidEscape('q'),
            }],
        );
        let diagnostics = lexer.diagnostics.into_vec();
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics[1]
                .message
                .contains("unknown escape sequence `\\q`")
        );
    }
}
