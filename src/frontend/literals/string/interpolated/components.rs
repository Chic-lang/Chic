use crate::frontend::literals::escape::{LiteralError, LiteralErrorKind, decode_escape};
use std::char;

use super::super::model::InterpolationSegment;

pub(super) fn parse_interpolation_segment(
    content: &str,
    start: usize,
    is_verbatim: bool,
) -> Result<(usize, InterpolationSegment, Vec<LiteralError>), LiteralError> {
    let mut depth = 0usize;
    let mut index = start;
    let mut errors = Vec::new();
    let mut end = None;
    while index < content.len() {
        let ch = content[index..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        match ch {
            '{' => {
                depth += 1;
                index += ch_len;
            }
            '}' => {
                if depth == 0 {
                    end = Some(index);
                    index += ch_len;
                    break;
                }
                depth = depth.saturating_sub(1);
                index += ch_len;
            }
            _ => index += ch_len,
        }
    }

    let expr_end = match end {
        Some(idx) => idx,
        None => {
            return Err(LiteralError::new(
                LiteralErrorKind::UnterminatedInterpolation,
                start - 1,
                content.len().saturating_sub(start - 1),
            ));
        }
    };

    let body = &content[start..expr_end];
    let (components, mut component_errors) =
        parse_interpolation_components(body, start, is_verbatim);
    let InterpolationComponents {
        expression,
        expression_offset,
        expression_len,
        alignment,
        format,
    } = components;
    errors.append(&mut component_errors);

    if expression.trim().is_empty() {
        errors.push(LiteralError::new(
            LiteralErrorKind::EmptyInterpolation,
            start,
            expr_end.saturating_sub(start),
        ));
    }

    let segment = InterpolationSegment {
        expression,
        alignment,
        format,
        expression_offset: expression_offset.saturating_add(start),
        expression_len,
    };
    Ok((index, segment, errors))
}

struct InterpolationComponents {
    expression: String,
    expression_offset: usize,
    expression_len: usize,
    alignment: Option<i32>,
    format: Option<String>,
}

fn parse_interpolation_components(
    body: &str,
    base_start: usize,
    is_verbatim: bool,
) -> (InterpolationComponents, Vec<LiteralError>) {
    let mut errors = Vec::new();
    let mut depth = 0usize;
    let mut comma_index: Option<usize> = None;
    let mut colon_index: Option<usize> = None;
    let mut offset = 0usize;
    while offset < body.len() {
        let ch = body[offset..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 && comma_index.is_none() && colon_index.is_none() => {
                comma_index = Some(offset);
            }
            ':' if depth == 0 && colon_index.is_none() => {
                colon_index = Some(offset);
                break;
            }
            _ => {}
        }
        offset += ch_len;
    }

    let expr_end = comma_index.unwrap_or_else(|| colon_index.unwrap_or(body.len()));
    let expr_slice = &body[..expr_end];
    let (trim_start, trim_end) = trim_offsets(expr_slice);
    let expression = if trim_start < trim_end {
        expr_slice[trim_start..trim_end].to_string()
    } else {
        String::new()
    };

    let alignment = comma_index
        .map(|comma_pos| {
            let end = colon_index.unwrap_or(body.len());
            let align_text = body[comma_pos + 1..end].trim();
            if align_text.is_empty() {
                errors.push(LiteralError::new(
                    LiteralErrorKind::InvalidAlignment,
                    base_start + comma_pos,
                    end.saturating_sub(comma_pos),
                ));
                None
            } else {
                match align_text.parse::<i32>() {
                    Ok(value) => Some(value),
                    Err(_) => {
                        errors.push(LiteralError::new(
                            LiteralErrorKind::InvalidAlignment,
                            base_start + comma_pos + 1,
                            align_text.len(),
                        ));
                        None
                    }
                }
            }
        })
        .flatten();

    let format = if let Some(colon_pos) = colon_index {
        let spec = body[colon_pos + 1..].trim();
        let (decoded, mut spec_errors) =
            decode_format_spec(spec, base_start + colon_pos + 1, is_verbatim);
        errors.append(&mut spec_errors);
        decoded
    } else {
        None
    };

    (
        InterpolationComponents {
            expression,
            expression_offset: trim_start,
            expression_len: trim_end.saturating_sub(trim_start),
            alignment,
            format,
        },
        errors,
    )
}

fn decode_format_spec(
    spec: &str,
    base_offset: usize,
    is_verbatim: bool,
) -> (Option<String>, Vec<LiteralError>) {
    if spec.is_empty() {
        return (None, Vec::new());
    }

    let mut output = String::with_capacity(spec.len());
    let mut errors = Vec::new();
    let mut index = 0;
    while index < spec.len() {
        let ch = spec[index..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if !is_verbatim && ch == '\\' {
            match decode_escape(spec, index) {
                Ok((decoded, consumed)) => match char::from_u32(decoded) {
                    Some(decoded) => {
                        output.push(decoded);
                        index += consumed;
                    }
                    None => {
                        let err = LiteralError::new(
                            LiteralErrorKind::InvalidCodepoint(decoded),
                            base_offset + index,
                            consumed,
                        );
                        errors.push(err);
                        index += consumed.max(1);
                        output.push('\\');
                    }
                },
                Err(mut err) => {
                    let advance = err.length.max(1);
                    index += advance;
                    output.push('\\');
                    err.offset += base_offset;
                    errors.push(err);
                }
            }
            continue;
        }

        if let Some(next) = spec[index + ch_len..].chars().next() {
            if (ch == '{' || ch == '}') && next == ch {
                output.push(ch);
                index += ch_len + next.len_utf8();
                continue;
            }
        }

        if is_verbatim && ch == '"' {
            if let Some(next) = spec[index + ch_len..].chars().next() {
                if next == '"' {
                    output.push('"');
                    index += ch_len + next.len_utf8();
                    continue;
                }
            }
        }

        output.push(ch);
        index += ch_len;
    }

    (Some(output), errors)
}

fn trim_offsets(section: &str) -> (usize, usize) {
    let mut start = section.len();
    for (idx, ch) in section.char_indices() {
        if !ch.is_whitespace() {
            start = idx;
            break;
        }
    }
    if start == section.len() {
        return (section.len(), section.len());
    }
    let mut end = start;
    for (idx, ch) in section.char_indices().rev() {
        if !ch.is_whitespace() {
            end = idx + ch.len_utf8();
            break;
        }
    }
    (start, end)
}
