use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};
use crate::error::Error;
use crate::mir::{BlockId, Operand};
use std::convert::TryFrom;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_switch_int(
        &mut self,
        buf: &mut Vec<u8>,
        discr: &Operand,
        targets: &[(i128, BlockId)],
        otherwise: BlockId,
    ) -> Result<(), Error> {
        wasm_debug!(
            "        lowering SwitchInt with {} targets and otherwise {}",
            targets.len(),
            otherwise
        );
        self.emit_operand(buf, discr)?;
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        for (value, block) in targets {
            wasm_debug!("          compare against literal {}", value);
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            let literal = Self::convert_switch_value(*value)?;
            emit_instruction(buf, Op::I32Const(literal));
            emit_instruction(buf, Op::I32Eq);
            emit_instruction(buf, Op::If);
            self.set_block(buf, *block);
            emit_instruction(buf, Op::Br(2));
            emit_instruction(buf, Op::End);
        }
        self.emit_match_default(buf, otherwise);
        Ok(())
    }

    pub(super) fn convert_switch_value(value: i128) -> Result<i32, Error> {
        i32::try_from(value).map_err(|_| {
            Error::Codegen(
                "switch literal outside 32-bit range is unsupported by the WASM backend".into(),
            )
        })
    }
}
