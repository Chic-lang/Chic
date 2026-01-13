use std::collections::HashSet;

pub(super) struct RuntimeImport {
    pub(super) module: &'static str,
    pub(super) name: &'static str,
    pub(super) type_index: u32,
}

pub(super) struct DataSegment {
    pub(super) offset: u32,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn align_u32(value: u32, align: u32) -> u32 {
    if align <= 1 {
        return value;
    }
    let mask = align - 1;
    (value + mask) & !mask
}

pub(super) fn make_unique_label(
    raw: &str,
    fallback_index: usize,
    used: &mut HashSet<String>,
) -> String {
    let mut sanitized: String = raw
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect();
    if sanitized.is_empty() {
        sanitized = format!("func{fallback_index}");
    }
    if sanitized
        .as_bytes()
        .first()
        .map_or(false, |byte| byte.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }

    let mut candidate = sanitized.clone();
    let mut suffix = 1;
    while !used.insert(candidate.clone()) {
        candidate = format!("{sanitized}_{suffix}");
        suffix += 1;
    }
    candidate
}
