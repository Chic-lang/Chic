use crate::codegen::wasm::RuntimeHook;
use crate::codegen::wasm::{ValueType, ensure_u32};
use crate::error::Error;
use crate::mir::{
    AtomicFenceScope, AtomicOrdering, BasicBlock, Operand, Place, Statement, StatementKind,
};

use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_block_statements(
        &mut self,
        code: &mut Vec<u8>,
        block: &BasicBlock,
    ) -> Result<(), Error> {
        for statement in &block.statements {
            wasm_debug!(
                "      block {}: statement kind {:?}",
                block.id,
                statement.kind
            );
            self.emit_statement(code, statement)?;
        }
        Ok(())
    }

    fn emit_statement(&mut self, buf: &mut Vec<u8>, statement: &Statement) -> Result<(), Error> {
        self.emit_coverage_hit(buf, statement)?;
        match &statement.kind {
            StatementKind::Assign { place, value } => self.emit_assign(buf, place, value),
            StatementKind::Assert { cond, expected, .. } => {
                let cond_ty = self.emit_operand(buf, cond)?;
                match cond_ty {
                    ValueType::I32 => {}
                    ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
                    other => {
                        return Err(Error::Codegen(format!(
                            "assert condition must be integral, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::I32Const(i32::from(*expected)));
                emit_instruction(buf, Op::I32Eq);
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::If);
                self.emit_runtime_panic_with_code(buf, 0x2100)?;
                emit_instruction(buf, Op::End);
                Ok(())
            }
            StatementKind::StorageLive(_) => Ok(()),
            StatementKind::StorageDead(local) => self.emit_storage_dead(buf, *local),
            StatementKind::Drop { place, .. } => self.emit_drop_statement(buf, place),
            StatementKind::Borrow {
                borrow_id,
                kind,
                place,
                ..
            } => self.emit_borrow_statement(buf, *borrow_id, *kind, place),
            StatementKind::Deinit(place) => self.emit_deinit_statement(buf, place),
            StatementKind::DefaultInit { place } => self.emit_zero_init(buf, place),
            StatementKind::ZeroInit { place } => self.emit_zero_init(buf, place),
            StatementKind::ZeroInitRaw { pointer, length } => {
                self.emit_zero_init_raw(buf, pointer, length)
            }
            StatementKind::MmioStore { target, value } => self.emit_mmio_store(buf, target, value),
            StatementKind::StaticStore { id, value } => self.emit_static_store(buf, *id, value),
            StatementKind::AtomicStore {
                target,
                value,
                order,
            } => self.emit_atomic_store(buf, target, value, *order),
            StatementKind::AtomicFence { order, scope } => {
                self.emit_atomic_fence(buf, *order, *scope)
            }
            StatementKind::InlineAsm(_) => Err(Error::Codegen(
                "inline assembly is not supported by the WASM backend".into(),
            )),
            StatementKind::EnterUnsafe
            | StatementKind::ExitUnsafe
            | StatementKind::Retag { .. }
            | StatementKind::DeferDrop { .. }
            | StatementKind::Eval(_)
            | StatementKind::EnqueueKernel { .. }
            | StatementKind::EnqueueCopy { .. }
            | StatementKind::RecordEvent { .. }
            | StatementKind::WaitEvent { .. }
            | StatementKind::MarkFallibleHandled { .. }
            | StatementKind::Pending(_)
            | StatementKind::Nop => Ok(()),
        }
    }

    fn emit_coverage_hit(&mut self, buf: &mut Vec<u8>, statement: &Statement) -> Result<(), Error> {
        if !self.coverage_enabled {
            return Ok(());
        }
        let key = statement as *const _ as usize;
        let statement_index = self
            .coverage_statement_indices
            .get(&key)
            .copied()
            .unwrap_or_else(|| {
                let fallback = self.coverage_statement_index;
                self.coverage_statement_index = self.coverage_statement_index.wrapping_add(1);
                fallback
            });
        let id = ((self.coverage_function_index as u64) << 32) | (statement_index as u64);
        if statement.span.is_none() {
            return Ok(());
        }
        let hook = self.runtime_hook_index(RuntimeHook::CoverageHit)?;
        emit_instruction(buf, Op::I64Const(id as i64));
        emit_instruction(buf, Op::Call(hook));
        Ok(())
    }

    pub(crate) fn emit_zero_init(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<(), Error> {
        let access = self.resolve_memory_access(place)?;
        let (size, _) = self
            .layouts
            .size_and_align_for_ty(&access.value_ty)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout information for `{}` during ZeroInit lowering",
                    access.value_ty.canonical_name()
                ))
            })?;
        if size == 0 {
            return Ok(());
        }
        self.emit_pointer_expression(buf, &access)?;
        let bytes = ensure_u32(size, "ZeroInit size exceeds wasm range")?;
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::I32Const(bytes as i32));
        emit_instruction(buf, Op::MemoryFill);
        Ok(())
    }

    fn emit_zero_init_raw(
        &mut self,
        buf: &mut Vec<u8>,
        pointer: &Operand,
        length: &Operand,
    ) -> Result<(), Error> {
        let pointer_ty = self.emit_operand(buf, pointer)?;
        match pointer_ty {
            ValueType::I32 => {}
            ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
            other => {
                return Err(Error::Codegen(format!(
                    "ZeroInitRaw expects pointer operands, found {:?}",
                    other
                )));
            }
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        let len_ty = self.emit_operand(buf, length)?;
        match len_ty {
            ValueType::I32 => {}
            ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
            other => {
                return Err(Error::Codegen(format!(
                    "ZeroInitRaw length must be integer, found {:?}",
                    other
                )));
            }
        }
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::MemoryFill);
        Ok(())
    }

    fn emit_atomic_fence(
        &mut self,
        buf: &mut Vec<u8>,
        order: AtomicOrdering,
        _scope: AtomicFenceScope,
    ) -> Result<(), Error> {
        FunctionEmitter::check_atomic_order(order)?;
        if matches!(order, AtomicOrdering::Relaxed) {
            return Ok(());
        }
        emit_instruction(buf, Op::AtomicFence);
        Ok(())
    }
}
