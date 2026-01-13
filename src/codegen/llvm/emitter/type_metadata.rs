use std::fmt::Write;

use super::context::LlvmEmitContext;
use crate::error::Error;

const ENTRY_TYPE: &str = "%__chx_type_metadata";
const INIT_FN: &str = "__chic_init_type_metadata";

pub(super) fn emit_type_metadata_table(
    out: &mut String,
    ctx: &LlvmEmitContext<'_>,
) -> Result<Option<String>, Error> {
    let entries = ctx.type_metadata();
    if entries.is_empty() {
        return Ok(None);
    }

    writeln!(
        out,
        "{ENTRY_TYPE} = type {{ i64, i64, i64, ptr, ptr, i64, i32 }}"
    )
    .ok();
    let mut variance_refs = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if entry.variances.is_empty() {
            variance_refs.push((String::from("ptr null"), 0usize));
            continue;
        }
        let symbol = format!("@__chx_type_var_{index}");
        let len = entry.variances.len();
        let mut values = String::new();
        values.push('[');
        for (i, variance) in entry.variances.iter().enumerate() {
            let suffix = if i + 1 == len { "" } else { ", " };
            let _ = write!(values, "i8 {}{suffix}", variance.encode());
        }
        values.push(']');
        writeln!(
            out,
            "{symbol} = private constant [{len} x i8] {values}",
            symbol = symbol,
            len = len,
            values = values
        )
        .ok();
        let ptr = format!(
            "ptr getelementptr inbounds ([{len} x i8], ptr {symbol}, i32 0, i32 0)",
            len = len,
            symbol = symbol
        );
        variance_refs.push((ptr, len));
    }
    let count = entries.len();
    writeln!(
        out,
        "@__chic_type_metadata = private constant [{count} x {ENTRY_TYPE}] ["
    )
    .ok();
    for (index, entry) in entries.iter().enumerate() {
        let suffix = if index + 1 == count { "" } else { "," };
        let drop_symbol = entry
            .drop_symbol
            .as_deref()
            .map(|symbol| format!("ptr @{symbol}"))
            .unwrap_or_else(|| "ptr null".into());
        let (variance_ptr, variance_len) = &variance_refs[index];
        let type_id = i64::from_ne_bytes(entry.type_identity.to_ne_bytes());
        writeln!(
            out,
            "    {ENTRY_TYPE} {{ i64 {type_id}, i64 {size}, i64 {align}, {drop_symbol}, {variance_ptr}, i64 {variance_len}, i32 {flags} }}{suffix}",
            type_id = type_id,
            size = entry.size,
            align = entry.align,
            variance_ptr = variance_ptr,
            variance_len = variance_len,
            flags = entry.flags.bits(),
        )
        .ok();
    }
    writeln!(out, "]").ok();

    writeln!(out, "define internal void @{INIT_FN}() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(
        out,
        "  call void @chic_rt_install_type_metadata(ptr getelementptr inbounds ([{count} x {ENTRY_TYPE}], ptr @__chic_type_metadata, i32 0, i32 0), i64 {count})"
    )
    .ok();
    writeln!(out, "  ret void").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    Ok(Some(INIT_FN.to_string()))
}
