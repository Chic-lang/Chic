use super::Range;
use crate::unicode::normalization;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/generated/unicode17/ident.rs"
));

const EXTENDED_PICTOGRAPHIC_RANGES: &[Range] = &[
    Range {
        start: 0xA9,
        end: 0xA9,
    },
    Range {
        start: 0xAE,
        end: 0xAE,
    },
    Range {
        start: 0x203C,
        end: 0x203C,
    },
    Range {
        start: 0x2049,
        end: 0x2049,
    },
    Range {
        start: 0x2122,
        end: 0x2122,
    },
    Range {
        start: 0x2139,
        end: 0x2139,
    },
    Range {
        start: 0x2194,
        end: 0x2199,
    },
    Range {
        start: 0x21A9,
        end: 0x21AA,
    },
    Range {
        start: 0x231A,
        end: 0x231B,
    },
    Range {
        start: 0x2328,
        end: 0x2328,
    },
    Range {
        start: 0x23CF,
        end: 0x23CF,
    },
    Range {
        start: 0x23E9,
        end: 0x23F3,
    },
    Range {
        start: 0x23F8,
        end: 0x23FA,
    },
    Range {
        start: 0x24C2,
        end: 0x24C2,
    },
    Range {
        start: 0x25AA,
        end: 0x25AB,
    },
    Range {
        start: 0x25B6,
        end: 0x25B6,
    },
    Range {
        start: 0x25C0,
        end: 0x25C0,
    },
    Range {
        start: 0x25FB,
        end: 0x25FE,
    },
    Range {
        start: 0x2600,
        end: 0x2604,
    },
    Range {
        start: 0x260E,
        end: 0x260E,
    },
    Range {
        start: 0x2611,
        end: 0x2611,
    },
    Range {
        start: 0x2614,
        end: 0x2615,
    },
    Range {
        start: 0x2618,
        end: 0x2618,
    },
    Range {
        start: 0x261D,
        end: 0x261D,
    },
    Range {
        start: 0x2620,
        end: 0x2620,
    },
    Range {
        start: 0x2622,
        end: 0x2623,
    },
    Range {
        start: 0x2626,
        end: 0x2626,
    },
    Range {
        start: 0x262A,
        end: 0x262A,
    },
    Range {
        start: 0x262E,
        end: 0x262F,
    },
    Range {
        start: 0x2638,
        end: 0x263A,
    },
    Range {
        start: 0x2640,
        end: 0x2640,
    },
    Range {
        start: 0x2642,
        end: 0x2642,
    },
    Range {
        start: 0x2648,
        end: 0x2653,
    },
    Range {
        start: 0x265F,
        end: 0x2660,
    },
    Range {
        start: 0x2663,
        end: 0x2663,
    },
    Range {
        start: 0x2665,
        end: 0x2666,
    },
    Range {
        start: 0x2668,
        end: 0x2668,
    },
    Range {
        start: 0x267B,
        end: 0x267B,
    },
    Range {
        start: 0x267E,
        end: 0x267F,
    },
    Range {
        start: 0x2692,
        end: 0x2697,
    },
    Range {
        start: 0x2699,
        end: 0x2699,
    },
    Range {
        start: 0x269B,
        end: 0x269C,
    },
    Range {
        start: 0x26A0,
        end: 0x26A1,
    },
    Range {
        start: 0x26A7,
        end: 0x26A7,
    },
    Range {
        start: 0x26AA,
        end: 0x26AB,
    },
    Range {
        start: 0x26B0,
        end: 0x26B1,
    },
    Range {
        start: 0x26BD,
        end: 0x26BE,
    },
    Range {
        start: 0x26C4,
        end: 0x26C5,
    },
    Range {
        start: 0x26C8,
        end: 0x26C8,
    },
    Range {
        start: 0x26CE,
        end: 0x26CF,
    },
    Range {
        start: 0x26D1,
        end: 0x26D1,
    },
    Range {
        start: 0x26D3,
        end: 0x26D4,
    },
    Range {
        start: 0x26E9,
        end: 0x26EA,
    },
    Range {
        start: 0x26F0,
        end: 0x26F5,
    },
    Range {
        start: 0x26F7,
        end: 0x26FA,
    },
    Range {
        start: 0x26FD,
        end: 0x26FD,
    },
    Range {
        start: 0x2702,
        end: 0x2702,
    },
    Range {
        start: 0x2705,
        end: 0x2705,
    },
    Range {
        start: 0x2708,
        end: 0x270D,
    },
    Range {
        start: 0x270F,
        end: 0x270F,
    },
    Range {
        start: 0x2712,
        end: 0x2712,
    },
    Range {
        start: 0x2714,
        end: 0x2714,
    },
    Range {
        start: 0x2716,
        end: 0x2716,
    },
    Range {
        start: 0x271D,
        end: 0x271D,
    },
    Range {
        start: 0x2721,
        end: 0x2721,
    },
    Range {
        start: 0x2728,
        end: 0x2728,
    },
    Range {
        start: 0x2733,
        end: 0x2734,
    },
    Range {
        start: 0x2744,
        end: 0x2744,
    },
    Range {
        start: 0x2747,
        end: 0x2747,
    },
    Range {
        start: 0x274C,
        end: 0x274C,
    },
    Range {
        start: 0x274E,
        end: 0x274E,
    },
    Range {
        start: 0x2753,
        end: 0x2755,
    },
    Range {
        start: 0x2757,
        end: 0x2757,
    },
    Range {
        start: 0x2763,
        end: 0x2764,
    },
    Range {
        start: 0x2795,
        end: 0x2797,
    },
    Range {
        start: 0x27A1,
        end: 0x27A1,
    },
    Range {
        start: 0x27B0,
        end: 0x27B0,
    },
    Range {
        start: 0x27BF,
        end: 0x27BF,
    },
    Range {
        start: 0x2934,
        end: 0x2935,
    },
    Range {
        start: 0x2B05,
        end: 0x2B07,
    },
    Range {
        start: 0x2B1B,
        end: 0x2B1C,
    },
    Range {
        start: 0x2B50,
        end: 0x2B50,
    },
    Range {
        start: 0x2B55,
        end: 0x2B55,
    },
    Range {
        start: 0x3030,
        end: 0x3030,
    },
    Range {
        start: 0x303D,
        end: 0x303D,
    },
    Range {
        start: 0x3297,
        end: 0x3297,
    },
    Range {
        start: 0x3299,
        end: 0x3299,
    },
    Range {
        start: 0x1F004,
        end: 0x1F004,
    },
    Range {
        start: 0x1F02C,
        end: 0x1F02F,
    },
    Range {
        start: 0x1F094,
        end: 0x1F09F,
    },
    Range {
        start: 0x1F0AF,
        end: 0x1F0B0,
    },
    Range {
        start: 0x1F0C0,
        end: 0x1F0C0,
    },
    Range {
        start: 0x1F0CF,
        end: 0x1F0D0,
    },
    Range {
        start: 0x1F0F6,
        end: 0x1F0FF,
    },
    Range {
        start: 0x1F170,
        end: 0x1F171,
    },
    Range {
        start: 0x1F17E,
        end: 0x1F17F,
    },
    Range {
        start: 0x1F18E,
        end: 0x1F18E,
    },
    Range {
        start: 0x1F191,
        end: 0x1F19A,
    },
    Range {
        start: 0x1F1AE,
        end: 0x1F1E5,
    },
    Range {
        start: 0x1F201,
        end: 0x1F20F,
    },
    Range {
        start: 0x1F21A,
        end: 0x1F21A,
    },
    Range {
        start: 0x1F22F,
        end: 0x1F22F,
    },
    Range {
        start: 0x1F232,
        end: 0x1F23A,
    },
    Range {
        start: 0x1F23C,
        end: 0x1F23F,
    },
    Range {
        start: 0x1F249,
        end: 0x1F25F,
    },
    Range {
        start: 0x1F266,
        end: 0x1F321,
    },
    Range {
        start: 0x1F324,
        end: 0x1F393,
    },
    Range {
        start: 0x1F396,
        end: 0x1F397,
    },
    Range {
        start: 0x1F399,
        end: 0x1F39B,
    },
    Range {
        start: 0x1F39E,
        end: 0x1F3F0,
    },
    Range {
        start: 0x1F3F3,
        end: 0x1F3F5,
    },
    Range {
        start: 0x1F3F7,
        end: 0x1F3FA,
    },
    Range {
        start: 0x1F400,
        end: 0x1F4FD,
    },
    Range {
        start: 0x1F4FF,
        end: 0x1F53D,
    },
    Range {
        start: 0x1F549,
        end: 0x1F54E,
    },
    Range {
        start: 0x1F550,
        end: 0x1F567,
    },
    Range {
        start: 0x1F56F,
        end: 0x1F570,
    },
    Range {
        start: 0x1F573,
        end: 0x1F57A,
    },
    Range {
        start: 0x1F587,
        end: 0x1F587,
    },
    Range {
        start: 0x1F58A,
        end: 0x1F58D,
    },
    Range {
        start: 0x1F590,
        end: 0x1F590,
    },
    Range {
        start: 0x1F595,
        end: 0x1F596,
    },
    Range {
        start: 0x1F5A4,
        end: 0x1F5A5,
    },
    Range {
        start: 0x1F5A8,
        end: 0x1F5A8,
    },
    Range {
        start: 0x1F5B1,
        end: 0x1F5B2,
    },
    Range {
        start: 0x1F5BC,
        end: 0x1F5BC,
    },
    Range {
        start: 0x1F5C2,
        end: 0x1F5C4,
    },
    Range {
        start: 0x1F5D1,
        end: 0x1F5D3,
    },
    Range {
        start: 0x1F5DC,
        end: 0x1F5DE,
    },
    Range {
        start: 0x1F5E1,
        end: 0x1F5E1,
    },
    Range {
        start: 0x1F5E3,
        end: 0x1F5E3,
    },
    Range {
        start: 0x1F5E8,
        end: 0x1F5E8,
    },
    Range {
        start: 0x1F5EF,
        end: 0x1F5EF,
    },
    Range {
        start: 0x1F5F3,
        end: 0x1F5F3,
    },
    Range {
        start: 0x1F5FA,
        end: 0x1F64F,
    },
    Range {
        start: 0x1F680,
        end: 0x1F6C5,
    },
    Range {
        start: 0x1F6CB,
        end: 0x1F6D2,
    },
    Range {
        start: 0x1F6D5,
        end: 0x1F6E5,
    },
    Range {
        start: 0x1F6E9,
        end: 0x1F6E9,
    },
    Range {
        start: 0x1F6EB,
        end: 0x1F6F0,
    },
    Range {
        start: 0x1F6F3,
        end: 0x1F6FF,
    },
    Range {
        start: 0x1F7DA,
        end: 0x1F7FF,
    },
    Range {
        start: 0x1F80C,
        end: 0x1F80F,
    },
    Range {
        start: 0x1F848,
        end: 0x1F84F,
    },
    Range {
        start: 0x1F85A,
        end: 0x1F85F,
    },
    Range {
        start: 0x1F888,
        end: 0x1F88F,
    },
    Range {
        start: 0x1F8AE,
        end: 0x1F8AF,
    },
    Range {
        start: 0x1F8BC,
        end: 0x1F8BF,
    },
    Range {
        start: 0x1F8C2,
        end: 0x1F8CF,
    },
    Range {
        start: 0x1F8D9,
        end: 0x1F8FF,
    },
    Range {
        start: 0x1F90C,
        end: 0x1F93A,
    },
    Range {
        start: 0x1F93C,
        end: 0x1F945,
    },
    Range {
        start: 0x1F947,
        end: 0x1F9FF,
    },
    Range {
        start: 0x1FA58,
        end: 0x1FA5F,
    },
    Range {
        start: 0x1FA6E,
        end: 0x1FAFF,
    },
    Range {
        start: 0x1FC00,
        end: 0x1FFFD,
    },
];

const FORBIDDEN_CONTROL_RANGES: &[Range] = &[
    Range {
        start: 0x061C,
        end: 0x061C,
    },
    Range {
        start: 0x200C,
        end: 0x200F,
    },
    Range {
        start: 0x202A,
        end: 0x202E,
    },
    Range {
        start: 0x2066,
        end: 0x2069,
    },
    Range {
        start: 0xFE00,
        end: 0xFE0F,
    },
    Range {
        start: 0xE0100,
        end: 0xE01EF,
    },
];

/// Result of analysing an identifier surface form.
#[derive(Debug, Clone)]
pub struct IdentifierStatus {
    /// NFC-normalised spelling of the identifier.
    pub normalized: String,
    /// Whether the source spelling required normalisation.
    pub was_normalized: bool,
    /// Disallowed code point encountered while scanning, if any.
    pub disallowed: Option<DisallowedChar>,
    /// Suggested replacement when the identifier needs normalisation or sanitisation.
    pub suggestion: Option<String>,
}

/// Description of a rejected identifier code point.
#[derive(Debug, Clone)]
pub struct DisallowedChar {
    pub offset: usize,
    pub ch: char,
    pub property: &'static str,
}

/// Analyse an identifier lexeme for Unicode 17 compliance.
pub fn analyse_identifier(raw: &str) -> IdentifierStatus {
    let mut disallowed = None;
    let mut sanitized = String::with_capacity(raw.len());
    for (offset, ch) in raw.char_indices() {
        if let Some(reason) = forbidden_control(ch) {
            disallowed = Some(DisallowedChar {
                offset,
                ch,
                property: reason,
            });
            // Drop forbidden control characters from the sanitised spelling so interned identifiers remain stable.
            continue;
        }
        let allowed = if offset == 0 {
            is_identifier_start(ch)
        } else {
            is_identifier_continue(ch)
        };
        if !allowed {
            let property = disallowed_reason(ch).unwrap_or("identifier code point not permitted");
            disallowed = Some(DisallowedChar {
                offset,
                ch,
                property,
            });
            // Keep scanning to build a complete sanitised replacement suggestion.
        }
        sanitized.push(ch);
    }

    let normalized = normalization::normalize_nfc(&sanitized);
    let was_normalized = normalization::normalize_nfc(raw) != raw;
    let suggestion = if normalized != raw || sanitized != raw {
        Some(normalized.clone())
    } else {
        None
    };
    IdentifierStatus {
        normalized,
        was_normalized,
        disallowed,
        suggestion,
    }
}

/// Returns true when the character is permitted at the start of an identifier.
pub fn is_identifier_start(ch: char) -> bool {
    ch == '_'
        || (forbidden_control(ch).is_none()
            && in_ranges(ch, ID_START_RANGES)
            && !is_pattern_disallowed(ch))
        || in_ranges(ch, EXTENDED_PICTOGRAPHIC_RANGES)
}

/// Returns true when the character can continue an identifier.
pub fn is_identifier_continue(ch: char) -> bool {
    ch == '_'
        || (forbidden_control(ch).is_none()
            && in_ranges(ch, ID_CONTINUE_RANGES)
            && !is_pattern_disallowed(ch))
        || in_ranges(ch, EXTENDED_PICTOGRAPHIC_RANGES)
}

/// Returns true when the character would otherwise qualify for ID_Start/ID_Continue
/// but is excluded by pattern syntax/whitespace restrictions.
pub fn disallowed_reason(ch: char) -> Option<&'static str> {
    if let Some(reason) = forbidden_control(ch) {
        return Some(reason);
    }
    if in_ranges(ch, PATTERN_SYNTAX_RANGES) {
        Some("Pattern_Syntax")
    } else if in_ranges(ch, PATTERN_WHITE_SPACE_RANGES) {
        Some("Pattern_White_Space")
    } else if in_ranges(ch, ID_START_RANGES) || in_ranges(ch, ID_CONTINUE_RANGES) {
        Some("Identifier property (ID_Start/ID_Continue) not permitted here")
    } else {
        None
    }
}

/// Returns true when the character appears in the identifier property sets (ID_Start/ID_Continue),
/// even if it is later disallowed by pattern syntax rules.
pub fn looks_like_identifier(ch: char) -> bool {
    let code = ch as u32;
    range_contains(code, ID_START_RANGES)
        || range_contains(code, ID_CONTINUE_RANGES)
        || range_contains(code, EXTENDED_PICTOGRAPHIC_RANGES)
}

/// Returns true when the character is a forbidden control/invisible that must not appear inside identifiers.
pub fn is_forbidden_identifier_control(ch: char) -> bool {
    forbidden_control(ch).is_some()
}

fn in_ranges(ch: char, ranges: &[Range]) -> bool {
    range_contains(ch as u32, ranges)
}

fn range_contains(value: u32, ranges: &[Range]) -> bool {
    if ranges.is_empty() {
        return false;
    }
    let idx = ranges.partition_point(|range| range.end < value);
    if let Some(range) = ranges.get(idx) {
        range.start <= value && value <= range.end
    } else {
        false
    }
}

#[inline]
fn is_pattern_disallowed(ch: char) -> bool {
    in_ranges(ch, PATTERN_SYNTAX_RANGES) || in_ranges(ch, PATTERN_WHITE_SPACE_RANGES)
}

fn forbidden_control(ch: char) -> Option<&'static str> {
    if in_ranges(ch, FORBIDDEN_CONTROL_RANGES) {
        if ch == '\u{200C}' || ch == '\u{200D}' {
            return Some("Join_Control");
        }
        if (0x202A..=0x202E).contains(&(ch as u32))
            || (0x2066..=0x2069).contains(&(ch as u32))
            || ch == '\u{061C}'
            || ch == '\u{200E}'
            || ch == '\u{200F}'
        {
            return Some("Bidi_Control");
        }
        if (0xFE00..=0xFE0F).contains(&(ch as u32)) || (0xE0100..=0xE01EF).contains(&(ch as u32)) {
            return Some("Variation_Selector");
        }
        return Some("forbidden control code point");
    }
    None
}
