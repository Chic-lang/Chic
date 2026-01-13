mod interpolated;
mod model;
mod regular;
mod verbatim;

#[cfg(test)]
mod tests;

pub use model::{
    InterpolationSegment, StringLiteral, StringLiteralContents, StringLiteralKind, StringSegment,
};

use regular::parse_regular_literal;
use verbatim::parse_verbatim_literal;

use crate::frontend::literals::LiteralError;
use crate::unicode::normalization;

/// Decode a string literal body (excluding delimiters) according to its flavour.
pub fn parse_string_literal(
    content: &str,
    kind: StringLiteralKind,
) -> (StringLiteral, Vec<LiteralError>) {
    let (contents, errors) = match kind {
        StringLiteralKind::Regular => parse_regular_literal(content, false),
        StringLiteralKind::Interpolated => parse_regular_literal(content, true),
        StringLiteralKind::Verbatim => parse_verbatim_literal(content, false),
        StringLiteralKind::InterpolatedVerbatim => parse_verbatim_literal(content, true),
    };

    (
        StringLiteral {
            kind,
            contents: normalize_contents(contents),
        },
        errors,
    )
}

fn normalize_contents(contents: StringLiteralContents) -> StringLiteralContents {
    match contents {
        StringLiteralContents::Simple(text) => {
            StringLiteralContents::Simple(normalization::normalize_nfc(&text))
        }
        StringLiteralContents::Interpolated(segments) => {
            let segments = segments
                .into_iter()
                .map(|segment| match segment {
                    StringSegment::Text(text) => {
                        StringSegment::Text(normalization::normalize_nfc(&text))
                    }
                    StringSegment::Interpolation(mut seg) => {
                        if let Some(format) = seg.format.take() {
                            seg.format = Some(normalization::normalize_nfc(&format));
                        }
                        StringSegment::Interpolation(seg)
                    }
                })
                .collect();
            StringLiteralContents::Interpolated(segments)
        }
    }
}
