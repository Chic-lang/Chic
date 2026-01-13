use std::fmt;

/// Mapping from flag names to their bitmask values.
pub type FlagMapping<'a> = &'a [(&'a str, u128)];

/// Check whether `value` contains all bits specified by `mask`.
#[must_use]
pub fn contains_all(value: u128, mask: u128) -> bool {
    (value & mask) == mask
}

/// Check whether `value` contains any bit specified by `mask`.
#[must_use]
pub fn contains_any(value: u128, mask: u128) -> bool {
    (value & mask) != 0
}

/// Combine a sequence of flag values into a single mask.
#[must_use]
pub fn combine(flags: impl IntoIterator<Item = u128>) -> u128 {
    flags.into_iter().fold(0, |acc, flag| acc | flag)
}

/// Format `value` into a human-readable list of flag names.
///
/// The mapping is consulted in declaration order. If an entry represents the
/// complete value (i.e. `mapping_value == value`), that name is emitted
/// directly. Otherwise, single-bit entries are expanded. When `value` is zero,
/// the formatter prefers the mapping entry whose mask is zero. If no names
/// match, the value is rendered as an uppercase hexadecimal literal.
#[must_use]
pub fn format_flags(value: u128, mapping: FlagMapping<'_>) -> String {
    if value == 0 {
        if let Some((name, _)) = mapping.iter().find(|(_, mask)| *mask == 0) {
            return (*name).to_string();
        }
        return "0x0".to_string();
    }

    if let Some((name, _)) = mapping
        .iter()
        .find(|(_, mask)| *mask == value && *mask != 0)
    {
        return (*name).to_string();
    }

    let mut parts = Vec::new();
    let mut remaining = value;
    for (name, mask) in mapping {
        if *mask == 0 {
            continue;
        }
        if is_power_of_two(*mask) && contains_all(remaining, *mask) {
            parts.push(*name);
            remaining &= !mask;
        }
    }

    if parts.is_empty() {
        format!("0x{value:X}")
    } else {
        parts.join(" | ")
    }
}

/// Try to parse `input` into a flag value using `mapping`.
pub fn parse_flags(input: &str, mapping: FlagMapping<'_>) -> Result<u128, FlagParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    let mut value = 0u128;
    for token in trimmed.split('|') {
        let part = token.trim();
        if part.is_empty() {
            continue;
        }
        let Some((_, mask)) = mapping.iter().find(|(name, _)| *name == part) else {
            return Err(FlagParseError::new(part));
        };
        value |= *mask;
    }
    Ok(value)
}

/// Iterate over the names that are active in `value`.
#[must_use]
pub fn iter_flags<'a>(value: u128, mapping: &'a [(&'a str, u128)]) -> FlagIter<'a> {
    let exact = if value != 0 {
        mapping
            .iter()
            .position(|(_, mask)| *mask == value && *mask != 0)
    } else {
        None
    };
    FlagIter {
        value,
        mapping,
        index: 0,
        yielded_zero: false,
        exact,
    }
}

/// Error returned when a flag token cannot be recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagParseError {
    token: String,
}

impl FlagParseError {
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Display for FlagParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown flag token `{}`", self.token)
    }
}

impl std::error::Error for FlagParseError {}

/// Iterator over flag names.
pub struct FlagIter<'a> {
    value: u128,
    mapping: &'a [(&'a str, u128)],
    index: usize,
    yielded_zero: bool,
    exact: Option<usize>,
}

impl<'a> Iterator for FlagIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.exact.take() {
            self.value = 0;
            self.yielded_zero = true;
            return Some(self.mapping[idx].0);
        }
        if self.value == 0 && !self.yielded_zero {
            self.yielded_zero = true;
            if let Some((name, _)) = self.mapping.iter().find(|(_, mask)| *mask == 0) {
                return Some(*name);
            }
            return None;
        }

        while let Some((name, mask)) = self.mapping.get(self.index) {
            self.index += 1;
            if *mask == 0 || !is_power_of_two(*mask) {
                continue;
            }
            if contains_all(self.value, *mask) {
                return Some(*name);
            }
        }
        None
    }
}

fn is_power_of_two(value: u128) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAPPING: &[(&str, u128)] = &[
        ("None", 0),
        ("Read", 1),
        ("Write", 2),
        ("Execute", 4),
        ("All", 7),
    ];

    #[test]
    fn contains_helpers_respect_masks() {
        assert!(contains_all(0b111, 0b011));
        assert!(!contains_all(0b101, 0b110));
        assert!(contains_any(0b100, 0b001 | 0b100));
        assert!(!contains_any(0b010, 0b101));
    }

    #[test]
    fn combine_accumulates_flags() {
        let mask = combine([1u128, 2, 8]);
        assert_eq!(mask, 0b1011);
    }

    #[test]
    fn format_flags_prefers_exact_match() {
        assert_eq!(format_flags(0, MAPPING), "None");
        assert_eq!(format_flags(7, MAPPING), "All");
        assert_eq!(format_flags(3, MAPPING), "Read | Write");
        assert_eq!(format_flags(8, MAPPING), "0x8");
    }

    #[test]
    fn parse_flags_recognises_tokens() {
        assert_eq!(parse_flags("", MAPPING).unwrap(), 0);
        assert_eq!(parse_flags("Read", MAPPING).unwrap(), 1);
        assert_eq!(parse_flags("Read | Write", MAPPING).unwrap(), 3);
        assert_eq!(parse_flags("All", MAPPING).unwrap(), 7);
        let err = parse_flags("Unknown", MAPPING).expect_err("expected parse error");
        assert_eq!(err.token(), "Unknown");
    }

    #[test]
    fn iter_flags_yields_expected_names() {
        let collected: Vec<_> = iter_flags(7, MAPPING).collect();
        assert_eq!(collected, vec!["All"]);

        let collected: Vec<_> = iter_flags(3, MAPPING).collect();
        assert_eq!(collected, vec!["Read", "Write"]);

        let collected: Vec<_> = iter_flags(0, MAPPING).collect();
        assert_eq!(collected, vec!["None"]);
    }
}
