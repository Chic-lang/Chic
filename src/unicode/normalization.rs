//! Canonical NFC normalisation built from generated Unicode 17 tables.

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/generated/unicode17/normalization.rs"
));

const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

/// Normalise text to Unicode NFC.
#[must_use]
pub fn normalize_nfc(input: &str) -> String {
    let mut decomposed: Vec<(u32, u8)> = Vec::with_capacity(input.len());
    for ch in input.chars() {
        decompose_scalar(ch as u32, &mut decomposed);
    }

    // Canonical ordering: reorder combining marks within each starter segment.
    let mut ordered: Vec<(u32, u8)> = Vec::with_capacity(decomposed.len());
    let mut cursor = 0usize;
    while cursor < decomposed.len() {
        let (scalar, class) = decomposed[cursor];
        ordered.push((scalar, class));
        cursor += 1;

        let start = ordered.len();
        while cursor < decomposed.len() && decomposed[cursor].1 != 0 {
            ordered.push(decomposed[cursor]);
            cursor += 1;
        }
        let end = ordered.len();
        ordered[start..end].sort_by(|a, b| a.1.cmp(&b.1));
    }

    compose(&ordered)
}

/// Returns true when the text is already in NFC form.
#[must_use]
pub fn is_nfc(input: &str) -> bool {
    normalize_nfc(input) == input
}

fn compose(decomposed: &[(u32, u8)]) -> String {
    if decomposed.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(decomposed.len());
    let mut starter = decomposed[0].0;
    let mut starter_ccc = decomposed[0].1;
    let mut last_ccc = starter_ccc;
    out.push(char::from_u32(starter).expect("valid Unicode scalar"));

    for &(scalar, class) in decomposed.iter().skip(1) {
        let mut composed = None;
        if starter_ccc == 0 && (last_ccc < class || last_ccc == 0) {
            composed = try_compose(starter, scalar);
        }

        if let Some(value) = composed {
            starter = value;
            starter_ccc = canonical_combining_class(value);
            last_ccc = starter_ccc;
            out.pop();
            out.push(char::from_u32(value).expect("valid Unicode scalar"));
            continue;
        }

        if class == 0 {
            starter = scalar;
            starter_ccc = 0;
        }
        last_ccc = class;
        out.push(char::from_u32(scalar).expect("valid Unicode scalar"));
    }

    out
}

fn decompose_scalar(scalar: u32, out: &mut Vec<(u32, u8)>) {
    if let Some(hangul) = decompose_hangul(scalar) {
        out.extend(hangul);
        return;
    }

    if let Some(mapping) = canonical_decomposition(scalar) {
        for part in mapping {
            decompose_scalar(*part, out);
        }
        return;
    }

    out.push((scalar, canonical_combining_class(scalar)));
}

fn canonical_decomposition(scalar: u32) -> Option<&'static [u32]> {
    let idx = CANONICAL_DECOMPOSITIONS.partition_point(|entry| entry.0 < scalar);
    CANONICAL_DECOMPOSITIONS
        .get(idx)
        .filter(|(value, _)| *value == scalar)
        .map(|(_, mapping)| *mapping)
}

fn canonical_combining_class(scalar: u32) -> u8 {
    let idx = CANONICAL_COMBINING_CLASSES.partition_point(|entry| entry.0 < scalar);
    CANONICAL_COMBINING_CLASSES
        .get(idx)
        .filter(|(value, _)| *value == scalar)
        .map(|(_, class)| *class)
        .unwrap_or(0)
}

fn try_compose(starter: u32, combining: u32) -> Option<u32> {
    if let Some(composed) = compose_hangul(starter, combining) {
        return Some(composed);
    }
    let idx = CANONICAL_COMPOSITIONS.partition_point(|entry| entry.0 < starter);
    let mut cursor = idx;
    while let Some(&(base, comb, composite)) = CANONICAL_COMPOSITIONS.get(cursor) {
        if base != starter {
            break;
        }
        if comb == combining {
            return Some(composite);
        }
        cursor += 1;
    }
    None
}

fn decompose_hangul(scalar: u32) -> Option<Vec<(u32, u8)>> {
    if !(S_BASE..S_BASE + S_COUNT).contains(&scalar) {
        return None;
    }
    let s_index = scalar - S_BASE;
    let l = L_BASE + s_index / N_COUNT;
    let v = V_BASE + (s_index % N_COUNT) / T_COUNT;
    let t = T_BASE + s_index % T_COUNT;

    let mut out = Vec::with_capacity(3);
    out.push((l, canonical_combining_class(l)));
    out.push((v, canonical_combining_class(v)));
    if t != T_BASE {
        out.push((t, canonical_combining_class(t)));
    }
    Some(out)
}

fn compose_hangul(starter: u32, combining: u32) -> Option<u32> {
    let l_index = starter.wrapping_sub(L_BASE);
    let v_index = combining.wrapping_sub(V_BASE);
    if l_index < L_COUNT && v_index < V_COUNT {
        let s_index = l_index * N_COUNT + v_index * T_COUNT;
        return Some(S_BASE + s_index);
    }

    let t_index = combining.wrapping_sub(T_BASE);
    if t_index > 0 && t_index < T_COUNT {
        if (starter - S_BASE) < S_COUNT && (starter - S_BASE) % T_COUNT == 0 {
            return Some(starter + t_index);
        }
    }

    None
}
