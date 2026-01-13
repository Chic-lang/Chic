use std::convert::TryFrom;

use crate::error::Error;
use crate::mir::BasicBlock;

use super::FunctionEmitter;
use super::ops::{Op, emit_instruction};

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_blocks(&mut self, code: &mut Vec<u8>) -> Result<(), Error> {
        for (idx, block) in self.function.body.blocks.iter().enumerate() {
            wasm_debug!(
                "    emit_body `{}`: visiting block {} (idx {}, {} statements)",
                self.function.name,
                block.id,
                idx,
                block.statements.len()
            );
            self.emit_block(code, block, idx)?;
        }
        Ok(())
    }
    pub(crate) fn emit_block(
        &mut self,
        code: &mut Vec<u8>,
        block: &BasicBlock,
        idx: usize,
    ) -> Result<(), Error> {
        self.emit_block_selector(code, idx)?;
        self.emit_block_statements(code, block)?;
        self.emit_block_terminator(code, block)?;
        emit_instruction(code, Op::End);
        Ok(())
    }

    fn emit_block_selector(&self, code: &mut Vec<u8>, idx: usize) -> Result<(), Error> {
        emit_instruction(code, Op::LocalGet(self.block_local));
        let block_idx = i32::try_from(idx)
            .map_err(|_| Error::Codegen("basic block index exceeds WebAssembly limits".into()))?;
        emit_instruction(code, Op::I32Const(block_idx));
        emit_instruction(code, Op::I32Eq);
        emit_instruction(code, Op::If);
        Ok(())
    }
    pub(crate) fn emit_epilogue(&self, code: &mut Vec<u8>) {
        emit_instruction(code, Op::Unreachable);
        emit_instruction(code, Op::End); // loop
        emit_instruction(code, Op::End); // block
        if let Some(ret) = self.return_local {
            emit_instruction(code, Op::LocalGet(ret));
        }
        wasm_debug!("    emit_body `{}`: final Return", self.function.name);
        emit_instruction(code, Op::Return);
        emit_instruction(code, Op::End);
    }
}
