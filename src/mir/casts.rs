use std::cmp::Ordering;

use crate::primitives::{PrimitiveKind, PrimitiveRegistry};

/// Metadata describing a builtin integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntInfo {
    pub bits: u16,
    pub signed: bool,
}

/// Metadata describing a builtin floating-point type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatInfo {
    pub bits: u16,
}

/// Describes scalar builtin kinds recognised by cast lowering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    Int(IntInfo),
    Float(FloatInfo),
}

/// Returns the short type name without namespace qualifiers or nullable suffix.
#[must_use]
pub fn short_type_name(name: &str) -> &str {
    let trimmed = name.strip_suffix('?').unwrap_or(name);
    trimmed.rsplit("::").next().unwrap_or(trimmed)
}

/// Detects builtin scalar type information (integer or float).
#[must_use]
pub fn classify_scalar(
    registry: &PrimitiveRegistry,
    name: &str,
    pointer_size: u32,
) -> Option<ScalarKind> {
    let desc = registry.descriptor_for_name(name)?;
    match desc.kind {
        PrimitiveKind::Int {
            bits,
            signed,
            pointer_sized,
        } => {
            let width = if pointer_sized {
                pointer_size.saturating_mul(8) as u16
            } else {
                bits
            };
            Some(ScalarKind::Int(IntInfo {
                bits: width,
                signed,
            }))
        }
        PrimitiveKind::Float { bits } => Some(ScalarKind::Float(FloatInfo { bits })),
        PrimitiveKind::Char { bits } => Some(ScalarKind::Int(IntInfo {
            bits,
            signed: false,
        })),
        _ => None,
    }
}

/// Convenience helper that extracts integer metadata if the type is an integer.
#[must_use]
pub fn int_info(registry: &PrimitiveRegistry, name: &str, pointer_size: u32) -> Option<IntInfo> {
    match classify_scalar(registry, name, pointer_size)? {
        ScalarKind::Int(info) => Some(info),
        _ => None,
    }
}

/// Convenience helper that extracts floating-point metadata if the type is a float.
#[must_use]
pub fn float_info(registry: &PrimitiveRegistry, name: &str) -> Option<FloatInfo> {
    match registry.kind_for_name(name)? {
        PrimitiveKind::Float { bits } => Some(FloatInfo { bits: *bits }),
        _ => None,
    }
}

/// Returns `true` when the supplied type name refers to a builtin primitive.
#[must_use]
pub fn is_builtin_primitive(registry: &PrimitiveRegistry, name: &str) -> bool {
    registry.descriptor_for_name(name).is_some()
}

/// Detects whether the supplied type string represents a pointer.
#[must_use]
pub fn is_pointer_type(name: &str) -> bool {
    pointer_depth(name) > 0
}

/// Counts trailing `*` characters to determine pointer indirection level.
#[must_use]
pub fn pointer_depth(name: &str) -> usize {
    let mut slice = name.trim_start();
    let mut depth = 0usize;
    loop {
        let trimmed = slice.trim_start();
        if let Some(rest) = trimmed.strip_prefix("*mut") {
            depth += 1;
            slice = skip_pointer_attributes(rest);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("*const") {
            depth += 1;
            slice = skip_pointer_attributes(rest);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('*') {
            depth += 1;
            slice = skip_pointer_attributes(rest);
            continue;
        }
        break;
    }
    if depth > 0 {
        return depth;
    }
    name.trim_end()
        .chars()
        .rev()
        .take_while(|ch| *ch == '*')
        .count()
}

fn skip_pointer_attributes(mut slice: &str) -> &str {
    loop {
        let trimmed = slice.trim_start();
        if !trimmed.starts_with('@') {
            return trimmed;
        }
        let mut rest = &trimmed[1..];
        let mut ident_len = 0usize;
        for (idx, ch) in rest.char_indices() {
            if ch.is_alphanumeric() || ch == '_' {
                ident_len = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        if ident_len == 0 {
            return rest;
        }
        rest = &rest[ident_len..];
        let trimmed_rest = rest.trim_start();
        if trimmed_rest.starts_with('(') {
            let mut cursor = &trimmed_rest[1..];
            while let Some(ch) = cursor.chars().next() {
                let len = ch.len_utf8();
                cursor = &cursor[len..];
                if ch == ')' {
                    break;
                }
            }
            slice = cursor;
        } else {
            slice = trimmed_rest;
        }
    }
}

/// Reports whether casting from `source` to `target` may truncate integer values.
#[must_use]
pub fn int_cast_may_truncate(source: IntInfo, target: IntInfo) -> bool {
    match source.bits.cmp(&target.bits) {
        Ordering::Greater => true,
        Ordering::Equal => source.signed && !target.signed,
        Ordering::Less => false,
    }
}
