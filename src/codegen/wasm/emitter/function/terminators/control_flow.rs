use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};
use crate::error::Error;
use crate::mir::BlockId;
use std::convert::TryFrom;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_goto(&mut self, buf: &mut Vec<u8>, target: BlockId) {
        self.set_block(buf, target);
        emit_instruction(buf, Op::Br(1));
    }

    pub(super) fn emit_return(&mut self, buf: &mut Vec<u8>) -> Result<(), Error> {
        if self.function.is_async {
            if self.emit_ready_task_return(buf)? {
                return Ok(());
            }
            if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
                eprintln!(
                    "[wasm-async] emit_return fell back for async function {} (ready_task_return declined)",
                    self.function.name
                );
            }
        }
        self.emit_trace_exit(buf)?;
        self.emit_frame_teardown(buf);
        emit_instruction(buf, Op::Br(2));
        Ok(())
    }

    pub(super) fn emit_trap(buf: &mut Vec<u8>) {
        emit_instruction(buf, Op::Unreachable);
    }

    pub(super) fn set_block(&self, buf: &mut Vec<u8>, block: BlockId) {
        wasm_debug!("          set_block {}", block);
        match self
            .block_map
            .get(&block)
            .and_then(|idx| i32::try_from(*idx).ok())
        {
            Some(value) => {
                emit_instruction(buf, Op::I32Const(value));
                emit_instruction(buf, Op::LocalSet(self.block_local));
            }
            None => emit_instruction(buf, Op::Unreachable),
        }
    }
}
