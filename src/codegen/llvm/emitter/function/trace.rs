//! LLVM tracepoint lowering helpers scoped to the function emitter.

use std::fmt::Write;

use super::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
use crate::error::Error;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_trace_enter(&mut self) -> Result<(), Error> {
        let Some(trace) = self.tracepoint else {
            return Ok(());
        };
        let Some(label_id) = trace.label_id else {
            return Ok(());
        };
        let Some(StrLiteralInfo {
            global,
            array_len,
            data_len,
        }) = self.str_literals.get(&label_id)
        else {
            return Ok(());
        };

        self.externals.insert("chic_rt_trace_enter");
        let label_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {label_tmp} = getelementptr inbounds [{array_len} x i8], ptr {global}, i32 0, i32 0"
        )
        .ok();
        let cpu_budget = trace
            .budget
            .as_ref()
            .and_then(|cost| cost.cpu_budget_us)
            .unwrap_or(0);
        let mem_budget = trace
            .budget
            .as_ref()
            .and_then(|cost| cost.mem_budget_bytes)
            .unwrap_or(0);
        let gpu_budget = trace
            .budget
            .as_ref()
            .and_then(|cost| cost.gpu_budget_us)
            .unwrap_or(0);
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_trace_enter(i64 {}, ptr {label_tmp}, i64 {data_len}, i64 {cpu_budget}, i64 {mem_budget}, i64 {gpu_budget})",
            trace.trace_id
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_trace_exit(&mut self) {
        let Some(trace) = self.tracepoint else {
            return;
        };
        self.externals.insert("chic_rt_trace_exit");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_trace_exit(i64 {})",
            trace.trace_id
        )
        .ok();
    }
}
