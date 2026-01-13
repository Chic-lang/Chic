use std::borrow::Cow;

/// Shared message kinds for literal diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralDiagnosticKind {
    UnexpectedEnd,
    InvalidEscape { ch: char, verbatim: bool },
    InvalidHexDigit { ch: char },
    InvalidUnicodeLength { expected: usize, actual: usize },
    InvalidCodepoint { codepoint: u32 },
    MissingHexDigits,
    NewlineInLiteral,
    EmptyCharLiteral,
    TooManyCharacters,
    UnmatchedClosingBrace,
    UnterminatedInterpolation,
    EmptyInterpolation,
    InvalidAlignment,
}

/// Render a literal diagnostic message consistent across frontend and runtime.
#[must_use]
pub fn literal_message(kind: &LiteralDiagnosticKind) -> Cow<'static, str> {
    match kind {
        LiteralDiagnosticKind::UnexpectedEnd => {
            Cow::Borrowed("unterminated escape sequence in literal")
        }
        LiteralDiagnosticKind::InvalidEscape { ch, verbatim } => {
            if *verbatim && *ch == '"' {
                Cow::Borrowed("verbatim strings use \"\" to represent a double quote")
            } else {
                Cow::Owned(format!("unknown escape sequence `\\{ch}`"))
            }
        }
        LiteralDiagnosticKind::InvalidHexDigit { ch } => {
            Cow::Owned(format!("invalid hexadecimal escape character `{ch}`"))
        }
        LiteralDiagnosticKind::InvalidUnicodeLength { expected, actual } => Cow::Owned(format!(
            "unicode escape expects {expected} hexadecimal digits but found {actual}"
        )),
        LiteralDiagnosticKind::InvalidCodepoint { codepoint } => {
            Cow::Owned(format!("invalid Unicode code point U+{codepoint:04X}"))
        }
        LiteralDiagnosticKind::MissingHexDigits => {
            Cow::Borrowed("hex escape sequence requires at least one hexadecimal digit")
        }
        LiteralDiagnosticKind::NewlineInLiteral => {
            Cow::Borrowed("newline not permitted in this literal")
        }
        LiteralDiagnosticKind::EmptyCharLiteral => {
            Cow::Borrowed("character literal must contain exactly one character")
        }
        LiteralDiagnosticKind::TooManyCharacters => {
            Cow::Borrowed("character literal may only contain a single character")
        }
        LiteralDiagnosticKind::UnmatchedClosingBrace => {
            Cow::Borrowed("found `}` without a matching `{` in interpolated string literal")
        }
        LiteralDiagnosticKind::UnterminatedInterpolation => {
            Cow::Borrowed("interpolated expression is missing a closing `}`")
        }
        LiteralDiagnosticKind::EmptyInterpolation => {
            Cow::Borrowed("interpolated expression cannot be empty")
        }
        LiteralDiagnosticKind::InvalidAlignment => {
            Cow::Borrowed("interpolated alignment component must be an integer literal")
        }
    }
}

/// Shared runtime diagnostic keys for string operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDiagnosticKind {
    InvalidPointer,
    Utf8,
    AllocationFailed,
    CapacityOverflow,
    OutOfBounds,
}

/// Render a runtime diagnostic message consistent across components.
#[must_use]
pub fn runtime_message(kind: RuntimeDiagnosticKind) -> &'static str {
    match kind {
        RuntimeDiagnosticKind::InvalidPointer => "string pointer was null",
        RuntimeDiagnosticKind::Utf8 => "operation would result in invalid UTF-8",
        RuntimeDiagnosticKind::AllocationFailed => "allocation failed while growing string",
        RuntimeDiagnosticKind::CapacityOverflow => {
            "string capacity would overflow the allowable range"
        }
        RuntimeDiagnosticKind::OutOfBounds => "requested operation exceeded string bounds",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_invalid_escape() {
        let msg = literal_message(&LiteralDiagnosticKind::InvalidEscape {
            ch: 'q',
            verbatim: false,
        });
        assert_eq!(msg, "unknown escape sequence `\\q`");
    }

    #[test]
    fn verbatim_double_quote_uses_shared_message() {
        let msg = literal_message(&LiteralDiagnosticKind::InvalidEscape {
            ch: '"',
            verbatim: true,
        });
        assert_eq!(msg, "verbatim strings use \"\" to represent a double quote");
    }

    #[test]
    fn runtime_utf8_message_shared() {
        assert_eq!(
            runtime_message(RuntimeDiagnosticKind::Utf8),
            "operation would result in invalid UTF-8"
        );
    }
}
