use std::fmt::Write;

use crate::error::Error;

use super::context::LlvmEmitContext;

const DROP_ENTRY_TYPE: &str = "%__chx_drop_entry";
const INIT_FN: &str = "__chic_init_drop_table";

pub(super) fn emit_drop_table(
    out: &mut String,
    ctx: &LlvmEmitContext<'_>,
) -> Result<Option<String>, Error> {
    let entries = ctx.drop_glue();
    if entries.is_empty() {
        return Ok(None);
    }

    writeln!(out, "{DROP_ENTRY_TYPE} = type {{ i64, ptr }}").ok();
    let count = entries.len();
    writeln!(
        out,
        "@__chic_drop_entries = private constant [{count} x {DROP_ENTRY_TYPE}] ["
    )
    .ok();
    for (index, glue) in entries.iter().enumerate() {
        let suffix = if index + 1 == count { "" } else { "," };
        let type_id = i64::from_ne_bytes(glue.type_identity.to_ne_bytes());
        writeln!(
            out,
            "    {DROP_ENTRY_TYPE} {{ i64 {type_id}, ptr @{symbol} }}{suffix}",
            type_id = type_id,
            symbol = glue.symbol
        )
        .ok();
    }
    writeln!(out, "]").ok();

    writeln!(out, "define internal void @{INIT_FN}() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(
        out,
        "  call void @chic_rt_install_drop_table(ptr getelementptr inbounds ([{count} x {DROP_ENTRY_TYPE}], ptr @__chic_drop_entries, i32 0, i32 0), i64 {count})"
    )
    .ok();
    writeln!(out, "  ret void").ok();
    writeln!(out, "}}").ok();

    writeln!(out).ok();

    Ok(Some(INIT_FN.to_string()))
}
