use crate::unicode::escapes;

/// Error produced when decoding a literal escape sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralError {
    pub kind: LiteralErrorKind,
    /// Byte offset within the literal (relative to the literal start).
    pub offset: usize,
    /// Length of the offending fragment in bytes if known.
    pub length: usize,
}

impl LiteralError {
    #[must_use]
    pub fn new(kind: LiteralErrorKind, offset: usize, length: usize) -> Self {
        Self {
            kind,
            offset,
            length,
        }
    }
}

/// Categorisation of literal parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralErrorKind {
    UnexpectedEnd,
    InvalidEscape(char),
    InvalidHexDigit(char),
    InvalidUnicodeLen { expected: usize, actual: usize },
    InvalidCodepoint(u32),
    MissingHexDigits,
    NewlineInLiteral,
    EmptyCharLiteral,
    TooManyCharacters,
    UnmatchedClosingBrace,
    UnterminatedInterpolation,
    EmptyInterpolation,
    InvalidAlignment,
}

/// Attempt to decode a C# escape sequence beginning at `start`.
///
/// Returns the decoded character and the number of bytes consumed.
pub fn decode_escape(text: &str, start: usize) -> Result<(u32, usize), LiteralError> {
    debug_assert!(text[start..].starts_with('\\'));
    let bytes = text.as_bytes();
    let mut index = start + 1;
    if index >= bytes.len() {
        return Err(LiteralError::new(
            LiteralErrorKind::UnexpectedEnd,
            start,
            bytes.len().saturating_sub(start),
        ));
    }
    let marker = bytes[index] as char;
    index += 1;
    if let Some(codepoint) = escapes::decode_basic_escape(marker) {
        return Ok((codepoint as u32, index - start));
    }
    match marker {
        'u' => decode_fixed_length_escape(bytes, start, index, 4),
        'U' => decode_fixed_length_escape(bytes, start, index, 8),
        'x' => decode_variable_length_escape(bytes, start, index, 1, 4),
        other => Err(LiteralError::new(
            LiteralErrorKind::InvalidEscape(other),
            start,
            index - start,
        )),
    }
}

fn decode_fixed_length_escape(
    bytes: &[u8],
    start: usize,
    mut index: usize,
    width: usize,
) -> Result<(u32, usize), LiteralError> {
    let slice = bytes.get(index..index + width).ok_or_else(|| {
        LiteralError::new(
            LiteralErrorKind::InvalidUnicodeLen {
                expected: width,
                actual: bytes.len().saturating_sub(index),
            },
            start,
            bytes.len().saturating_sub(start),
        )
    })?;
    match parse_hex_slice(slice) {
        Ok((value, consumed)) => {
            index += consumed;
            if value > 0x0010_FFFF {
                return Err(LiteralError::new(
                    LiteralErrorKind::InvalidCodepoint(value),
                    start,
                    index - start,
                ));
            }
            Ok((value, index - start))
        }
        Err((offset, invalid)) => Err(LiteralError::new(
            LiteralErrorKind::InvalidHexDigit(invalid),
            start + offset + 2,
            1,
        )),
    }
}

fn decode_variable_length_escape(
    bytes: &[u8],
    start: usize,
    mut index: usize,
    min: usize,
    max: usize,
) -> Result<(u32, usize), LiteralError> {
    let mut consumed = 0;
    let mut value: u32 = 0;
    while consumed < max {
        if let Some(&next) = bytes.get(index) {
            let ch = next as char;
            if let Some(digit) = ch.to_digit(16) {
                value = (value << 4) | digit;
                consumed += 1;
                index += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if consumed < min {
        return Err(LiteralError::new(
            LiteralErrorKind::MissingHexDigits,
            start,
            (index - start).max(1),
        ));
    }

    if value > 0x0010_FFFF {
        return Err(LiteralError::new(
            LiteralErrorKind::InvalidCodepoint(value),
            start,
            index - start,
        ));
    }

    Ok((value, index - start))
}

fn parse_hex_slice(slice: &[u8]) -> Result<(u32, usize), (usize, char)> {
    let mut value: u32 = 0;
    for (idx, &byte) in slice.iter().enumerate() {
        let ch = byte as char;
        let Some(digit) = ch.to_digit(16) else {
            return Err((idx, ch));
        };
        value = (value << 4) | digit;
    }
    Ok((value, slice.len()))
}
