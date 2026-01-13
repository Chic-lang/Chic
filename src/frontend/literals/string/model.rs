/// Specific flavour of string literal encountered in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringLiteralKind {
    Regular,
    Verbatim,
    Interpolated,
    InterpolatedVerbatim,
}

/// Parsed string literal with decoded contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub kind: StringLiteralKind,
    pub contents: StringLiteralContents,
}

/// Contents of a parsed string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLiteralContents {
    Simple(String),
    Interpolated(Vec<StringSegment>),
}

/// Segment composing an interpolated string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringSegment {
    Text(String),
    Interpolation(InterpolationSegment),
}

/// Interpolation entry with optional alignment and format specifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationSegment {
    pub expression: String,
    pub alignment: Option<i32>,
    pub format: Option<String>,
    /// Byte offset within the literal content where the trimmed expression begins.
    pub expression_offset: usize,
    /// Trimmed expression length in bytes.
    pub expression_len: usize,
}

impl StringLiteralContents {
    pub fn simple(text: String) -> Self {
        StringLiteralContents::Simple(text)
    }

    pub fn interpolated(segments: Vec<StringSegment>) -> Self {
        StringLiteralContents::Interpolated(segments)
    }
}

impl From<StringSegment> for StringLiteralContents {
    fn from(segment: StringSegment) -> Self {
        StringLiteralContents::Interpolated(vec![segment])
    }
}

pub(crate) fn segments_to_contents(segments: Vec<StringSegment>) -> StringLiteralContents {
    if segments
        .iter()
        .all(|segment| matches!(segment, StringSegment::Text(_)))
    {
        let mut text = String::new();
        for segment in segments {
            if let StringSegment::Text(value) = segment {
                text.push_str(&value);
            }
        }
        StringLiteralContents::Simple(text)
    } else {
        StringLiteralContents::Interpolated(segments)
    }
}
