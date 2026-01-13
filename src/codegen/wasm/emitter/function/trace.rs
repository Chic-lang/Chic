//! WASM tracepoint lowering utilities.

use std::convert::TryFrom;

use super::builder::FunctionEmitter;
use super::ops::{Op, emit_instruction};
use crate::codegen::wasm::RuntimeHook;
use crate::error::Error;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_trace_enter(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        let Some(trace) = self.tracepoint else {
            return Ok(());
        };
        let Some(label_id) = trace.label_id else {
            return Ok(());
        };
        let Some(literal) = self.string_literals.get(&label_id) else {
            return Ok(());
        };
        let hook = self.runtime_hook_index(RuntimeHook::TraceEnter)?;
        let offset = i32::try_from(literal.offset).map_err(|_| {
            Error::Codegen("trace label offset exceeds WebAssembly address space".into())
        })?;
        let len = i64::try_from(literal.len)
            .map_err(|_| Error::Codegen("trace label length exceeds WebAssembly limits".into()))?;
        emit_instruction(buf, Op::I64Const(trace.trace_id as i64));
        emit_instruction(buf, Op::I32Const(offset));
        emit_instruction(buf, Op::I64Const(len));
        emit_instruction(
            buf,
            Op::I64Const(
                trace
                    .budget
                    .as_ref()
                    .and_then(|cost| cost.cpu_budget_us)
                    .unwrap_or(0) as i64,
            ),
        );
        emit_instruction(
            buf,
            Op::I64Const(
                trace
                    .budget
                    .as_ref()
                    .and_then(|cost| cost.mem_budget_bytes)
                    .unwrap_or(0) as i64,
            ),
        );
        emit_instruction(
            buf,
            Op::I64Const(
                trace
                    .budget
                    .as_ref()
                    .and_then(|cost| cost.gpu_budget_us)
                    .unwrap_or(0) as i64,
            ),
        );
        emit_instruction(buf, Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_trace_exit(&self, buf: &mut Vec<u8>) -> Result<(), Error> {
        let Some(trace) = self.tracepoint else {
            return Ok(());
        };
        let hook = self.runtime_hook_index(RuntimeHook::TraceExit)?;
        emit_instruction(buf, Op::I64Const(trace.trace_id as i64));
        emit_instruction(buf, Op::Call(hook));
        Ok(())
    }
}
