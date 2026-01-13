use crate::mir::ConstValue;
use std::char;

pub(super) fn format_const_value(value: &ConstValue) -> String {
    match value {
        ConstValue::Int(i) | ConstValue::Int32(i) => i.to_string(),
        ConstValue::UInt(u) => u.to_string(),
        ConstValue::Float(f) => {
            let value = f.to_f64();
            if value.is_finite() {
                let mut text = f.display();
                if !text.contains(['.', 'e', 'E']) {
                    text.push_str(".0");
                }
                text
            } else {
                f.display()
            }
        }
        ConstValue::Decimal(value) => value.into_decimal().to_string(),
        ConstValue::Bool(b) => b.to_string(),
        ConstValue::Char(c) => format!("'{}'", escape_char(*c)),
        ConstValue::Str { value, .. } | ConstValue::RawStr(value) => {
            format!("\"{}\"", escape_string(value))
        }
        ConstValue::Symbol(sym) => sym.clone(),
        ConstValue::Enum {
            type_name, variant, ..
        } => {
            let short = type_name.rsplit("::").next().unwrap_or(type_name);
            format!("{short}::{variant}")
        }
        ConstValue::Struct { type_name, .. } => {
            let short = type_name.rsplit("::").next().unwrap_or(type_name);
            format!("{short} {{ .. }}")
        }
        ConstValue::Null => "null".to_string(),
        ConstValue::Unit => "()".to_string(),
        ConstValue::Unknown => "<unknown>".to_string(),
    }
}

pub(super) fn escape_char(value: u16) -> String {
    if let Some(ch) = char::from_u32(u32::from(value)) {
        ch.escape_default().collect()
    } else {
        format!("\\u{:04X}", value)
    }
}

pub(super) fn escape_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        escaped.extend(ch.escape_default());
    }
    escaped
}

pub(super) fn escape_interpolated_text(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        match ch {
            '{' => escaped.push_str("{{"),
            '}' => escaped.push_str("}}"),
            _ => escaped.extend(ch.escape_default()),
        }
    }
    escaped
}
