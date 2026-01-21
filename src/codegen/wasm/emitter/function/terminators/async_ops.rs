use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};
use crate::codegen::wasm::{RuntimeHook, ValueType, ensure_u32, map_type};
use crate::error::Error;
use crate::mir::async_types::{future_result_ty, is_future_ty, is_task_ty, task_result_ty};
use crate::mir::{BasicBlock, BlockId, GenericArg, LocalId, Operand, Place, StructLayout, Ty};

use super::{
    AWAIT_READY, AsyncTaskLayout, FUTURE_FLAG_COMPLETED, FUTURE_FLAG_READY, FutureResultLayout,
    align_to, wasm_async_debug_log,
};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_ready_task_return(&mut self, buf: &mut Vec<u8>) -> Result<bool, Error> {
        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
            eprintln!(
                "[wasm-async] ready_task_return {} invoked",
                self.function.name
            );
        }
        let Some(ret_local) = self.return_local else {
            if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
                eprintln!(
                    "[wasm-async] ready_task_return {} skipped: no return_local",
                    self.function.name
                );
            }
            return Ok(false);
        };
        wasm_async_debug_log(format!(
            "[wasm-async] ready_task_return {} invoked return_local={ret_local:?}",
            self.function.name
        ));
        let async_result_ty = self
            .function
            .body
            .async_machine
            .as_ref()
            .and_then(|machine| machine.result_ty.as_ref())
            .or(self.function.async_result.as_ref());
        let Some(async_result_ty) = async_result_ty else {
            return Ok(false);
        };
        let result_local_from_async_state = self
            .function
            .body
            .async_machine
            .as_ref()
            .and_then(|machine| machine.result_local);
        let result_local = result_local_from_async_state
            .or_else(|| {
                self.function
                    .body
                    .locals
                    .iter()
                    .position(|decl| decl.name.as_deref() == Some("async_result"))
                    .map(LocalId)
            })
            .or_else(|| {
                self.function
                    .body
                    .locals
                    .iter()
                    .position(|decl| &decl.ty == async_result_ty)
                    .map(LocalId)
            })
            .or_else(|| {
                let canonical = async_result_ty.canonical_name();
                self.function
                    .body
                    .locals
                    .iter()
                    .position(|decl| decl.ty.canonical_name() == canonical)
                    .map(LocalId)
            })
            .or_else(|| {
                self.function
                    .body
                    .locals
                    .iter()
                    .position(|decl| decl.name.as_deref() == Some("_ret"))
                    .map(LocalId)
            });
        let Some(result_local) = result_local else {
            if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
                let locals: Vec<_> = self
                    .function
                    .body
                    .locals
                    .iter()
                    .map(|decl| (decl.name.clone(), decl.ty.canonical_name()))
                    .collect();
                eprintln!(
                    "[wasm-async] ready_task_return {} missing result local; async_result_ty={} locals={:?}",
                    self.function.name,
                    async_result_ty.canonical_name(),
                    locals
                );
            }
            return Ok(false);
        };
        wasm_async_debug_log(format!(
            "[wasm-async] ready_task_return {} result_local={:?} from_state_machine={} async_result_ty={}",
            self.function.name,
            result_local,
            result_local_from_async_state.is_some(),
            async_result_ty.canonical_name(),
        ));
        if self.function.name.contains("AsyncWorkflow") {
            eprintln!(
                "[wasm-async] ready_task_return {} using result_local {:?} return_local={ret_local:?} from_state_machine={}",
                self.function.name,
                result_local,
                result_local_from_async_state.is_some()
            );
        }
        if let (Some(frame_local), Some(offset)) =
            (self.frame_local, self.return_local_frame_offset())
        {
            emit_instruction(buf, Op::LocalGet(frame_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::LocalSet(ret_local));
        }
        let layout = self.async_task_layout(async_result_ty)?;
        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
            let local_desc = self.function.body.locals.get(result_local.0).map(|decl| {
                (
                    decl.name
                        .clone()
                        .unwrap_or_else(|| format!("_{}", result_local.0)),
                    decl.ty.canonical_name(),
                )
            });
            eprintln!(
                "[wasm-async] ready_task_return {} result_local={:?} local_desc={:?} vtable_off={} header_flags_off={} task_flags_off={} inner_flags_off={} inner_completed_off={} inner_result_off={}",
                self.function.name,
                result_local,
                local_desc,
                layout.task_header_vtable_offset,
                layout.task_header_flags_offset,
                layout.task_flags_offset,
                layout.inner_future_header_flags_offset,
                layout.inner_future_completed_offset,
                layout.inner_future_result_offset,
            );
            wasm_async_debug_log(format!(
                "[wasm-async] ready_task_return {} layout: vtable_off={} header_flags_off={} task_flags_off={} inner_flags_off={} inner_completed_off={} inner_result_off={}",
                self.function.name,
                layout.task_header_vtable_offset,
                layout.task_header_flags_offset,
                layout.task_flags_offset,
                layout.inner_future_header_flags_offset,
                layout.inner_future_completed_offset,
                layout.inner_future_result_offset,
            ));
        }
        if let Some(offset) = self.async_vtable_offsets.get(&self.function.name) {
            emit_instruction(buf, Op::LocalGet(ret_local));
            emit_instruction(buf, Op::I32Const(layout.task_header_vtable_offset as i32));
            emit_instruction(buf, Op::I32Add);
            emit_instruction(buf, Op::I32Const(*offset as i32));
            emit_instruction(buf, Op::I32Store(0));
        }
        // Write ready/completed flags to the task header and outer task flags.
        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(buf, Op::I32Const(layout.task_header_flags_offset as i32));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Const(FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED));
        emit_instruction(buf, Op::I32Store(0));

        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(buf, Op::I32Const(layout.task_flags_offset as i32));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Const(FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED));
        emit_instruction(buf, Op::I32Store(0));

        // Mark the inner future as completed.
        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(
            buf,
            Op::I32Const(layout.inner_future_header_flags_offset as i32),
        );
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Const(FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED));
        emit_instruction(buf, Op::I32Store(0));

        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(
            buf,
            Op::I32Const(layout.inner_future_completed_offset as i32),
        );
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Store(0));

        // Store the async result into the inner future's Result field (i32-only for now).
        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(buf, Op::I32Const(layout.inner_future_result_offset as i32));
        emit_instruction(buf, Op::I32Add);
        let value_ty = self.emit_place_value(buf, &Place::new(result_local))?;
        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
            eprintln!(
                "[wasm-async] ready_task_return {} storing async_result local {:?} type={:?} into inner_result_off={}",
                self.function.name, result_local, value_ty, layout.inner_future_result_offset,
            );
        }
        match value_ty {
            ValueType::I32 => emit_instruction(buf, Op::I32Store(0)),
            other => {
                return Err(Error::Codegen(format!(
                    "async result type `{}` is unsupported by ready_task_return in WASM backend",
                    format!("{other:?}")
                )));
            }
        }

        self.emit_trace_exit(buf)?;
        emit_instruction(buf, Op::LocalGet(ret_local));
        emit_instruction(buf, Op::Return);
        Ok(true)
    }

    pub(super) fn emit_await(
        &mut self,
        buf: &mut Vec<u8>,
        block: &BasicBlock,
        future: &Place,
        destination: Option<&Place>,
        resume: BlockId,
        drop: BlockId,
    ) -> Result<(), Error> {
        wasm_debug!(
            "        lowering Await in block {} resume {} drop {}",
            block.id,
            resume,
            drop
        );
        let hook = self.runtime_hook_index(RuntimeHook::Await)?;
        let future_ty = self.emit_place_value(buf, future)?;
        if !matches!(future_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "WASM await lowering currently supports i32 future handles only".into(),
            ));
        }
        // Stash the future pointer.
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        // Runtime context (async executor state).
        if let Some(ctx) = self.async_context_pointer() {
            emit_instruction(buf, Op::LocalGet(ctx));
        } else {
            emit_instruction(buf, Op::I32Const(0));
        }
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        // ready?
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Const(AWAIT_READY));
        emit_instruction(buf, Op::I32Eq);
        emit_instruction(buf, Op::If);
        if let Some(place) = destination {
            let mir_ty = self.mir_place_ty(future)?;
            if is_task_ty(&mir_ty) {
                let result_ty = task_result_ty(&mir_ty).ok_or_else(|| {
                    Error::Codegen(
                        "await on Std.Async.Task without result type is unsupported in WASM backend"
                            .into(),
                    )
                })?;
                self.emit_task_result_store(buf, self.temp_local, &result_ty, place)?;
            } else if is_future_ty(&mir_ty) {
                let layout = self.future_result_layout(&mir_ty)?.ok_or_else(|| {
                    Error::Codegen(
                        "await operand missing future result layout in WASM backend".into(),
                    )
                })?;
                self.emit_store_future_result(buf, self.temp_local, &layout, place)?;
            }
        }
        self.emit_goto(buf, resume);
        emit_instruction(buf, Op::End);
        self.emit_goto(buf, drop);
        Ok(())
    }

    pub(super) fn emit_yield(
        &mut self,
        buf: &mut Vec<u8>,
        block: &BasicBlock,
        value: &Operand,
        resume: BlockId,
        drop: BlockId,
    ) -> Result<(), Error> {
        wasm_debug!(
            "        lowering Yield in block {} resume {} drop {}",
            block.id,
            resume,
            drop
        );
        let hook = self.runtime_hook_index(RuntimeHook::Yield)?;
        // Preserve side effects of the yielded value.
        let _ = self.emit_operand(buf, value)?;
        emit_instruction(buf, Op::Drop);
        if let Some(ctx) = self.async_context_pointer() {
            emit_instruction(buf, Op::LocalGet(ctx));
        } else {
            emit_instruction(buf, Op::I32Const(0));
        }
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Const(AWAIT_READY));
        emit_instruction(buf, Op::I32Eq);
        emit_instruction(buf, Op::If);
        self.emit_goto(buf, resume);
        emit_instruction(buf, Op::End);
        self.emit_goto(buf, drop);
        Ok(())
    }

    fn future_result_layout(&self, future_ty: &Ty) -> Result<Option<FutureResultLayout>, Error> {
        let Some(result_ty) = future_result_ty(future_ty) else {
            return Ok(None);
        };
        self.future_result_layout_from_result_ty(&result_ty)
            .map(Some)
    }

    fn future_result_layout_from_result_ty(
        &self,
        result_ty: &Ty,
    ) -> Result<FutureResultLayout, Error> {
        let header_ty = Ty::named("Std.Async.FutureHeader");
        let (header_size, header_align) = self
            .layouts
            .size_and_align_for_ty(&header_ty)
            .ok_or_else(|| {
                Error::Codegen(
                    "missing layout metadata for Std.Async.FutureHeader in WASM backend".into(),
                )
            })?;
        let bool_ty = Ty::named("bool");
        let (bool_size, bool_align) =
            self.layouts
                .size_and_align_for_ty(&bool_ty)
                .ok_or_else(|| {
                    Error::Codegen("missing layout metadata for bool in WASM async lowering".into())
                })?;
        let (_, result_align) = self
            .layouts
            .size_and_align_for_ty(result_ty)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for async result `{}`",
                    result_ty.canonical_name()
                ))
            })?;
        let (_result_size, _) = self
            .layouts
            .size_and_align_for_ty(result_ty)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for async result `{}`",
                    result_ty.canonical_name()
                ))
            })?;
        let mut offset = align_to(0, header_align);
        offset = align_to(
            offset.checked_add(header_size).ok_or_else(|| {
                Error::Codegen("future header size overflow in WASM backend".into())
            })?,
            bool_align,
        );
        offset = align_to(
            offset
                .checked_add(bool_size)
                .ok_or_else(|| Error::Codegen("future completion flag overflow".into()))?,
            result_align,
        );
        let value_ty = map_type(result_ty);
        let offset = ensure_u32(
            offset,
            "future result offset exceeds wasm32 addressable range",
        )?;
        Ok(FutureResultLayout { offset, value_ty })
    }

    fn emit_store_future_result(
        &mut self,
        buf: &mut Vec<u8>,
        base_local: u32,
        layout: &FutureResultLayout,
        destination: &Place,
    ) -> Result<(), Error> {
        emit_instruction(buf, Op::LocalGet(base_local));
        emit_instruction(buf, Op::I32Const(layout.offset as i32));
        emit_instruction(buf, Op::I32Add);
        match layout.value_ty {
            ValueType::I32 => emit_instruction(buf, Op::I32Load(0)),
            ValueType::I64 => emit_instruction(buf, Op::I64Load(0)),
            ValueType::F32 => emit_instruction(buf, Op::F32Load(0)),
            ValueType::F64 => emit_instruction(buf, Op::F64Load(0)),
        }
        self.store_value_into_place(buf, destination, layout.value_ty)
    }

    fn emit_task_result_store(
        &mut self,
        buf: &mut Vec<u8>,
        task_local: u32,
        result_ty: &Ty,
        destination: &Place,
    ) -> Result<(), Error> {
        let layout = self.async_task_layout(result_ty)?;
        let (size, _) = self
            .layouts
            .size_and_align_for_ty(result_ty)
            .or_else(|| {
                let base = result_ty.canonical_name();
                let short = base.rsplit("::").next().unwrap_or(&base);
                if short.eq_ignore_ascii_case("bool") || short.eq_ignore_ascii_case("boolean") {
                    Some((1, 1))
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for async result `{}`",
                    result_ty.canonical_name()
                ))
            })?;
        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
            eprintln!(
                "[wasm-async] async_task_result_store func={} result_ty={} size={} inner_result_offset={}",
                self.function.name,
                result_ty.canonical_name(),
                size,
                layout.inner_future_result_offset,
            );
        }
        // src pointer
        emit_instruction(buf, Op::LocalGet(task_local));
        emit_instruction(buf, Op::I32Const(layout.inner_future_result_offset as i32));
        emit_instruction(buf, Op::I32Add);
        // dst pointer
        let access = self.resolve_memory_access(destination)?;
        self.emit_pointer_expression(buf, &access)?;
        let result_size = ensure_u32(size, "task result size exceeds wasm32 addressable range")?;
        emit_instruction(buf, Op::I32Const(result_size as i32));
        let hook = self.runtime_hook_index(RuntimeHook::AsyncTaskResult)?;
        emit_instruction(buf, Op::Call(hook));
        Ok(())
    }

    fn async_task_layout(&self, result_ty: &Ty) -> Result<AsyncTaskLayout, Error> {
        let task_ty = Ty::named_generic(
            "Std::Async::Task",
            vec![GenericArg::Type(result_ty.clone())],
        );
        let task_layout = self
            .layouts
            .layout_for_name(&task_ty.canonical_name())
            .and_then(|layout| match layout {
                crate::mir::TypeLayout::Struct(data) | crate::mir::TypeLayout::Class(data) => {
                    Some(data)
                }
                _ => None,
            })
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for `{}` in WASM backend",
                    task_ty.canonical_name()
                ))
            })?;
        let header_offset = self.layout_field_offset(task_layout, "Header")?;
        let task_flags_offset = self.layout_field_offset(task_layout, "Flags")?;
        let inner_future_offset = self.layout_field_offset(task_layout, "InnerFuture")?;

        let header_layout = self
            .layouts
            .layout_for_name("Std::Async::FutureHeader")
            .and_then(|layout| match layout {
                crate::mir::TypeLayout::Struct(data) | crate::mir::TypeLayout::Class(data) => {
                    Some(data)
                }
                _ => None,
            })
            .ok_or_else(|| {
                Error::Codegen("missing Std.Async.FutureHeader layout for WASM".into())
            })?;
        let header_flags_offset = self.layout_field_offset(header_layout, "Flags")?;
        let header_vtable_offset = self.layout_field_offset(header_layout, "VTablePointer")?;

        let future_ty = Ty::named_generic(
            "Std::Async::Future",
            vec![GenericArg::Type(result_ty.clone())],
        );
        let future_layout = self
            .layouts
            .layout_for_name(&future_ty.canonical_name())
            .and_then(|layout| match layout {
                crate::mir::TypeLayout::Struct(data) | crate::mir::TypeLayout::Class(data) => {
                    Some(data)
                }
                _ => None,
            })
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for `{}` in WASM backend",
                    future_ty.canonical_name()
                ))
            })?;
        let future_header_offset = self.layout_field_offset(future_layout, "Header")?;
        let future_completed_offset = self.layout_field_offset(future_layout, "Completed")?;
        let future_result_offset = self.layout_field_offset(future_layout, "Result")?;

        if std::env::var_os("CHIC_DEBUG_WASM_ASYNC").is_some() {
            eprintln!(
                "[wasm-async] async_task_layout func={} result_ty={} header_vtable={} header_flags={} task_flags={} inner_offset={} future_header_offset={} future_completed_offset={} future_result_offset={}",
                self.function.name,
                result_ty.canonical_name(),
                header_vtable_offset,
                header_flags_offset,
                task_flags_offset,
                inner_future_offset,
                future_header_offset,
                future_completed_offset,
                future_result_offset,
            );
        }
        Ok(AsyncTaskLayout {
            task_header_vtable_offset: ensure_u32(
                header_offset
                    .checked_add(header_vtable_offset)
                    .ok_or_else(|| Error::Codegen("task header vtable offset overflow".into()))?,
                "task header vtable offset exceeds wasm32 range",
            )?,
            task_header_flags_offset: ensure_u32(
                header_offset
                    .checked_add(header_flags_offset)
                    .ok_or_else(|| Error::Codegen("task header flags offset overflow".into()))?,
                "task header flags offset exceeds wasm32 range",
            )?,
            task_flags_offset: ensure_u32(
                task_flags_offset,
                "task flags offset exceeds wasm32 range",
            )?,
            inner_future_header_flags_offset: ensure_u32(
                inner_future_offset
                    .checked_add(future_header_offset)
                    .and_then(|base| base.checked_add(header_flags_offset))
                    .ok_or_else(|| Error::Codegen("inner future flags offset overflow".into()))?,
                "inner future flags offset exceeds wasm32 range",
            )?,
            inner_future_completed_offset: ensure_u32(
                inner_future_offset
                    .checked_add(future_completed_offset)
                    .ok_or_else(|| {
                        Error::Codegen("inner future completed offset overflow".into())
                    })?,
                "inner future completed offset exceeds wasm32 range",
            )?,
            inner_future_result_offset: ensure_u32(
                inner_future_offset
                    .checked_add(future_result_offset)
                    .ok_or_else(|| Error::Codegen("inner future result offset overflow".into()))?,
                "inner future result offset exceeds wasm32 range",
            )?,
        })
    }

    fn return_local_frame_offset(&self) -> Option<u32> {
        let ret_local = self.return_local?;
        let (idx, _) = self
            .locals
            .iter()
            .enumerate()
            .find(|(_, slot)| slot.map(|idx| idx == ret_local).unwrap_or(false))?;
        self.aggregate_allocations
            .get(idx)
            .and_then(|entry| entry.map(|info| info.offset))
    }

    fn layout_field_offset(&self, layout: &StructLayout, field: &str) -> Result<usize, Error> {
        layout
            .fields
            .iter()
            .find(|f| f.name == field)
            .and_then(|f| f.offset)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing field `{}` offset in WASM async layout",
                    field
                ))
            })
    }
}
