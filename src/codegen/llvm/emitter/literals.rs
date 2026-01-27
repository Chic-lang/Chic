use std::collections::HashMap;
use std::fmt::Write;

use crate::mir::{InternedStr, StrId};

// `Std::String` / `ChicString` layout: ptr, len, cap, inline[32]
pub(crate) const LLVM_STRING_TYPE: &str = "{ ptr, i64, i64, { [32 x i8] } }";
pub(crate) const LLVM_VEC_TYPE: &str = "{ i8*, i64, i64, i64, i64, ptr }";
pub(crate) const LLVM_STR_TYPE: &str = "{ ptr, i64 }";

#[derive(Debug, Clone)]
pub(crate) struct StrLiteralInfo {
    pub(crate) global: String,
    pub(crate) array_len: usize,
    pub(crate) data_len: usize,
}

pub(crate) fn emit_string_literals(
    out: &mut String,
    literals: &[InternedStr],
) -> HashMap<StrId, StrLiteralInfo> {
    let mut map = HashMap::new();
    if literals.is_empty() {
        return map;
    }

    for (index, literal) in literals.iter().enumerate() {
        let array_len = literal.value.len().max(1);
        let symbol = format!("@__chx_str_{index}");
        if literal.value.is_empty() {
            writeln!(
                out,
                "{symbol} = private unnamed_addr constant [{array_len} x i8] zeroinitializer"
            )
            .ok();
        } else {
            let encoded = encode_llvm_bytes(literal.value.as_bytes());
            writeln!(
                out,
                "{symbol} = private unnamed_addr constant [{array_len} x i8] c\"{encoded}\""
            )
            .ok();
        }
        map.insert(
            literal.id,
            StrLiteralInfo {
                global: symbol,
                array_len,
                data_len: literal.value.len(),
            },
        );
    }
    writeln!(out).ok();
    map
}

pub(crate) fn encode_llvm_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::new();
    for &byte in bytes {
        match byte {
            b'\"' => encoded.push_str("\\22"),
            b'\\' => encoded.push_str("\\5C"),
            0x20..=0x7E => encoded.push(byte as char),
            _ => {
                encoded.push_str(&format!("\\{:02X}", byte));
            }
        }
    }
    encoded
}
