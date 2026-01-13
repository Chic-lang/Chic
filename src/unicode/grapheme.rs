use super::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphemeBreakProperty {
    CR,
    LF,
    Control,
    Extend,
    RegionalIndicator,
    Prepend,
    SpacingMark,
    L,
    V,
    T,
    LV,
    LVT,
    ZWJ,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphemeBreakRange {
    pub start: u32,
    pub end: u32,
    pub property: GraphemeBreakProperty,
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/generated/unicode17/grapheme.rs"
));

/// Compute byte offsets for grapheme cluster boundaries within `text`.
#[must_use]
pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::with_capacity(text.len().saturating_add(1));
    boundaries.push(0);

    let mut prev_prop: Option<GraphemeBreakProperty> = None;
    let mut last_non_extend_is_ext_pic = false;
    let mut prev_was_zwj_after_ext_pic = false;
    let mut ri_run = 0usize;

    for (idx, ch) in text.char_indices() {
        let prop = property_for(ch);
        let is_ext_pic = is_extended_pictographic(ch);
        let should_break = match prev_prop {
            None => false,
            Some(prev) => break_between(prev, prop, prev_was_zwj_after_ext_pic, is_ext_pic, ri_run),
        };

        if should_break {
            boundaries.push(idx);
            ri_run = 0;
        }

        if prop == GraphemeBreakProperty::RegionalIndicator {
            ri_run += 1;
        } else {
            ri_run = 0;
        }

        prev_was_zwj_after_ext_pic =
            prop == GraphemeBreakProperty::ZWJ && last_non_extend_is_ext_pic;
        if !matches!(prop, GraphemeBreakProperty::Extend) {
            last_non_extend_is_ext_pic = is_ext_pic;
        }

        prev_prop = Some(prop);
    }

    boundaries.push(text.len());
    boundaries
}

/// Number of grapheme clusters in the provided text.
#[must_use]
pub fn grapheme_count(text: &str) -> usize {
    grapheme_boundaries(text).len().saturating_sub(1)
}

/// Grapheme-aware column (1-based) for a byte offset into `text`.
#[must_use]
pub fn grapheme_column(text: &str, byte_offset: usize) -> usize {
    let boundaries = grapheme_boundaries(text);
    let idx = boundary_index(&boundaries, byte_offset);
    idx + 1
}

/// Grapheme cluster length for the `[start, end)` byte range.
#[must_use]
pub fn grapheme_span_len(text: &str, start: usize, end: usize) -> usize {
    let boundaries = grapheme_boundaries(text);
    let mut clusters = 0usize;
    for window in boundaries.windows(2) {
        let (b_start, b_end) = (window[0], window[1]);
        if b_end <= start {
            continue;
        }
        if b_start >= end {
            break;
        }
        clusters += 1;
    }
    clusters.max(1)
}

fn property_for(ch: char) -> GraphemeBreakProperty {
    let value = ch as u32;
    let idx = GRAPHEME_BREAK_RANGES.partition_point(|range| range.end < value);
    if let Some(range) = GRAPHEME_BREAK_RANGES.get(idx) {
        if range.start <= value && value <= range.end {
            return range.property;
        }
    }
    GraphemeBreakProperty::Other
}

fn is_extended_pictographic(ch: char) -> bool {
    let value = ch as u32;
    let idx = EXTENDED_PICTOGRAPHIC_RANGES.partition_point(|range| range.end < value);
    EXTENDED_PICTOGRAPHIC_RANGES
        .get(idx)
        .map_or(false, |range| range.contains(value))
}

fn break_between(
    prev: GraphemeBreakProperty,
    curr: GraphemeBreakProperty,
    prev_was_zwj_after_ext_pic: bool,
    curr_is_ext_pic: bool,
    ri_run: usize,
) -> bool {
    use GraphemeBreakProperty::*;

    if prev == CR && curr == LF {
        return false;
    }
    if matches!(prev, CR | LF | Control) || matches!(curr, CR | LF | Control) {
        return true;
    }
    if matches!(prev, L) && matches!(curr, L | V | LV | LVT) {
        return false;
    }
    if matches!(prev, LV | V) && matches!(curr, V | T) {
        return false;
    }
    if matches!(prev, LVT | T) && matches!(curr, T) {
        return false;
    }
    if matches!(curr, Extend | ZWJ) {
        return false;
    }
    if curr == SpacingMark {
        return false;
    }
    if prev == Prepend {
        return false;
    }
    if prev == ZWJ && prev_was_zwj_after_ext_pic && curr_is_ext_pic {
        return false;
    }
    if prev == RegionalIndicator && curr == RegionalIndicator {
        // GB12 / GB13: do not break between RI pairs; break after every second RI.
        return ri_run % 2 == 0;
    }
    true
}

fn boundary_index(boundaries: &[usize], offset: usize) -> usize {
    let idx = boundaries.partition_point(|value| *value <= offset);
    idx.saturating_sub(1)
        .min(boundaries.len().saturating_sub(1))
}
