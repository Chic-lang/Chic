use crate::frontend::literals::escape::{LiteralError, LiteralErrorKind, decode_escape};
use std::char;

use super::super::model::StringSegment;
use super::components::parse_interpolation_segment;

pub(crate) fn parse_interpolated(
    content: &str,
    is_verbatim: bool,
) -> (Vec<StringSegment>, Vec<LiteralError>) {
    let mut segments: Vec<StringSegment> = Vec::new();
    let mut current = String::new();
    let mut errors = Vec::new();
    let mut index = 0;

    while index < content.len() {
        let ch = content[index..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        if !is_verbatim && ch == '\\' {
            match decode_escape(content, index) {
                Ok((decoded, consumed)) => match char::from_u32(decoded) {
                    Some(decoded) => {
                        current.push(decoded);
                        index += consumed;
                    }
                    None => {
                        errors.push(LiteralError::new(
                            LiteralErrorKind::InvalidCodepoint(decoded),
                            index,
                            consumed,
                        ));
                        index += consumed.max(1);
                        current.push('\\');
                    }
                },
                Err(err) => {
                    let advance = err.length.max(1);
                    index += advance;
                    current.push('\\');
                    errors.push(err);
                }
            }
            continue;
        }

        if ch == '\n' || ch == '\r' {
            verify_no_newline(is_verbatim, ch, index, ch_len, &mut errors);
            current.push(ch);
            index += ch_len;
            continue;
        }

        if ch == '{' {
            if let Some(next) = content[index + ch_len..].chars().next() {
                if next == '{' {
                    current.push('{');
                    index += ch_len + next.len_utf8();
                    continue;
                }
            }

            flush_text_segment(&mut current, &mut segments);
            match parse_interpolation_segment(content, index + ch_len, is_verbatim) {
                Ok((next_index, segment, mut seg_errors)) => {
                    segments.push(StringSegment::Interpolation(segment));
                    errors.append(&mut seg_errors);
                    index = next_index;
                }
                Err(err) => {
                    errors.push(err);
                    break;
                }
            }
            continue;
        }

        if ch == '}' {
            if let Some(next) = content[index + ch_len..].chars().next() {
                if next == '}' {
                    current.push('}');
                    index += ch_len + next.len_utf8();
                    continue;
                }
            }
            errors.push(LiteralError::new(
                LiteralErrorKind::UnmatchedClosingBrace,
                index,
                ch_len,
            ));
            index += ch_len;
            continue;
        }

        if is_verbatim && ch == '"' {
            if let Some(next) = content[index + ch_len..].chars().next() {
                if next == '"' {
                    current.push('"');
                    index += ch_len + next.len_utf8();
                    continue;
                }
            }
            errors.push(LiteralError::new(
                LiteralErrorKind::InvalidEscape('"'),
                index,
                ch_len,
            ));
            current.push('"');
            index += ch_len;
            continue;
        }

        current.push(ch);
        index += ch_len;
    }

    flush_text_segment(&mut current, &mut segments);
    (segments, errors)
}

pub(crate) fn verify_no_newline(
    is_verbatim: bool,
    ch: char,
    offset: usize,
    ch_len: usize,
    errors: &mut Vec<LiteralError>,
) {
    if !is_verbatim && (ch == '\n' || ch == '\r') {
        errors.push(LiteralError::new(
            LiteralErrorKind::NewlineInLiteral,
            offset,
            ch_len,
        ));
    }
}

fn flush_text_segment(current: &mut String, segments: &mut Vec<StringSegment>) {
    if !current.is_empty() {
        segments.push(StringSegment::Text(std::mem::take(current)));
    }
}
