//! Shared helpers for MIR trait infrastructure.

/// Produce the mangled vtable symbol name for a `(trait, impl)` pairing.
///
/// This mirrors the lowering behaviour used by the module builder so every
/// consumer (MIR builders, interpreters, backends) agrees on the exact
/// identifier that should be referenced inside the object file.
#[must_use]
pub fn trait_vtable_symbol_name(trait_name: &str, impl_name: &str) -> String {
    fn sanitize(raw: &str) -> String {
        let mut text = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => text.push(ch),
                ':' => {
                    if chars.peek() == Some(&':') {
                        chars.next();
                    }
                    text.push_str("__");
                }
                _ => text.push('_'),
            }
        }
        if text.is_empty() {
            text.push_str("anon");
        }
        if text
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            text.insert(0, '_');
        }
        text
    }

    let trait_part = sanitize(trait_name);
    let impl_part = sanitize(impl_name);
    format!("__vtable_{trait_part}__{impl_part}")
}

/// Produce the mangled vtable symbol name for a concrete class type.
#[must_use]
pub fn class_vtable_symbol_name(type_name: &str) -> String {
    fn sanitize(raw: &str) -> String {
        let mut text = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => text.push(ch),
                ':' => {
                    if chars.peek() == Some(&':') {
                        chars.next();
                    }
                    text.push_str("__");
                }
                _ => text.push('_'),
            }
        }
        if text.is_empty() {
            text.push_str("anon");
        }
        if text
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            text.insert(0, '_');
        }
        text
    }

    let ty_part = sanitize(type_name);
    format!("__class_vtable_{ty_part}")
}
