use super::interpolated::parse_interpolated;
use super::model::{StringLiteralContents, segments_to_contents};
use crate::frontend::literals::escape::{LiteralError, LiteralErrorKind};

pub(crate) fn parse_verbatim_literal(
    content: &str,
    interpolated: bool,
) -> (StringLiteralContents, Vec<LiteralError>) {
    if interpolated {
        let (segments, errors) = parse_interpolated(content, true);
        (segments_to_contents(segments), errors)
    } else {
        let (text, errors) = parse_verbatim_non_interpolated(content);
        (StringLiteralContents::Simple(text), errors)
    }
}

fn parse_verbatim_non_interpolated(content: &str) -> (String, Vec<LiteralError>) {
    let mut errors = Vec::new();
    let mut output = String::with_capacity(content.len());
    let mut chars = content.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '"' {
            if let Some(&(_, next)) = chars.peek() {
                if next == '"' {
                    output.push('"');
                    chars.next();
                    continue;
                }
            }
            errors.push(LiteralError::new(
                LiteralErrorKind::InvalidEscape('"'),
                idx,
                ch.len_utf8(),
            ));
            output.push('"');
        } else {
            output.push(ch);
        }
    }
    (output, errors)
}
