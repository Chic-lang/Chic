use super::interpolated::{parse_interpolated, verify_no_newline};
use super::model::{StringLiteralContents, segments_to_contents};
use crate::frontend::literals::escape::{LiteralError, LiteralErrorKind, decode_escape};
use std::char;

pub(crate) fn parse_regular_literal(
    content: &str,
    interpolated: bool,
) -> (StringLiteralContents, Vec<LiteralError>) {
    if interpolated {
        let (segments, errors) = parse_interpolated(content, false);
        (segments_to_contents(segments), errors)
    } else {
        let (text, errors) = parse_regular_non_interpolated(content);
        (StringLiteralContents::Simple(text), errors)
    }
}

fn parse_regular_non_interpolated(content: &str) -> (String, Vec<LiteralError>) {
    let mut errors = Vec::new();
    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    while index < content.len() {
        let ch = content[index..].chars().next().unwrap();
        if ch == '\\' {
            match decode_escape(content, index) {
                Ok((decoded, consumed)) => match char::from_u32(decoded) {
                    Some(decoded) => {
                        output.push(decoded);
                        index += consumed;
                    }
                    None => {
                        errors.push(LiteralError::new(
                            LiteralErrorKind::InvalidCodepoint(decoded),
                            index,
                            consumed,
                        ));
                        index += consumed.max(1);
                        output.push('\\');
                    }
                },
                Err(err) => {
                    let advance = err.length.max(1);
                    index += advance;
                    output.push('\\');
                    errors.push(err);
                }
            }
            continue;
        }
        verify_no_newline(false, ch, index, ch.len_utf8(), &mut errors);
        output.push(ch);
        index += ch.len_utf8();
    }
    (output, errors)
}
