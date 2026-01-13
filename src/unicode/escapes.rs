/// Mapping for a basic escape sequence (e.g. `\n`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscapeMapping {
    pub marker: char,
    pub codepoint: char,
    pub description: &'static str,
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/generated/unicode/escapes.rs"
));

/// Return the decoded codepoint for a basic escape marker such as `n` or `t`.
#[must_use]
pub fn decode_basic_escape(marker: char) -> Option<char> {
    if marker.is_ascii() {
        BASIC_ESCAPE_BY_MARKER[marker as usize]
    } else {
        None
    }
}

/// Return the canonical escape marker for ASCII control characters.
#[must_use]
pub fn encode_basic_escape(ch: char) -> Option<char> {
    if ch.is_ascii() {
        BASIC_ESCAPE_BY_CODEPOINT[ch as usize]
    } else {
        None
    }
}

/// Expose the full list of escape mappings for downstream validation/tests.
#[must_use]
pub fn basic_escape_mappings() -> &'static [EscapeMapping] {
    BASIC_ESCAPE_MAPPINGS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_known_escape() {
        assert_eq!(decode_basic_escape('n'), Some('\n'));
        assert_eq!(decode_basic_escape('t'), Some('\t'));
    }

    #[test]
    fn encode_known_escape() {
        assert_eq!(encode_basic_escape('\r'), Some('r'));
        assert_eq!(encode_basic_escape('\u{0007}'), Some('a'));
    }

    #[test]
    fn ignores_non_ascii_inputs() {
        assert_eq!(decode_basic_escape('å'), None);
        assert_eq!(encode_basic_escape('å'), None);
    }
}
