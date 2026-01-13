use crate::codegen::wasm::{RuntimeHook, ValueType};
use crate::error::Error;
use crate::mir::{MmioOperand, Operand};
use crate::mmio::encode_flags;

use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_mmio_read(
        &mut self,
        buf: &mut Vec<u8>,
        spec: &MmioOperand,
    ) -> Result<ValueType, Error> {
        let address = self.mmio_address(spec)?;
        emit_instruction(buf, Op::I64Const(address));
        emit_instruction(buf, Op::I32Const(spec.width_bits as i32));
        emit_instruction(buf, Op::I32Const(self.mmio_flags(spec)));
        let hook = self.runtime_hook_index(RuntimeHook::MmioRead)?;
        emit_instruction(buf, Op::Call(hook));
        if spec.width_bits <= 32 {
            emit_instruction(buf, Op::I32WrapI64);
            Ok(ValueType::I32)
        } else {
            Ok(ValueType::I64)
        }
    }

    pub(super) fn emit_mmio_store(
        &mut self,
        buf: &mut Vec<u8>,
        spec: &MmioOperand,
        value: &Operand,
    ) -> Result<(), Error> {
        let address = self.mmio_address(spec)?;
        emit_instruction(buf, Op::I64Const(address));
        let value_ty = self.emit_operand(buf, value)?;
        match value_ty {
            ValueType::I32 => emit_instruction(buf, Op::I64ExtendI32U),
            ValueType::I64 => {}
            other => {
                return Err(Error::Codegen(format!(
                    "unsupported value type {:?} for MMIO store",
                    other
                )));
            }
        }
        emit_instruction(buf, Op::I32Const(spec.width_bits as i32));
        emit_instruction(buf, Op::I32Const(self.mmio_flags(spec)));
        let hook = self.runtime_hook_index(RuntimeHook::MmioWrite)?;
        emit_instruction(buf, Op::Call(hook));
        Ok(())
    }

    fn mmio_address(&self, spec: &MmioOperand) -> Result<i64, Error> {
        let absolute = spec
            .base_address
            .checked_add(u64::from(spec.offset))
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "MMIO address for register {} overflows 64-bit range",
                    spec.name.as_deref().unwrap_or("<unknown>")
                ))
            })?;
        i64::try_from(absolute).map_err(|_| {
            Error::Codegen(format!(
                "MMIO address for register {} exceeds i64 range",
                spec.name.as_deref().unwrap_or("<unknown>")
            ))
        })
    }

    fn mmio_flags(&self, spec: &MmioOperand) -> i32 {
        encode_flags(spec.endianness, spec.address_space)
    }
}
