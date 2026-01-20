use super::FunctionEmitter;
use super::ops::{Op, emit_instruction};
use crate::codegen::wasm::ValueType;
use crate::error::Error;
use crate::mir::{BasicBlock, Terminator};

mod async_ops;
mod calls;
mod control_flow;
mod matches;
mod place;
mod switch_int;

const FUTURE_FLAG_READY: i32 = 0x0000_0001;
const FUTURE_FLAG_COMPLETED: i32 = 0x0000_0002;
const AWAIT_READY: i32 = 1;

#[derive(Clone, Copy)]
struct FutureResultLayout {
    offset: u32,
    value_ty: ValueType,
}

#[derive(Clone, Copy)]
struct AsyncTaskLayout {
    task_header_vtable_offset: u32,
    task_header_flags_offset: u32,
    task_flags_offset: u32,
    inner_future_header_flags_offset: u32,
    inner_future_completed_offset: u32,
    inner_future_result_offset: u32,
}

fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + align - 1) / align * align
    }
}

fn wasm_async_debug_log(message: impl AsRef<str>) {
    if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_none() {
        return;
    }
    eprintln!("[wasm-async] {}", message.as_ref());
}

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_block_terminator(
        &mut self,
        code: &mut Vec<u8>,
        block: &BasicBlock,
    ) -> Result<(), Error> {
        match &block.terminator {
            Some(term) => {
                wasm_debug!("      block {}: terminator {:?}", block.id, term);
                self.emit_terminator(code, block, term)?;
            }
            None => {
                wasm_debug!("      block {}: no terminator, inserting Br(0)", block.id);
                emit_instruction(code, Op::Br(0));
            }
        }
        Ok(())
    }

    fn emit_terminator(
        &mut self,
        buf: &mut Vec<u8>,
        block: &BasicBlock,
        term: &Terminator,
    ) -> Result<(), Error> {
        wasm_debug!("        emit_terminator {:?}", term);
        match term {
            Terminator::Goto { target } => {
                self.emit_goto(buf, *target);
                Ok(())
            }
            Terminator::Return => self.emit_return(buf),
            Terminator::SwitchInt {
                discr,
                targets,
                otherwise,
            } => self.emit_switch_int(buf, discr, targets, *otherwise),
            Terminator::Match {
                value,
                arms,
                otherwise,
            } => self.emit_match(buf, value, arms, *otherwise),
            Terminator::Call {
                func,
                args,
                arg_modes,
                destination,
                target,
                unwind,
                dispatch,
                ..
            } => self.emit_call(
                buf,
                calls::CallLowering {
                    func,
                    args,
                    modes: arg_modes,
                    destination: destination.as_ref(),
                    target: *target,
                    unwind: *unwind,
                    dispatch: dispatch.as_ref(),
                },
            ),
            Terminator::Throw { exception, ty } => self.emit_throw(buf, exception, ty),
            Terminator::Panic => {
                self.emit_runtime_panic(buf)?;
                Ok(())
            }
            Terminator::Unreachable => {
                Self::emit_trap(buf);
                Ok(())
            }
            Terminator::Await {
                future,
                destination,
                resume,
                drop,
            } => self.emit_await(buf, block, future, destination.as_ref(), *resume, *drop),
            Terminator::Yield {
                value,
                resume,
                drop,
            } => self.emit_yield(buf, block, value, *resume, *drop),
            Terminator::Pending(_) => Err(Error::Codegen(
                "WASM backend cannot lower pending terminators".into(),
            )),
        }
    }
}
