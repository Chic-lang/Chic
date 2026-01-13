use super::escape::{LiteralError, LiteralErrorKind, decode_escape};

/// Parsed representation of a character literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharLiteral {
    pub value: u16,
}

impl Default for CharLiteral {
    fn default() -> Self {
        Self { value: 0 }
    }
}

/// Parse the body of a character literal (excluding surrounding quotes).
pub fn parse_char_literal(content: &str) -> (Option<CharLiteral>, Vec<LiteralError>) {
    if content.is_empty() {
        return (
            None,
            vec![LiteralError::new(LiteralErrorKind::EmptyCharLiteral, 0, 0)],
        );
    }

    if content == "\n" || content == "\r" || content == "\r\n" {
        return (
            None,
            vec![LiteralError::new(
                LiteralErrorKind::NewlineInLiteral,
                0,
                content.len(),
            )],
        );
    }

    if content.starts_with('\\') {
        match decode_escape(content, 0) {
            Ok((value, consumed)) => {
                if value > 0xFFFF {
                    let errors = vec![LiteralError::new(
                        LiteralErrorKind::TooManyCharacters,
                        consumed.min(content.len()),
                        content.len().saturating_sub(consumed),
                    )];
                    return (None, errors);
                }
                if consumed != content.len() {
                    let extra = content.len().saturating_sub(consumed);
                    let errors = vec![LiteralError::new(
                        LiteralErrorKind::TooManyCharacters,
                        consumed,
                        extra,
                    )];
                    (
                        Some(CharLiteral {
                            value: value as u16,
                        }),
                        errors,
                    )
                } else {
                    (
                        Some(CharLiteral {
                            value: value as u16,
                        }),
                        Vec::new(),
                    )
                }
            }
            Err(err) => (None, vec![err]),
        }
    } else {
        let mut chars = content.chars();
        let first_scalar = chars.next().unwrap();
        if first_scalar == '\n' || first_scalar == '\r' {
            return (
                None,
                vec![LiteralError::new(
                    LiteralErrorKind::NewlineInLiteral,
                    0,
                    first_scalar.len_utf8(),
                )],
            );
        }
        let mut utf16 = content.encode_utf16();
        let first_unit = utf16.next().unwrap();
        if utf16.next().is_some() {
            return (
                None,
                vec![LiteralError::new(
                    LiteralErrorKind::TooManyCharacters,
                    first_scalar.len_utf8(),
                    content.len().saturating_sub(first_scalar.len_utf8()),
                )],
            );
        }
        (Some(CharLiteral { value: first_unit }), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_single_value(result: (Option<CharLiteral>, Vec<LiteralError>)) -> char {
        let (value, errors) = result;
        if !errors.is_empty() {
            panic!("unexpected errors: {errors:?}");
        }
        char::from_u32(u32::from(value.expect("expected literal").value))
            .expect("valid BMP scalar for test")
    }

    #[test]
    fn parses_simple_character() {
        let value = assert_single_value(parse_char_literal("A"));
        assert_eq!(value, 'A');
    }

    #[test]
    fn parses_escaped_character() {
        let value = assert_single_value(parse_char_literal("\\n"));
        assert_eq!(value, '\n');
    }

    #[test]
    fn reports_too_many_characters() {
        let (_, errors) = parse_char_literal("ab");
        assert!(
            errors
                .iter()
                .any(|err| matches!(err.kind, LiteralErrorKind::TooManyCharacters))
        );
    }

    #[test]
    fn parses_supplementary_plane_scalar() {
        let (value, errors) = parse_char_literal("\\U0001F60A");
        assert!(value.is_none());
        assert!(
            errors
                .iter()
                .any(|err| matches!(err.kind, LiteralErrorKind::TooManyCharacters))
        );
    }

    #[test]
    fn rejects_surrogate_escape() {
        let (value, errors) = parse_char_literal("\\uD800");
        assert!(errors.is_empty());
        assert_eq!(value.expect("literal").value, 0xD800);
    }

    #[test]
    fn rejects_multiple_scalars() {
        let (value, errors) = parse_char_literal("🇺🇳");
        assert!(value.is_none());
        assert!(
            errors
                .iter()
                .any(|err| matches!(err.kind, LiteralErrorKind::TooManyCharacters))
        );
    }

    #[test]
    fn rejects_values_above_unicode_range() {
        let (value, errors) = parse_char_literal("\\U00110000");
        assert!(value.is_none());
        assert!(
            errors
                .iter()
                .any(|err| matches!(err.kind, LiteralErrorKind::InvalidCodepoint(0x0011_0000)))
        );
    }
}
