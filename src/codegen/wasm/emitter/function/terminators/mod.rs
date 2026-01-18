use super::ops::{Op, emit_instruction};
use super::{FunctionEmitter, LocalRepresentation};
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::codegen::wasm::{
    RuntimeHook, STACK_POINTER_GLOBAL_INDEX, ValueType, compute_aggregate_allocation, ensure_u32,
    local_requires_memory, map_type,
};
use crate::drop_glue::drop_type_identity;
use crate::error::Error;
use crate::mir::async_types::{future_result_ty, is_future_ty, is_task_ty, task_result_ty};
use crate::mir::{
    BasicBlock, BlockId, BorrowKind, CallDispatch, ConstOperand, ConstValue, FnTy, GenericArg,
    LocalId, MatchArm, Operand, ParamMode, Pattern, PendingOperandInfo, Place, StructLayout,
    Terminator, TraitObjectDispatch, Ty, TypeLayout, VariantPatternFields, VirtualDispatch,
};
use crate::syntax::numeric::NumericLiteralType;
use std::convert::TryFrom;

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

#[derive(Clone, Copy)]
struct CallLowering<'a> {
    func: &'a Operand,
    args: &'a [Operand],
    modes: &'a [ParamMode],
    destination: Option<&'a Place>,
    target: BlockId,
    unwind: Option<BlockId>,
    dispatch: Option<&'a CallDispatch>,
}

impl<'a> FunctionEmitter<'a> {
    fn ty_requires_sret(&self, ty: &Ty) -> bool {
        !matches!(ty, Ty::Unit) && local_requires_memory(ty, self.layouts)
    }

    fn resolve_callee_return_ty(
        &self,
        func: &Operand,
        callee_name: Option<&str>,
        callee: u32,
    ) -> Option<Ty> {
        if let Some(name) = callee_name {
            if let Some(ty) = self.function_return_tys.get(name) {
                return Some(ty.clone());
            }
            if let Some(base) = name.split('<').next() {
                if let Some(ty) = self.function_return_tys.get(base) {
                    return Some(ty.clone());
                }
            }
        }

        if let Operand::Pending(pending) = func.clone() {
            if let Some(info) = &pending.info {
                let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                for candidate in candidates {
                    if self.lookup_function_index(&candidate.qualified) == Some(callee) {
                        return Some(candidate.signature.ret.as_ref().clone());
                    }
                }
            }
        }

        None
    }

    fn call_destination_requires_sret(&self, destination: Option<&Place>) -> Result<bool, Error> {
        let Some(place) = destination else {
            return Ok(false);
        };
        let ty = self.mir_place_ty(place)?;
        Ok(self.ty_requires_sret(&ty))
    }

    fn emit_sret_out_pointer(
        &mut self,
        buf: &mut Vec<u8>,
        destination: Option<&Place>,
        return_ty: Option<&Ty>,
    ) -> Result<(), Error> {
        if let Some(place) = destination {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
            return Ok(());
        }

        let Some(return_ty) = return_ty else {
            return Err(Error::Codegen(
                "WASM backend cannot lower an aggregate-return call without a destination".into(),
            ));
        };
        let allocation =
            compute_aggregate_allocation(return_ty, self.layouts).ok_or_else(|| {
                Error::Codegen(format!(
                    "missing layout metadata for aggregate return `{}` in WASM backend",
                    return_ty.canonical_name()
                ))
            })?;
        self.allocate_stack_block(buf, allocation.size, allocation.align)?;
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        Ok(())
    }

    fn emit_call_argument_for_mode(
        &mut self,
        buf: &mut Vec<u8>,
        arg: &Operand,
        mode: ParamMode,
    ) -> Result<(), Error> {
        if matches!(mode, ParamMode::Value) {
            self.emit_operand(buf, arg)?;
            return Ok(());
        }
        match arg {
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(())
            }
            Operand::Borrow(borrow) => {
                let access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(())
            }
            _ => Err(Error::Codegen(format!(
                "WASM backend requires place-backed arguments for {:?} parameters (caller={}, arg={:?})",
                mode, self.function.name, arg
            ))),
        }
    }

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
                CallLowering {
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

    fn emit_goto(&mut self, buf: &mut Vec<u8>, target: BlockId) {
        self.set_block(buf, target);
        emit_instruction(buf, Op::Br(1));
    }

    fn emit_return(&mut self, buf: &mut Vec<u8>) -> Result<(), Error> {
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

    fn emit_switch_int(
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

    fn emit_match(
        &mut self,
        buf: &mut Vec<u8>,
        value: &Place,
        arms: &[MatchArm],
        otherwise: BlockId,
    ) -> Result<(), Error> {
        wasm_debug!(
            "        lowering Match on {:?} with {} arms, otherwise {}",
            value,
            arms.len(),
            otherwise
        );
        if !value.projection.is_empty() {
            return Err(Error::Codegen(
                "WASM backend does not yet support projected match values".into(),
            ));
        }
        let value_ty = self.emit_place_value(buf, value)?;
        if !matches!(value_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "match discriminant must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let enum_ty = self.local_tys.get(value.local.0).cloned();

        for arm in arms {
            if self.emit_match_arm(buf, arm, enum_ty.as_ref())? {
                return Ok(());
            }
        }

        self.emit_match_default(buf, otherwise);
        Ok(())
    }

    fn emit_ready_task_return(&mut self, buf: &mut Vec<u8>) -> Result<bool, Error> {
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

    fn emit_match_arm(
        &mut self,
        buf: &mut Vec<u8>,
        arm: &MatchArm,
        enum_ty: Option<&Ty>,
    ) -> Result<bool, Error> {
        wasm_debug!(
            "          arm target {} pattern {:?}",
            arm.target,
            arm.pattern
        );
        if arm.guard.is_some() {
            wasm_debug!("            arm guard detected; guard block will handle predicate");
        }
        if !arm.bindings.is_empty() {
            wasm_debug!("            arm includes {} binding(s)", arm.bindings.len());
        }

        match &arm.pattern {
            Pattern::Wildcard | Pattern::Binding(_) => {
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(1));
                Ok(true)
            }
            Pattern::Struct { .. } | Pattern::Tuple(_) => {
                if Self::pattern_is_irrefutable(&arm.pattern) {
                    self.set_block(buf, arm.target);
                    emit_instruction(buf, Op::Br(1));
                    Ok(true)
                } else {
                    Err(Error::Codegen(
                        "complex destructuring patterns are not yet supported by the WASM backend"
                            .into(),
                    ))
                }
            }
            Pattern::Literal(literal) => {
                let literal_op = Self::const_to_op(literal)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, literal_op);
                emit_instruction(buf, Op::I32Eq);
                emit_instruction(buf, Op::If);
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(2));
                emit_instruction(buf, Op::End);
                Ok(false)
            }
            Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let layout = enum_ty
                    .and_then(|ty| self.lookup_enum_layout(ty))
                    .or_else(|| {
                        let candidate = path.join("::");
                        self.layouts
                            .layout_for_name(&candidate)
                            .and_then(|layout| match layout {
                                TypeLayout::Enum(data) => Some(data),
                                _ => None,
                            })
                    });
                let Some(layout) = layout else {
                    if std::env::var_os("CHIC_DEBUG_WASM_MATCH").is_some() {
                        eprintln!(
                            "[wasm-match-missing-layout] func={} ty={} path={} variant={}",
                            self.function.name,
                            enum_ty
                                .map(|ty| ty.canonical_name())
                                .unwrap_or_else(|| "<unknown>".into()),
                            path.join("::"),
                            variant
                        );
                    }
                    self.set_block(buf, arm.target);
                    emit_instruction(buf, Op::Br(1));
                    return Ok(true);
                };
                if !matches!(fields, VariantPatternFields::Unit) {
                    return Err(Error::Codegen(
                        "enum patterns with payloads are not yet supported by the WASM backend"
                            .into(),
                    ));
                }
                let variant_layout = layout
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "enum `{}` does not define variant `{variant}`",
                            layout.name
                        ))
                    })?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                let literal = Self::convert_switch_value(variant_layout.discriminant)?;
                emit_instruction(buf, Op::I32Const(literal));
                emit_instruction(buf, Op::I32Eq);
                emit_instruction(buf, Op::If);
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(2));
                emit_instruction(buf, Op::End);
                Ok(false)
            }
        }
    }

    fn emit_match_default(&self, buf: &mut Vec<u8>, otherwise: BlockId) {
        wasm_debug!("        match lowering: default branch {}", otherwise);
        self.set_block(buf, otherwise);
        emit_instruction(buf, Op::Br(1));
    }

    fn pattern_is_irrefutable(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard | Pattern::Binding(_) => true,
            Pattern::Tuple(items) => items.iter().all(Self::pattern_is_irrefutable),
            Pattern::Struct { fields, .. } => fields
                .iter()
                .all(|field| Self::pattern_is_irrefutable(&field.pattern)),
            Pattern::Literal(_) | Pattern::Enum { .. } => false,
        }
    }

    fn emit_call(&mut self, buf: &mut Vec<u8>, call: CallLowering<'_>) -> Result<(), Error> {
        if let Some(dispatch) = call.dispatch {
            match dispatch {
                CallDispatch::Trait(trait_dispatch) => {
                    return self.emit_trait_object_call(buf, &call, trait_dispatch);
                }
                CallDispatch::Virtual(virtual_dispatch) => {
                    return self.emit_virtual_call(buf, &call, virtual_dispatch);
                }
            }
        }
        if self.try_emit_startup_runtime_call(buf, &call)? {
            return Ok(());
        }
        if self.try_emit_decimal_fast_runtime_call(buf, &call)? {
            return Ok(());
        }
        if std::env::var_os("CHIC_DEBUG_WASM_CALLS").is_some() {
            if let Operand::Const(constant) = call.func {
                if let ConstValue::Symbol(name) = constant.value() {
                    eprintln!(
                        "[wasm-call] func={name} args={:?} in {}",
                        call.args, self.function.name
                    );
                }
            }
        }
        let fn_ty = self.call_operand_fn_ty(call.func);
        if fn_ty.is_none()
            && matches!(
                call.func,
                Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_)
            )
        {
            if std::env::var_os("CHIC_DEBUG_WASM_CALLS").is_some() {
                eprintln!(
                    "[wasm-call-unsup] func={:?} args={:?} in {}",
                    call.func, call.args, self.function.name
                );
            }
            let (place, ty) = match &call.func {
                Operand::Copy(place) | Operand::Move(place) => {
                    (place, self.local_tys.get(place.local.0))
                }
                Operand::Borrow(borrow) => {
                    (&borrow.place, self.local_tys.get(borrow.place.local.0))
                }
                _ => unreachable!(),
            };
            let ty_name = ty
                .map(|ty| ty.canonical_name())
                .unwrap_or_else(|| "<unknown>".into());
            return Err(Error::Codegen(format!(
                "first-class function values are not yet supported by the WASM backend (local {} type {} projection {:?})",
                place.local.0, ty_name, place.projection
            )));
        }
        if let Some(fn_ty) = fn_ty {
            self.emit_indirect_call(buf, call, fn_ty)
        } else {
            self.emit_direct_call(buf, call)
        }
    }

    fn emit_pending_exception_check(
        &mut self,
        buf: &mut Vec<u8>,
        unwind: Option<BlockId>,
    ) -> Result<(), Error> {
        let hook = self.runtime_hook_index(RuntimeHook::HasPendingException)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::If);
        if let Some(unwind) = unwind {
            self.set_block(buf, unwind);
            emit_instruction(buf, Op::Br(2));
        } else {
            self.emit_frame_teardown(buf);
            emit_instruction(buf, Op::Br(3));
        }
        emit_instruction(buf, Op::End);
        Ok(())
    }

    fn emit_trait_object_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
        dispatch: &TraitObjectDispatch,
    ) -> Result<(), Error> {
        let receiver = call.args.get(dispatch.receiver_index).ok_or_else(|| {
            Error::Codegen("trait object call is missing its receiver argument".into())
        })?;
        let place = match receiver {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(
                    "trait object receiver must be addressable in WASM backend".into(),
                ));
            }
        };
        let trait_label = dispatch
            .trait_name
            .rsplit("::")
            .next()
            .unwrap_or(dispatch.trait_name.as_str());
        let lookup_slot = |impl_type: &str| {
            self.trait_vtables
                .iter()
                .find(|table| {
                    Self::names_equivalent(&table.trait_name, &dispatch.trait_name)
                        && Self::names_equivalent(&table.impl_type, impl_type)
                })
                .and_then(|table| table.slots.get(dispatch.slot_index as usize))
                .map(|slot| slot.symbol.clone())
        };
        let lookup_method_symbol = |impl_type: &str| {
            let candidates = [
                format!("{impl_type}::{trait_label}::{}", dispatch.method),
                format!("{impl_type}::{}", dispatch.method),
            ];
            for candidate in candidates {
                let canonical = canonical_symbol_name(&candidate);
                if self.function_signatures.contains_key(&canonical)
                    || self.functions.contains_key(&canonical)
                {
                    return Some(candidate);
                }
            }
            None
        };
        let resolve_direct_symbol =
            |impl_type: &str| lookup_slot(impl_type).or_else(|| lookup_method_symbol(impl_type));
        let mut direct_symbol = dispatch
            .impl_type
            .as_deref()
            .and_then(|impl_type| resolve_direct_symbol(impl_type));
        let mut receiver_ty = None;
        if direct_symbol.is_none() && dispatch.impl_type.is_some() {
            if let Ok(place_ty) = self.mir_place_ty(place) {
                let resolved = self.resolve_self_ty(&place_ty);
                receiver_ty = Some(resolved.clone());
                let mut core_ty = &resolved;
                loop {
                    match core_ty {
                        Ty::Pointer(ptr) => core_ty = &ptr.element,
                        Ty::Ref(reference) => core_ty = &reference.element,
                        Ty::Nullable(inner) => core_ty = inner.as_ref(),
                        _ => break,
                    }
                }
                if !matches!(core_ty, Ty::TraitObject(_)) {
                    direct_symbol = resolve_direct_symbol(&core_ty.canonical_name());
                }
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_TRAIT_CALL").is_some() {
            eprintln!(
                "[wasm-trait-call] func={} trait={} method={} impl_hint={:?} receiver_ty={:?} direct_symbol={:?}",
                self.function.name,
                dispatch.trait_name,
                dispatch.method,
                dispatch.impl_type,
                receiver_ty.as_ref().map(|ty: &Ty| ty.canonical_name()),
                direct_symbol
            );
        }
        if let Some(symbol) = direct_symbol {
            let direct = Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)));
            let direct_call = CallLowering {
                func: &direct,
                args: call.args,
                modes: call.modes,
                destination: call.destination,
                target: call.target,
                unwind: call.unwind,
                dispatch: None,
            };
            return self.emit_direct_call(buf, direct_call);
        }
        if self.function.name.contains("ThreadFunctionRunner::Run") {
            eprintln!(
                "[wasm-trait-call] func={} trait={} method={} slot_index={} impl_hint={:?}",
                self.function.name,
                dispatch.trait_name,
                dispatch.method,
                dispatch.slot_index,
                dispatch.impl_type
            );
        }
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalTee(self.block_local));
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let mut vtable_offset: Option<u32> = None;
        if let Some(impl_type) = dispatch.impl_type.as_deref()
            && impl_type != dispatch.trait_name
        {
            let tables_for_trait = self
                .trait_vtables
                .iter()
                .filter(|table| Self::names_equivalent(&table.trait_name, &dispatch.trait_name))
                .collect::<Vec<_>>();
            let selected = tables_for_trait
                .iter()
                .copied()
                .find(|table| Self::names_equivalent(&table.impl_type, impl_type))
                .or_else(|| (tables_for_trait.len() == 1).then(|| tables_for_trait[0]));
            if let Some(table) = selected {
                vtable_offset = self.trait_vtable_offsets.get(&table.symbol).copied();
            }
        }
        if let Some(offset) = vtable_offset {
            emit_instruction(
                buf,
                Op::I32Const(i32::try_from(offset).map_err(|_| {
                    Error::Codegen("trait vtable offset exceeds i32 range in WASM backend".into())
                })?),
            );
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            // Fallback to dyn trait object layout: load the vtable pointer from the second
            // word of the `{ data_ptr, vtable_ptr }` pair.
            emit_instruction(buf, Op::LocalGet(self.block_local));
            emit_instruction(buf, Op::I32Const(4));
            emit_instruction(buf, Op::I32Add);
            emit_instruction(buf, Op::I32Load(0));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }
        let slot_offset = dispatch
            .slot_index
            .checked_mul(4)
            .ok_or_else(|| Error::Codegen("trait vtable slot offset overflow".into()))?;
        let slot_offset = i32::try_from(slot_offset).map_err(|_| {
            Error::Codegen("trait vtable slot offset exceeds addressable range".into())
        })?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32Const(slot_offset));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.block_local));

        if dispatch.trait_name.contains("ThreadStart") && dispatch.method == "Run" {
            if let Some(run_index) =
                self.lookup_function_index("Std::Platform::Thread::ThreadFunctionStartAdapter::Run")
            {
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::If);
                emit_instruction(
                    buf,
                    Op::I32Const(i32::try_from(run_index).unwrap_or_default()),
                );
                emit_instruction(buf, Op::LocalSet(self.block_local));
                emit_instruction(buf, Op::End);
            }
        }

        let symbol = self.trait_vtable_slot_symbol(dispatch)?;
        let signature = self.function_signatures.get(symbol).ok_or_else(|| {
            Error::Codegen(format!(
                "missing WebAssembly signature metadata for trait method `{symbol}`"
            ))
        })?;
        let type_index = *self.signature_indices.get(signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function signature for `{symbol}` is not registered in the WASM type table"
            ))
        })?;

        let signature_requires_sret = signature.params.len() == call.args.len() + 1
            && matches!(signature.params.first(), Some(ValueType::I32))
            && signature.results.len() == 1
            && signature.results[0] == ValueType::I32;
        let ret_is_sret =
            signature_requires_sret || self.call_destination_requires_sret(call.destination)?;

        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?
                .or_else(|| self.function_return_tys.get(symbol).cloned());
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            if index == dispatch.receiver_index {
                emit_instruction(buf, Op::LocalGet(self.temp_local));
            } else {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
        }

        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn trait_vtable_slot_symbol(&self, dispatch: &TraitObjectDispatch) -> Result<&str, Error> {
        let table = self
            .trait_vtables
            .iter()
            .find(|table| table.trait_name == dispatch.trait_name)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "trait `{}` does not have vtable metadata in this module",
                    dispatch.trait_name
                ))
            })?;
        let slot = table
            .slots
            .get(dispatch.slot_index as usize)
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "trait `{}` vtable is missing slot {}",
                    dispatch.trait_name, dispatch.slot_index
                ))
            })?;
        Ok(slot.symbol.as_str())
    }

    fn emit_virtual_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
        dispatch: &VirtualDispatch,
    ) -> Result<(), Error> {
        let receiver = call.args.get(dispatch.receiver_index).ok_or_else(|| {
            Error::Codegen("virtual dispatch is missing its receiver argument".into())
        })?;
        match receiver {
            Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_) => {}
            _ => {
                return Err(Error::Codegen(
                    "virtual dispatch receiver must be addressable in WASM backend".into(),
                ));
            }
        };

        let receiver_ty = self.emit_operand(buf, receiver)?;
        if receiver_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "virtual dispatch receiver must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        if let Some(owner) = dispatch.base_owner.as_deref() {
            let table = self
                .class_vtables
                .iter()
                .find(|table| table.type_name == owner)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "class `{owner}` is not defined in this module; base dispatch cannot resolve its vtable"
                    ))
                })?;
            let offset = self
                .class_vtable_offsets
                .get(&table.symbol)
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "class `{owner}` does not have vtable data in this module"
                    ))
                })?;
            emit_instruction(
                buf,
                Op::I32Const(i32::try_from(*offset).map_err(|_| {
                    Error::Codegen("class vtable offset exceeds i32 range in WASM backend".into())
                })?),
            );
            emit_instruction(buf, Op::LocalSet(self.block_local));
        } else {
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            emit_instruction(buf, Op::I32Load(0));
            emit_instruction(buf, Op::LocalSet(self.block_local));
        }

        let slot_offset_bytes = dispatch
            .slot_index
            .checked_mul(4)
            .ok_or_else(|| Error::Codegen("virtual dispatch slot offset overflow".into()))?;
        let slot_offset = i32::try_from(slot_offset_bytes)
            .map_err(|_| Error::Codegen("virtual dispatch slot offset exceeds i32 range".into()))?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::I32Const(slot_offset));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::I32Load(0));
        emit_instruction(buf, Op::LocalSet(self.block_local));

        let ret_is_sret = self.call_destination_requires_sret(call.destination)?;
        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?;
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            if index == dispatch.receiver_index {
                emit_instruction(buf, Op::LocalGet(self.temp_local));
            } else {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
        }

        let callee = match call.func {
            Operand::Const(constant) => match constant.value() {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => {
                    return Err(Error::Codegen(
                        "virtual dispatch requires a symbol operand for its callee".into(),
                    ));
                }
            },
            _ => {
                return Err(Error::Codegen(
                    "virtual dispatch requires a constant callee operand in WASM backend".into(),
                ));
            }
        };

        let signature = self.function_signatures.get(&callee).ok_or_else(|| {
            Error::Codegen(format!(
                "missing WebAssembly signature metadata for virtual call `{callee}`"
            ))
        })?;
        let type_index = *self.signature_indices.get(signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function signature `{callee}` is not registered in the WASM type table"
            ))
        })?;

        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_direct_call(&mut self, buf: &mut Vec<u8>, call: CallLowering<'_>) -> Result<(), Error> {
        let callee = self.resolve_callee(call.func, call.args)?;
        let mut callee_name = match call.func {
            Operand::Const(constant) => match constant.value() {
                ConstValue::Symbol(name) => Some(canonical_symbol_name(name)),
                _ => None,
            },
            Operand::Pending(pending) => {
                let mut resolved = None;
                if let Some(info) = &pending.info {
                    let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                    for candidate in candidates {
                        if let Some(idx) = self.lookup_function_index(&candidate.qualified) {
                            if idx == callee {
                                resolved = Some(canonical_symbol_name(&candidate.qualified));
                                break;
                            }
                        }
                    }
                }
                if resolved.is_some() {
                    resolved
                } else {
                    let repr = pending.repr.replace('.', "::");
                    if self.lookup_function_index(&repr) == Some(callee) {
                        Some(repr)
                    } else {
                        None
                    }
                }
            }
            _ => None,
        };
        let callee_name_by_index = self
            .functions
            .iter()
            .find_map(|(name, idx)| (*idx == callee).then_some(name.clone()));
        if callee_name.is_none() {
            callee_name = callee_name_by_index.clone();
        }
        let mut expected_params = callee_name.as_ref().and_then(|name| {
            self.function_signatures
                .get(name)
                .map(|sig| sig.params.clone())
        });
        let mut expected_results = callee_name.as_ref().and_then(|name| {
            self.function_signatures
                .get(name)
                .map(|sig| sig.results.clone())
        });
        let mut expected_mir_params = callee_name
            .as_deref()
            .and_then(|name| self.function_param_tys.get(name));
        if expected_params.is_none() && expected_results.is_none() {
            if let Some(name) = callee_name_by_index.clone() {
                if let Some(sig) = self.function_signatures.get(&name) {
                    expected_params = Some(sig.params.clone());
                    expected_results = Some(sig.results.clone());
                    callee_name = Some(name);
                    expected_mir_params = callee_name
                        .as_deref()
                        .and_then(|n| self.function_param_tys.get(n));
                }
            } else if let Some(name) = callee_name
                .as_deref()
                .and_then(|name| name.split('<').next())
            {
                if let Some(sig) = self.function_signatures.get(name) {
                    expected_params = Some(sig.params.clone());
                    expected_results = Some(sig.results.clone());
                    callee_name = Some(name.to_string());
                    expected_mir_params = callee_name
                        .as_deref()
                        .and_then(|n| self.function_param_tys.get(n));
                }
            }
        }
        if let Some(name) = callee_name.as_deref() {
            let tail = name.rsplit("::").next().unwrap_or(name);
            if name == "chic_rt::string_as_slice"
                || tail == "chic_rt_string_as_slice"
                || tail == "string_as_slice"
            {
                let signature = RuntimeHook::StringAsSlice.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if name == "chic_rt::string_as_chars"
                || tail == "chic_rt_string_as_chars"
                || tail == "string_as_chars"
            {
                let signature = RuntimeHook::StringAsChars.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if name == "chic_rt::str_as_chars"
                || tail == "chic_rt_str_as_chars"
                || tail == "str_as_chars"
            {
                let signature = RuntimeHook::StrAsChars.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            } else if expected_results.is_none()
                && (name.contains("AsUtf8Span")
                    || name.contains("AsUtf8")
                    || name.contains("AsSpan"))
            {
                let signature = RuntimeHook::StringAsSlice.signature();
                expected_params = Some(signature.params);
                expected_results = Some(signature.results);
            }
        }
        let callee_return_ty =
            self.resolve_callee_return_ty(call.func, callee_name.as_deref(), callee);
        let mut ret_is_sret = if let Some(ty) = callee_return_ty.as_ref() {
            self.ty_requires_sret(ty)
        } else {
            self.call_destination_requires_sret(call.destination)?
        };
        if std::env::var_os("CHIC_DEBUG_WASM_CALL_SIG").is_some() {
            eprintln!(
                "[wasm-call-sig] caller={} callee={:?} callee_idx={} args={} expected_params={} expected_results={} ret_is_sret={}",
                self.function.name,
                callee_name.as_deref(),
                callee,
                call.args.len(),
                expected_params.as_ref().map(|p| p.len()).unwrap_or(0),
                expected_results.as_ref().map(|r| r.len()).unwrap_or(0),
                ret_is_sret
            );
        }
        let multi_result = expected_results
            .as_ref()
            .map(|results| results.len() > 1)
            .unwrap_or(false);
        if multi_result {
            ret_is_sret = false;
        }
        if let Some(name) = callee_name.as_deref() {
            ensure_std_runtime_intrinsic_owner(name)?;
        }
        if let Some(name) = callee_name.as_deref() {
            if matches!(
                name,
                "Std::Numeric::PointerIntrinsics::AsByteMut"
                    | "Std::Numeric::PointerIntrinsics::AsByteConst"
                    | "Std::Numeric::PointerIntrinsics::AsByteConstFromMut"
            ) {
                if call.args.len() != 1 {
                    return Err(Error::Codegen(format!(
                        "pointer intrinsic `{name}` expects a single argument"
                    )));
                }
                self.emit_operand(buf, &call.args[0])?;
                if let Some(dest) = call.destination {
                    self.store_call_result(buf, dest)?;
                } else {
                    emit_instruction(buf, Op::Drop);
                }
                self.release_call_borrows(buf, call.args, call.modes)?;
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        let is_thread_fn_init = callee == 1466
            || callee_name
                .as_deref()
                .is_some_and(|name| name.contains("ThreadFunctionStartAdapter::init"));
        if callee == 1466 && call.args.len() >= 2 {
            let self_arg = &call.args[0];
            let fn_arg = &call.args[1];
            if let Some(fn_ty) = self.call_operand_fn_ty(fn_arg) {
                self.emit_operand(buf, self_arg)?;
                self.emit_fn_argument(buf, fn_arg, &fn_ty)?;
                emit_instruction(buf, Op::Call(callee));
                self.release_call_borrows(buf, call.args, call.modes)?;
                if let Some(place) = call.destination {
                    self.store_call_result(buf, place)?;
                }
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        if is_thread_fn_init && call.args.len() == 2 {
            // Force a well-formed init call: push the adapter `self` first, then a freshly
            // built fn struct derived from the invoke value of the second argument.
            self.emit_operand(buf, &call.args[0])?;
            let arg = &call.args[1];
            let arg_ty = self.operand_ty(arg).or_else(|| match arg {
                Operand::Copy(place) | Operand::Move(place) => {
                    self.local_tys.get(place.local.0).cloned()
                }
                _ => None,
            });
            if let Some(fn_ty) = arg_ty {
                if let Some(layout) = self
                    .lookup_struct_layout(&fn_ty)
                    .cloned()
                    .filter(Self::is_fn_pointer_layout)
                {
                    let allocation = compute_aggregate_allocation(&fn_ty, self.layouts)
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "function pointer layout missing allocation metadata for `{}` in WASM backend",
                                fn_ty.canonical_name()
                            ))
                        })?;
                    self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalSet(self.temp_local));
                    self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                    emit_instruction(buf, Op::LocalSet(self.block_local));
                    self.initialise_fn_from_invoke(buf, &layout, &fn_ty, self.block_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(buf, Op::Call(callee));
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
            }
        }
        if ret_is_sret {
            let return_ty = call
                .destination
                .map(|dest| self.mir_place_ty(dest))
                .transpose()?
                .or_else(|| callee_return_ty.clone());
            self.emit_sret_out_pointer(buf, call.destination, return_ty.as_ref())?;
        }
        const NULLABLE_UNWRAP_PANIC_CODE: i32 = 0x2010;

        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            if !matches!(mode, ParamMode::Value) {
                self.emit_call_argument_for_mode(buf, arg, mode)?;
                continue;
            }
            let arg_ty = self.operand_ty(arg).or_else(|| match arg {
                Operand::Copy(place) | Operand::Move(place) => {
                    self.local_tys.get(place.local.0).cloned()
                }
                _ => None,
            });
            let expected_param = expected_params
                .as_ref()
                .and_then(|params| params.get(index + if ret_is_sret { 1 } else { 0 }))
                .copied();

            if let Some(expected_param) = expected_param {
                let is_str_operand = matches!(arg_ty.as_ref(), Some(Ty::Str))
                    || matches!(
                        arg,
                        Operand::Const(constant)
                            if matches!(constant.value(), ConstValue::Str { .. })
                    );

                if is_str_operand && expected_param == ValueType::I32 {
                    // Some call sites (notably runtime shims) expect `str` to be passed by pointer to
                    // a `{ ptr: u32, len: u32 }` pair in linear memory. Materialize the packed `i64`
                    // str value into a stack slot and pass the slot pointer.
                    self.allocate_stack_block(buf, 8, 8)?;
                    let value_ty = self.emit_operand(buf, arg)?;
                    Self::ensure_operand_type(value_ty, ValueType::I64, "str argument")?;
                    emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                    emit_instruction(buf, Op::I64Store(0));

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    continue;
                }

                if expected_param == ValueType::I32 {
                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if map_type(&arg_ty) == ValueType::I64
                            && !local_requires_memory(&arg_ty, self.layouts)
                            && !matches!(arg_ty, Ty::Str)
                        {
                            let value_ty = self.emit_operand(buf, arg)?;
                            Self::ensure_operand_type(value_ty, ValueType::I64, "i64->i32 arg")?;
                            emit_instruction(buf, Op::I32WrapI64);
                            continue;
                        }
                    }
                }

                if let Some(Ty::Nullable(inner)) = arg_ty.as_ref() {
                    let inner_value_ty = map_type(inner.as_ref());
                    let expected_mir_param =
                        expected_mir_params.and_then(|params| params.get(index));
                    let should_unwrap = if let Some(expected_mir_param) = expected_mir_param {
                        let expected_mir_param = self.resolve_self_ty(expected_mir_param);
                        let expected_inner = self.resolve_self_ty(inner.as_ref());
                        inner_value_ty == expected_param
                            && matches!(inner_value_ty, ValueType::I32 | ValueType::I64)
                            && !matches!(expected_mir_param, Ty::Nullable(_))
                            && expected_mir_param == expected_inner
                    } else {
                        // Without MIR parameter types, unwrapping `Nullable(T)` based solely on wasm
                        // value types is ambiguous for `i32` (both the nullable wrapper pointer and
                        // many inner values are `i32`). Only unwrap by value type for `i64`.
                        inner_value_ty == expected_param && inner_value_ty == ValueType::I64
                    };
                    if should_unwrap {
                        if std::env::var_os("CHIC_DEBUG_WASM_NULLABLE_UNWRAP").is_some() {
                            eprintln!(
                                "[wasm-nullable-unwrap] caller={} callee={:?} arg_index={} arg_ty={:?} inner_ty={:?} expected_mir_param={:?} expected_wasm={:?}",
                                self.function.name,
                                callee_name.as_deref(),
                                index,
                                arg_ty,
                                inner.as_ref(),
                                expected_mir_param
                                    .map(|ty| self.resolve_self_ty(ty))
                                    .unwrap_or_else(|| Ty::Unknown),
                                expected_param,
                            );
                        }
                        // Flow typing can treat a nullable value as non-null after a null-check.
                        // The MIR still passes the nullable pointer, so load the inner payload.
                        let ptr_ty = self.emit_operand(buf, arg)?;
                        Self::ensure_operand_type(ptr_ty, ValueType::I32, "nullable unwrap")?;
                        emit_instruction(buf, Op::LocalSet(self.temp_local));

                        let nullable_ty = Ty::Nullable(inner.clone());
                        let (_, has_value_offset) =
                            self.resolve_field_by_name(&nullable_ty, None, "HasValue")?;
                        let (_, value_offset) =
                            self.resolve_field_by_name(&nullable_ty, None, "Value")?;

                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        let has_value_offset = ensure_u32(
                            has_value_offset,
                            "nullable HasValue offset exceeds wasm32 addressable range",
                        )?;
                        if has_value_offset != 0 {
                            emit_instruction(buf, Op::I32Const(has_value_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load8U(0));
                        emit_instruction(buf, Op::I32Eqz);
                        emit_instruction(buf, Op::If);
                        self.emit_runtime_panic_with_code(buf, NULLABLE_UNWRAP_PANIC_CODE)?;
                        emit_instruction(buf, Op::End);

                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        let value_offset = ensure_u32(
                            value_offset,
                            "nullable payload offset exceeds wasm32 addressable range",
                        )?;
                        if value_offset != 0 {
                            emit_instruction(buf, Op::I32Const(value_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        match expected_param {
                            ValueType::I32 => emit_instruction(buf, Op::I32Load(0)),
                            ValueType::I64 => emit_instruction(buf, Op::I64Load(0)),
                            _ => {}
                        }
                        continue;
                    }
                }

                if expected_param == ValueType::I64 {
                    if matches!(arg_ty.as_ref(), Some(Ty::String)) {
                        // Some call sites end up passing a `string` where a `str` is expected
                        // (notably base-ctor forwarding). Convert via `string_as_slice` and pack.
                        let string_ptr_ty = self.emit_operand(buf, arg)?;
                        Self::ensure_operand_type(
                            string_ptr_ty,
                            ValueType::I32,
                            "string->str conversion",
                        )?;
                        let hook = self.runtime_hook_index(RuntimeHook::StringAsSlice)?;
                        emit_instruction(buf, Op::Call(hook));
                        // Results are `(ptr, len)` with `len` on top of the stack.
                        emit_instruction(buf, Op::LocalSet(self.block_local)); // len
                        emit_instruction(buf, Op::LocalSet(self.temp_local)); // ptr

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        emit_instruction(buf, Op::I64ExtendI32U);
                        emit_instruction(buf, Op::I64Const(32));
                        emit_instruction(buf, Op::I64Shl);
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(buf, Op::I64ExtendI32U);
                        emit_instruction(buf, Op::I64Or);
                        continue;
                    }

                    if arg_ty.is_none() {
                        let value_ty = self.emit_operand(buf, arg)?;
                        if value_ty == ValueType::I32 {
                            let signed = matches!(
                                arg,
                                Operand::Const(constant)
                                    if matches!(constant.value(), ConstValue::Int(_))
                            );
                            emit_instruction(
                                buf,
                                if signed {
                                    Op::I64ExtendI32S
                                } else {
                                    Op::I64ExtendI32U
                                },
                            );
                        }
                        continue;
                    }

                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if local_requires_memory(&arg_ty, self.layouts) {
                            if let Some(allocation) =
                                compute_aggregate_allocation(&arg_ty, self.layouts)
                            {
                                if allocation.size == 8 {
                                    match arg {
                                        Operand::Copy(place) | Operand::Move(place) => {
                                            let access = self.resolve_memory_access(place)?;
                                            self.emit_pointer_expression(buf, &access)?;
                                            emit_instruction(buf, Op::I64Load(0));
                                            continue;
                                        }
                                        Operand::Borrow(borrow) => {
                                            let access =
                                                self.resolve_memory_access(&borrow.place)?;
                                            self.emit_pointer_expression(buf, &access)?;
                                            emit_instruction(buf, Op::I64Load(0));
                                            continue;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }

                if expected_param == ValueType::I64 {
                    if let Some(arg_ty) = arg_ty.as_ref().map(|ty| self.resolve_self_ty(ty)) {
                        if map_type(&arg_ty) == ValueType::I32
                            && !local_requires_memory(&arg_ty, self.layouts)
                            && !matches!(arg_ty, Ty::Str | Ty::Nullable(_))
                        {
                            let value_ty = self.emit_operand(buf, arg)?;
                            Self::ensure_operand_type(value_ty, ValueType::I32, "i32->i64 arg")?;
                            let signed = match &arg_ty {
                                Ty::Pointer(_)
                                | Ty::Ref(_)
                                | Ty::Rc(_)
                                | Ty::Arc(_)
                                | Ty::Fn(_) => false,
                                _ => crate::mir::casts::int_info(
                                    &self.layouts.primitive_registry,
                                    &arg_ty.canonical_name(),
                                    self.pointer_width_bits() / 8,
                                )
                                .map(|info| info.signed)
                                .unwrap_or(true),
                            };
                            emit_instruction(
                                buf,
                                if signed {
                                    Op::I64ExtendI32S
                                } else {
                                    Op::I64ExtendI32U
                                },
                            );
                            continue;
                        }
                    }
                }
            }

            if is_thread_fn_init && index == 1 {
                let fn_ty = arg_ty.as_ref().map(|ty| match ty {
                    Ty::Fn(_) => ty.clone(),
                    Ty::Pointer(inner) => inner.element.clone(),
                    Ty::Ref(inner) => inner.element.clone(),
                    Ty::Nullable(inner) => match inner.as_ref() {
                        Ty::Pointer(ptr) => ptr.element.clone(),
                        Ty::Ref(r) => r.element.clone(),
                        other => other.clone(),
                    },
                    other => other.clone(),
                });
                if let Some(fn_ty) = fn_ty {
                    if let Some(layout) = self
                        .lookup_struct_layout(&fn_ty)
                        .cloned()
                        .filter(Self::is_fn_pointer_layout)
                    {
                        let allocation = compute_aggregate_allocation(&fn_ty, self.layouts)
                            .ok_or_else(|| {
                                Error::Codegen(format!(
                                    "function pointer layout missing allocation metadata for `{}` in WASM backend",
                                    fn_ty.canonical_name()
                                ))
                            })?;
                        self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        emit_instruction(buf, Op::LocalSet(self.temp_local));
                        self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, &layout, &fn_ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        continue;
                    }
                }
            }
            if let Operand::Copy(place) | Operand::Move(place) = arg {
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::Scalar)
                ) {
                    let _layout_name = arg_ty
                        .as_ref()
                        .and_then(|ty| self.lookup_struct_layout(ty))
                        .map(|layout| layout.name.clone());
                    let is_fn_like = arg_ty
                        .as_ref()
                        .and_then(|ty| {
                            self.lookup_struct_layout(ty)
                                .filter(|layout| Self::is_fn_pointer_layout(layout))
                        })
                        .is_some()
                        || matches!(
                            arg_ty.as_ref(),
                            Some(Ty::Fn(fn_ty))
                                if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        );
                    if is_fn_like {
                        if let Some(arg_ty) = arg_ty {
                            let layout =
                                self.lookup_struct_layout(&arg_ty).cloned().ok_or_else(|| {
                                    Error::Codegen(format!(
                                        "missing function pointer layout for `{}` in WASM backend",
                                        arg_ty.canonical_name()
                                    ))
                                })?;
                            self.emit_fn_invoke_argument(buf, arg, &arg_ty, &layout)?;
                            continue;
                        }
                    }
                    if !is_fn_like {
                        self.emit_operand(buf, arg)?;
                        continue;
                    }
                }
            }
            if let Some(ty) = self.operand_ty(arg) {
                if let Some(layout) = self.lookup_struct_layout(&ty).cloned().filter(
                    crate::codegen::wasm::emitter::function::FunctionEmitter::is_fn_pointer_layout,
                ) {
                    let arg_repr = match arg {
                        Operand::Copy(place) | Operand::Move(place) => {
                            self.representations.get(place.local.0)
                        }
                        Operand::Borrow(borrow) => self.representations.get(borrow.place.local.0),
                        _ => None,
                    };
                    if matches!(
                        arg_repr,
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        match arg {
                            Operand::Copy(place) | Operand::Move(place) => {
                                if let Ok(access) = self.resolve_memory_access(place) {
                                    self.emit_pointer_expression(buf, &access)?;
                                } else {
                                    self.emit_operand(buf, arg)?;
                                }
                            }
                            Operand::Borrow(borrow) => {
                                let access = self.resolve_memory_access(&borrow.place)?;
                                self.emit_pointer_expression(buf, &access)?;
                            }
                            _ => {
                                self.emit_operand(buf, arg)?;
                            }
                        }
                        continue;
                    }
                    self.emit_fn_invoke_argument(buf, arg, &ty, &layout)?;
                    continue;
                }
                if let Ty::Fn(fn_ty) = &ty {
                    if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                        // Extern fn pointers are thin; treat them as scalars.
                    } else {
                        let layout = self.lookup_struct_layout(&ty).cloned().ok_or_else(|| {
                            Error::Codegen(format!(
                                "missing function pointer layout for `{}` in WASM backend",
                                ty.canonical_name()
                            ))
                        })?;
                        self.emit_fn_invoke_argument(buf, arg, &ty, &layout)?;
                        continue;
                    }
                }
                let pointer_to_fn = match &ty {
                    Ty::Pointer(inner) => {
                        matches!(
                            &inner.element,
                            Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        ) || self
                            .lookup_struct_layout(&inner.element)
                            .is_some_and(Self::is_fn_pointer_layout)
                    }
                    Ty::Ref(inner) => {
                        matches!(
                            &inner.element,
                            Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                        ) || self
                            .lookup_struct_layout(&inner.element)
                            .is_some_and(Self::is_fn_pointer_layout)
                    }
                    _ => false,
                };
                if pointer_to_fn {
                    let fn_ty = match &ty {
                        Ty::Pointer(inner) => inner.element.clone(),
                        Ty::Ref(inner) => inner.element.clone(),
                        Ty::Nullable(inner) => match inner.as_ref() {
                            Ty::Pointer(ptr) => ptr.element.clone(),
                            Ty::Ref(r) => r.element.clone(),
                            other => other.clone(),
                        },
                        other => other.clone(),
                    };
                    let layout = self.lookup_struct_layout(&fn_ty).cloned().ok_or_else(|| {
                        Error::Codegen(format!(
                            "missing function pointer layout for `{}` in WASM backend",
                            fn_ty.canonical_name()
                        ))
                    })?;
                    self.emit_fn_invoke_argument(buf, arg, &fn_ty, &layout)?;
                    continue;
                }

                // WASM internal calling convention passes non-scalar (frame-allocated) values
                // by pointer to their storage. Keep this consistent even when the MIR argument
                // is passed by value (Copy/Move), otherwise callers can accidentally pass the
                // first field value instead of the aggregate address.
                if !matches!(ty, Ty::Fn(_))
                    && !pointer_to_fn
                    && !self.ty_is_reference(&ty)
                    && local_requires_memory(&ty, self.layouts)
                {
                    match arg {
                        Operand::Copy(place) | Operand::Move(place) => {
                            let access = self.resolve_memory_access(place)?;
                            self.emit_pointer_expression(buf, &access)?;
                            continue;
                        }
                        Operand::Borrow(borrow) => {
                            let access = self.resolve_memory_access(&borrow.place)?;
                            self.emit_pointer_expression(buf, &access)?;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            self.emit_operand(buf, arg)?;
        }
        emit_instruction(buf, Op::Call(callee));
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            if multi_result {
                self.store_multi_call_result(
                    buf,
                    place,
                    expected_results
                        .as_ref()
                        .expect("multi_result implies expected results"),
                )?;
            } else {
                self.store_call_result(buf, place)?;
            }
        } else if expected_results
            .as_ref()
            .is_some_and(|results| results.len() == 1)
        {
            emit_instruction(buf, Op::Drop);
        } else if let Some(results) = expected_results.as_ref().filter(|r| r.len() > 1) {
            for _ in results {
                emit_instruction(buf, Op::Drop);
            }
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_indirect_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
        fn_ty: FnTy,
    ) -> Result<(), Error> {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            return self.emit_extern_indirect_call(buf, call, fn_ty);
        }
        let signature = FunctionSignature::from_fn_ty(&fn_ty, self.layouts);
        let type_index = *self.signature_indices.get(&signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer signature `{}` is not registered in WASM type table",
                fn_ty.canonical_name()
            ))
        })?;
        let ret_is_sret = self.ty_requires_sret(fn_ty.ret.as_ref());
        if call.args.len() != fn_ty.params.len() {
            return Err(Error::Codegen(format!(
                "function pointer call expected {} argument(s) but found {}",
                fn_ty.params.len(),
                call.args.len()
            )));
        }

        if let Some(operand_ty) = self.operand_ty(&call.func) {
            let (trait_ty, via_pointer) = match &operand_ty {
                Ty::Pointer(inner) => (inner.element.clone(), true),
                Ty::Ref(inner) => (inner.element.clone(), true),
                Ty::Nullable(inner) => match inner.as_ref() {
                    Ty::Pointer(ptr) => (ptr.element.clone(), true),
                    Ty::Ref(r) => (r.element.clone(), true),
                    other => (other.clone(), false),
                },
                other => (other.clone(), false),
            };
            if self.ty_is_trait_object_like(&trait_ty) {
                let layout = self.lookup_struct_layout(&trait_ty).cloned();
                let (context_offset, vtable_offset) = layout
                    .as_ref()
                    .and_then(|layout| {
                        let context_offset = layout
                            .fields
                            .iter()
                            .find_map(|field| field.offset.filter(|off| *off == 0));
                        let vtable_offset = layout
                            .fields
                            .iter()
                            .find_map(|field| {
                                if field.name.contains("vtable") {
                                    field.offset
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                layout
                                    .fields
                                    .iter()
                                    .find_map(|field| field.offset.filter(|off| *off != 0))
                            });
                        Some((
                            context_offset.unwrap_or(0) as u32,
                            vtable_offset.unwrap_or(4) as u32,
                        ))
                    })
                    .unwrap_or((0, 4));
                let context_offset = ensure_u32(
                    context_offset as usize,
                    "trait object context offset exceeds wasm32 range",
                )?;
                let vtable_offset = ensure_u32(
                    vtable_offset as usize,
                    "trait object vtable offset exceeds wasm32 range",
                )?;

                let base_ptr = match &call.func {
                    Operand::Copy(place) | Operand::Move(place) => {
                        if via_pointer {
                            let ptr_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                            Self::ensure_operand_type(
                                ptr_ty,
                                ValueType::I32,
                                "trait object pointer",
                            )?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else if let Ok(access) = self.resolve_memory_access(place) {
                            self.emit_pointer_expression(buf, &access)?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else if matches!(
                            self.representations.get(place.local.0),
                            Some(
                                LocalRepresentation::PointerParam
                                    | LocalRepresentation::FrameAllocated
                                    | LocalRepresentation::Scalar
                            )
                        ) {
                            let ptr_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                            Self::ensure_operand_type(
                                ptr_ty,
                                ValueType::I32,
                                "trait object pointer",
                            )?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else {
                            None
                        }
                    }
                    Operand::Borrow(borrow) => {
                        if let Ok(access) = self.resolve_memory_access(&borrow.place) {
                            self.emit_pointer_expression(buf, &access)?;
                            emit_instruction(buf, Op::LocalSet(self.block_local));
                            Some(self.block_local)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(base_local) = base_ptr {
                    emit_instruction(buf, Op::LocalGet(base_local));
                    if context_offset != 0 {
                        emit_instruction(buf, Op::I32Const(context_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

                    emit_instruction(buf, Op::LocalGet(base_local));
                    if vtable_offset != 0 {
                        emit_instruction(buf, Op::I32Const(vtable_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Eqz);
                    emit_instruction(buf, Op::If);
                    Self::emit_trap(buf);
                    emit_instruction(buf, Op::End);

                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    if ret_is_sret {
                        self.emit_sret_out_pointer(
                            buf,
                            call.destination,
                            Some(fn_ty.ret.as_ref()),
                        )?;
                    }
                    for (index, arg) in call.args.iter().enumerate() {
                        let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                        self.emit_call_argument_for_mode(buf, arg, mode)?;
                    }
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(
                        buf,
                        Op::CallIndirect {
                            type_index,
                            table_index: 0,
                        },
                    );
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if ret_is_sret {
                        emit_instruction(buf, Op::Drop);
                    } else if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
            }
        }

        if let Operand::Copy(place) | Operand::Move(place) = call.func {
            if matches!(
                self.representations.get(place.local.0),
                Some(LocalRepresentation::Scalar)
            ) {
                let invoke_ty = self.emit_operand(buf, &Operand::Copy(place.clone()))?;
                Self::ensure_operand_type(invoke_ty, ValueType::I32, "fn invoke value")?;
                emit_instruction(buf, Op::LocalTee(self.temp_local));
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::If);
                Self::emit_trap(buf);
                emit_instruction(buf, Op::End);
                emit_instruction(buf, Op::I32Const(0));
                if ret_is_sret {
                    self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
                }
                for (index, arg) in call.args.iter().enumerate() {
                    let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                    self.emit_call_argument_for_mode(buf, arg, mode)?;
                }
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(
                    buf,
                    Op::CallIndirect {
                        type_index,
                        table_index: 0,
                    },
                );
                self.release_call_borrows(buf, call.args, call.modes)?;
                if ret_is_sret {
                    emit_instruction(buf, Op::Drop);
                } else if let Some(place) = call.destination {
                    self.store_call_result(buf, place)?;
                }
                self.emit_goto(buf, call.target);
                return Ok(());
            }
        }
        if let Some(layout) = self.lookup_struct_layout(&Ty::Fn(fn_ty.clone())).cloned() {
            let invoke_offset = layout
                .fields
                .iter()
                .find_map(|field| {
                    Self::fn_field_key(&field.name)
                        .filter(|key| *key == "invoke")
                        .and_then(|_| field.offset)
                })
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "function pointer layout `{}` missing invoke field for WASM lowering",
                        layout.name
                    ))
                })?;
            let context_offset = layout
                .fields
                .iter()
                .find_map(|field| {
                    Self::fn_field_key(&field.name)
                        .filter(|key| *key == "context")
                        .and_then(|_| field.offset)
                })
                .ok_or_else(|| {
                    Error::Codegen(format!(
                        "function pointer layout `{}` missing context field for WASM lowering",
                        layout.name
                    ))
                })?;
            let invoke_offset = ensure_u32(
                invoke_offset,
                "function pointer invoke offset exceeds wasm32 range",
            )?;
            let context_offset = ensure_u32(
                context_offset,
                "function pointer context offset exceeds wasm32 range",
            )?;

            match &call.func {
                Operand::Copy(place) | Operand::Move(place) => {
                    if !matches!(
                        self.representations.get(place.local.0),
                        Some(LocalRepresentation::Scalar)
                    ) {
                        let access = self.resolve_memory_access(place)?;
                        self.emit_pointer_expression(buf, &access)?;
                        emit_instruction(buf, Op::LocalSet(self.block_local));

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalTee(self.temp_local));
                        emit_instruction(buf, Op::I32Eqz);
                        emit_instruction(buf, Op::If);
                        Self::emit_trap(buf);
                        emit_instruction(buf, Op::End);

                        emit_instruction(buf, Op::LocalGet(self.block_local));
                        if context_offset != 0 {
                            emit_instruction(buf, Op::I32Const(context_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        if ret_is_sret {
                            self.emit_sret_out_pointer(
                                buf,
                                call.destination,
                                Some(fn_ty.ret.as_ref()),
                            )?;
                        }
                        for (index, arg) in call.args.iter().enumerate() {
                            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                            self.emit_call_argument_for_mode(buf, arg, mode)?;
                        }
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        emit_instruction(
                            buf,
                            Op::CallIndirect {
                                type_index,
                                table_index: 0,
                            },
                        );
                        self.release_call_borrows(buf, call.args, call.modes)?;
                        if ret_is_sret {
                            emit_instruction(buf, Op::Drop);
                        } else if let Some(place) = call.destination {
                            self.store_call_result(buf, place)?;
                        }
                        self.emit_goto(buf, call.target);
                        return Ok(());
                    }
                }
                Operand::Borrow(borrow) => {
                    let access = self.resolve_memory_access(&borrow.place)?;
                    self.emit_pointer_expression(buf, &access)?;
                    emit_instruction(buf, Op::LocalSet(self.block_local));

                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    if invoke_offset != 0 {
                        emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    emit_instruction(buf, Op::LocalTee(self.temp_local));
                    emit_instruction(buf, Op::I32Eqz);
                    emit_instruction(buf, Op::If);
                    Self::emit_trap(buf);
                    emit_instruction(buf, Op::End);

                    emit_instruction(buf, Op::LocalGet(self.block_local));
                    if context_offset != 0 {
                        emit_instruction(buf, Op::I32Const(context_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    if ret_is_sret {
                        self.emit_sret_out_pointer(
                            buf,
                            call.destination,
                            Some(fn_ty.ret.as_ref()),
                        )?;
                    }
                    for (index, arg) in call.args.iter().enumerate() {
                        let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                        self.emit_call_argument_for_mode(buf, arg, mode)?;
                    }
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    emit_instruction(
                        buf,
                        Op::CallIndirect {
                            type_index,
                            table_index: 0,
                        },
                    );
                    self.release_call_borrows(buf, call.args, call.modes)?;
                    if ret_is_sret {
                        emit_instruction(buf, Op::Drop);
                    } else if let Some(place) = call.destination {
                        self.store_call_result(buf, place)?;
                    }
                    self.emit_goto(buf, call.target);
                    return Ok(());
                }
                _ => {}
            }
        }
        let mut base_place = match call.func {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => {
                return Err(Error::Codegen(
                    "function pointer operand must be place-backed in WASM backend".into(),
                ));
            }
        };

        let mut invoke_place = base_place.clone();
        invoke_place
            .projection
            .push(crate::mir::ProjectionElem::FieldNamed("invoke".into()));
        let invoke_operand = Operand::Copy(invoke_place);

        base_place
            .projection
            .push(crate::mir::ProjectionElem::FieldNamed("context".into()));
        let context_operand = Operand::Copy(base_place);

        self.emit_operand(buf, &context_operand)?;
        if ret_is_sret {
            self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let invoke_ty = self.emit_operand(buf, &invoke_operand)?;
        if invoke_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "function pointer `invoke` field must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalTee(self.temp_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        Self::emit_trap(buf);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_extern_indirect_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
        fn_ty: FnTy,
    ) -> Result<(), Error> {
        let signature = FunctionSignature::from_fn_ty(&fn_ty, self.layouts);
        let type_index = *self.signature_indices.get(&signature).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer signature `{}` is not registered in WASM type table",
                fn_ty.canonical_name()
            ))
        })?;
        let ret_is_sret = self.ty_requires_sret(fn_ty.ret.as_ref());
        if call.args.len() != fn_ty.params.len() {
            return Err(Error::Codegen(format!(
                "function pointer call expected {} argument(s) but found {}",
                fn_ty.params.len(),
                call.args.len()
            )));
        }

        if ret_is_sret {
            self.emit_sret_out_pointer(buf, call.destination, Some(fn_ty.ret.as_ref()))?;
        }
        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let fn_ptr_ty = self.emit_extern_fn_ptr_value(buf, call.func)?;
        Self::ensure_operand_type(fn_ptr_ty, ValueType::I32, "extern fn pointer value")?;
        emit_instruction(buf, Op::LocalTee(self.temp_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        Self::emit_trap(buf);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(
            buf,
            Op::CallIndirect {
                type_index,
                table_index: 0,
            },
        );
        self.release_call_borrows(buf, call.args, call.modes)?;
        if ret_is_sret {
            emit_instruction(buf, Op::Drop);
        } else if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        }
        self.emit_pending_exception_check(buf, call.unwind)?;
        self.emit_goto(buf, call.target);
        Ok(())
    }

    fn emit_extern_fn_ptr_value(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    return self.emit_load_from_place(buf, place);
                }
                self.emit_operand(buf, operand)
            }
            Operand::Borrow(borrow) => {
                let access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::I32Load(0));
                Ok(ValueType::I32)
            }
            _ => self.emit_operand(buf, operand),
        }
    }

    fn emit_fn_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        fn_ty: &FnTy,
    ) -> Result<(), Error> {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            let ptr_ty = self.emit_extern_fn_ptr_value(buf, operand)?;
            Self::ensure_operand_type(ptr_ty, ValueType::I32, "extern fn pointer argument")?;
            return Ok(());
        }
        let fn_ty_wrapped = Ty::Fn(fn_ty.clone());
        let layout = self
            .lookup_struct_layout(&fn_ty_wrapped)
            .cloned()
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing function pointer layout for `{}` in WASM backend",
                    fn_ty_wrapped.canonical_name()
                ))
            })?;
        let allocation =
            compute_aggregate_allocation(&fn_ty_wrapped, self.layouts).ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer layout missing allocation metadata for `{}` in WASM backend",
                    fn_ty_wrapped.canonical_name()
                ))
            })?;
        self.allocate_stack_block(buf, allocation.size, allocation.align)?;

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        match operand {
            Operand::Const(constant) => {
                self.initialise_fn_from_const(buf, &layout, &fn_ty_wrapped, constant)?;
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                    eprintln!(
                        "[wasm-fn-arg] func={} operand_local={} repr={:?}",
                        self.function.name,
                        place.local.0,
                        self.representations.get(place.local.0),
                    );
                }
                if let Ok(src_access) = self.resolve_memory_access(place) {
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(
                            buf,
                            &layout,
                            &fn_ty_wrapped,
                            self.block_local,
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(
                            buf,
                            &layout,
                            &fn_ty_wrapped,
                            self.block_local,
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                }
                let ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(ty, ValueType::I32, "fn invoke argument")?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.initialise_fn_from_invoke(buf, &layout, &fn_ty_wrapped, self.block_local)?;
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, &layout, self.stack_temp_local, self.temp_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported function argument operand for WASM lowering".into(),
                ));
            }
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        Ok(())
    }

    #[allow(dead_code)]
    fn emit_fn_like_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        ty: &Ty,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        match ty {
            Ty::Fn(fn_ty) => return self.emit_fn_argument(buf, operand, fn_ty),
            _ => {}
        }

        // Handle pointer/ref/nullable-to-fn layouts by copying from the pointed-to memory.
        let (base_ty, via_pointer) = match ty {
            Ty::Pointer(inner) => (inner.element.clone(), true),
            Ty::Ref(inner) => (inner.element.clone(), true),
            Ty::Nullable(inner) => match inner.as_ref() {
                Ty::Pointer(ptr) => (ptr.element.clone(), true),
                Ty::Ref(r) => (r.element.clone(), true),
                other => (other.clone(), false),
            },
            other => (other.clone(), false),
        };
        if via_pointer {
            if let Some(base_layout) = self
                .lookup_struct_layout(&base_ty)
                .cloned()
                .filter(|l| Self::is_fn_pointer_layout(l))
            {
                let allocation =
                    compute_aggregate_allocation(&base_ty, self.layouts).ok_or_else(|| {
                        Error::Codegen(format!(
                            "function pointer layout missing allocation metadata for `{}` in WASM backend",
                            base_ty.canonical_name()
                        ))
                    })?;
                self.allocate_stack_block(buf, allocation.size, allocation.align)?;
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                match operand {
                    Operand::Copy(_place) | Operand::Move(_place) => {
                        let pointer_ty = self.emit_operand(buf, operand)?;
                        Self::ensure_operand_type(
                            pointer_ty,
                            ValueType::I32,
                            "fn pointer argument",
                        )?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(
                            buf,
                            &base_layout,
                            self.stack_temp_local,
                            self.temp_local,
                        )?;
                    }
                    Operand::Borrow(borrow) => {
                        let src_access = self.resolve_memory_access(&borrow.place)?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                        self.copy_fn_fields(
                            buf,
                            &base_layout,
                            self.stack_temp_local,
                            self.temp_local,
                        )?;
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported function pointer argument operand for WASM lowering"
                                .into(),
                        ));
                    }
                }
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(());
            }
        }

        let allocation = compute_aggregate_allocation(ty, self.layouts).ok_or_else(|| {
            Error::Codegen(format!(
                "function pointer layout missing allocation metadata for `{}` in WASM backend",
                ty.canonical_name()
            ))
        })?;
        self.allocate_stack_block(buf, allocation.size, allocation.align)?;

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        match operand {
            Operand::Const(constant) => {
                self.initialise_fn_from_const(buf, layout, ty, constant)?;
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                    eprintln!(
                        "[wasm-fn-like-arg] func={} operand_local={} repr={:?}",
                        self.function.name,
                        place.local.0,
                        self.representations.get(place.local.0),
                    );
                }
                if let Ok(src_access) = self.resolve_memory_access(place) {
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        self.emit_pointer_expression(buf, &src_access)?;
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                    self.emit_pointer_expression(buf, &src_access)?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.temp_local));
                    return Ok(());
                }
                if matches!(
                    self.representations.get(place.local.0),
                    Some(LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated)
                ) {
                    let pointer_ty = self.emit_operand(buf, operand)?;
                    Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                    emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                    if let Some(invoke_offset) = layout.fields.iter().find_map(|field| {
                        Self::fn_field_key(&field.name)
                            .filter(|key| *key == "invoke")
                            .and_then(|_| field.offset)
                    }) {
                        let invoke_offset = ensure_u32(
                            invoke_offset,
                            "function pointer invoke offset exceeds wasm32 range",
                        )?;
                        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                        if invoke_offset != 0 {
                            emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                            emit_instruction(buf, Op::I32Add);
                        }
                        emit_instruction(buf, Op::I32Load(0));
                        emit_instruction(buf, Op::LocalSet(self.block_local));
                        self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
                        emit_instruction(buf, Op::LocalGet(self.temp_local));
                        return Ok(());
                    }
                }
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "fn invoke argument")?;
                emit_instruction(buf, Op::LocalSet(self.block_local));
                self.initialise_fn_from_invoke(buf, layout, ty, self.block_local)?;
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.copy_fn_fields(buf, layout, self.stack_temp_local, self.temp_local)?;
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported function argument operand for WASM lowering".into(),
                ));
            }
        }

        emit_instruction(buf, Op::LocalGet(self.temp_local));
        Ok(())
    }

    pub(crate) fn emit_fn_invoke_argument(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        ty: &Ty,
        layout: &StructLayout,
    ) -> Result<(), Error> {
        if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
            eprintln!(
                "[wasm-fn-invoke-arg] func={} operand={:?} ty={} layout={} repr={:?}",
                self.function.name,
                operand,
                ty.canonical_name(),
                layout.name,
                match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        self.representations.get(place.local.0)
                    }
                    _ => None,
                }
            );
        }
        let invoke_offset = layout
            .fields
            .iter()
            .find_map(|field| {
                Self::fn_field_key(&field.name)
                    .filter(|key| *key == "invoke")
                    .and_then(|_| field.offset)
            })
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "missing invoke field offset for `{}` (type `{}`) in WASM backend",
                    layout.name,
                    ty.canonical_name()
                ))
            })?;
        let invoke_offset = ensure_u32(
            invoke_offset,
            "function pointer invoke offset exceeds wasm32 range",
        )?;
        match operand {
            Operand::Const(constant) => {
                let value = match constant.value() {
                    ConstValue::Symbol(name) => {
                        let index = self.lookup_function_index(name).ok_or_else(|| {
                            Error::Codegen(format!(
                                "unable to resolve function `{name}` for function pointer literal"
                            ))
                        })?;
                        i32::try_from(index).map_err(|_| {
                            Error::Codegen(
                                "function index exceeds i32 range in WASM backend".into(),
                            )
                        })?
                    }
                    ConstValue::Null => 0,
                    other => {
                        return Err(Error::Codegen(format!(
                            "function pointer invoke argument does not support constant operand {other:?}"
                        )));
                    }
                };
                emit_instruction(buf, Op::I32Const(value));
                return Ok(());
            }
            Operand::Copy(place) | Operand::Move(place) => {
                if let Ok(access) = self.resolve_memory_access(place) {
                    self.emit_pointer_expression(buf, &access)?;
                    if invoke_offset != 0 {
                        emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Load(0));
                    return Ok(());
                }
                let pointer_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(pointer_ty, ValueType::I32, "fn pointer argument")?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                if invoke_offset != 0 {
                    emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                    emit_instruction(buf, Op::I32Add);
                }
                emit_instruction(buf, Op::I32Load(0));
                return Ok(());
            }
            Operand::Borrow(borrow) => {
                let src_access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &src_access)?;
                if invoke_offset != 0 {
                    emit_instruction(buf, Op::I32Const(invoke_offset as i32));
                    emit_instruction(buf, Op::I32Add);
                }
                emit_instruction(buf, Op::I32Load(0));
                Ok(())
            }
            _ => Err(Error::Codegen(
                "unsupported function invoke argument for WASM lowering".into(),
            )),
        }
    }

    pub(crate) fn allocate_stack_block(
        &mut self,
        buf: &mut Vec<u8>,
        size: u32,
        align: u32,
    ) -> Result<(), Error> {
        let mask = if align > 1 { !((align as i32) - 1) } else { -1 };
        emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
        emit_instruction(buf, Op::LocalSet(self.temp_local)); // old SP
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Const(size as i32));
        emit_instruction(buf, Op::I32Sub);
        if align > 1 {
            emit_instruction(buf, Op::I32Const(mask));
            emit_instruction(buf, Op::I32And);
        }
        emit_instruction(buf, Op::LocalTee(self.stack_temp_local));
        emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));

        emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
        Ok(())
    }

    pub(crate) fn initialise_fn_from_invoke(
        &self,
        buf: &mut Vec<u8>,
        layout: &StructLayout,
        fn_ty: &Ty,
        invoke_local: u32,
    ) -> Result<(), Error> {
        let type_id = drop_type_identity(&fn_ty.canonical_name()) as i64;
        for field in &layout.fields {
            let offset = field.offset.ok_or_else(|| {
                Error::Codegen(format!(
                    "function pointer field `{}` missing offset for WASM lowering",
                    field.name
                ))
            })?;
            let offset = ensure_u32(
                offset,
                "function pointer field offset exceeds wasm32 addressable range",
            )?;
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            if offset != 0 {
                emit_instruction(buf, Op::I32Const(offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            let value_ty = map_type(&field.ty);
            let key = Self::fn_field_key(&field.name).ok_or_else(|| {
                Error::Codegen(format!(
                    "unknown function pointer field `{}` in WASM lowering",
                    field.name
                ))
            })?;
            match key {
                "invoke" => {
                    emit_instruction(buf, Op::LocalGet(invoke_local));
                }
                "context" | "drop_glue" | "env_size" | "env_align" => {
                    emit_instruction(buf, Op::I32Const(0));
                }
                "type_id" => {
                    emit_instruction(buf, Op::I64Const(type_id));
                }
                _ => unreachable!("unexpected function pointer field key"),
            }
            match value_ty {
                ValueType::I32 => emit_instruction(buf, Op::I32Store(0)),
                ValueType::I64 => emit_instruction(buf, Op::I64Store(0)),
                ValueType::F32 => emit_instruction(buf, Op::F32Store(0)),
                ValueType::F64 => emit_instruction(buf, Op::F64Store(0)),
            }
        }
        Ok(())
    }

    fn release_call_borrows(
        &mut self,
        buf: &mut Vec<u8>,
        args: &[Operand],
        modes: &[ParamMode],
    ) -> Result<(), Error> {
        for (arg, mode) in args.iter().zip(modes.iter()) {
            if !matches!(mode, ParamMode::Ref | ParamMode::Out | ParamMode::In) {
                continue;
            }
            match arg {
                Operand::Borrow(borrow) => {
                    let meta = self
                        .borrow_regions
                        .remove(&borrow.region.0)
                        .or_else(|| self.borrow_destinations.get(&borrow.place.local.0).cloned())
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "unable to resolve borrow metadata for argument to release `{:?}` in `{}`",
                                borrow.place, self.function.name
                            ))
                        })?;
                    if meta.kind != BorrowKind::Raw {
                        self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                    }
                    self.initialised_borrow_locals.remove(&borrow.place.local.0);
                }
                Operand::Copy(place) | Operand::Move(place) => {
                    let local_index = place.local.0;
                    let meta = self
                        .borrow_destinations
                        .get(&local_index)
                        .cloned()
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "missing borrow metadata for local {} when releasing call argument in `{}`",
                                local_index, self.function.name
                            ))
                        })?;
                    if meta.kind != BorrowKind::Raw {
                        self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
                    }
                    self.initialised_borrow_locals.remove(&local_index);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn emit_trap(buf: &mut Vec<u8>) {
        emit_instruction(buf, Op::Unreachable);
    }

    fn call_operand_fn_ty(&self, operand: &Operand) -> Option<FnTy> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.local_fn_ty(place),
            Operand::Borrow(borrow) => self.local_fn_ty(&borrow.place),
            _ => None,
        }
    }

    fn local_fn_ty(&self, place: &Place) -> Option<FnTy> {
        let mut ty = self.local_tys.get(place.local.0)?.clone();
        if !place.projection.is_empty() {
            if let Ok(plan) = self.compute_projection_offset(&ty, &place.projection) {
                ty = plan.value_ty;
            } else {
                return None;
            }
        }
        match ty {
            Ty::Fn(fn_ty) => Some(fn_ty.clone()),
            Ty::Nullable(inner) => match inner.as_ref() {
                Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                Ty::Named(named) => self
                    .layouts
                    .delegate_signature(&named.canonical_path())
                    .cloned(),
                _ => None,
            },
            Ty::Named(named) => self
                .layouts
                .delegate_signature(&named.canonical_path())
                .cloned(),
            _ => None,
        }
    }

    fn emit_await(
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

    fn emit_yield(
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

    fn emit_place_value(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<ValueType, Error> {
        let operand = Operand::Copy(place.clone());
        self.emit_operand(buf, &operand)
    }

    fn async_context_pointer(&self) -> Option<u32> {
        let idx = self
            .function
            .body
            .locals
            .iter()
            .position(|decl| decl.name.as_deref() == Some("__async_ctx"))?;
        let local = LocalId(idx);
        self.pointer_local_index(local).ok()
    }

    pub(super) fn mir_place_ty(&self, place: &Place) -> Result<Ty, Error> {
        let base_ty = self
            .local_tys
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?;
        let base_ty = self.resolve_self_ty(base_ty);
        Ok(self
            .compute_projection_offset(&base_ty, &place.projection)?
            .value_ty)
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

    fn try_emit_startup_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
    ) -> Result<bool, Error> {
        let canonical = match call.func {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        if !canonical.ends_with("::chic_rt_startup_exit") && canonical != "chic_rt_startup_exit" {
            return Ok(false);
        }
        if call.destination.is_some() {
            return Err(Error::Codegen(
                "startup exit call cannot assign to a destination in WASM backend".into(),
            ));
        }
        if call.args.len() != 1 {
            return Err(Error::Codegen(
                "startup exit call expects exactly one argument".into(),
            ));
        }
        let value_ty = self.emit_operand(buf, &call.args[0])?;
        if value_ty != ValueType::I32 {
            return Err(Error::Codegen(
                "startup exit argument must lower to i32 in WASM backend".into(),
            ));
        }
        let hook = self.runtime_hook_index(RuntimeHook::Abort)?;
        emit_instruction(buf, Op::Call(hook));
        self.release_call_borrows(buf, call.args, call.modes)?;
        self.emit_goto(buf, call.target);
        Ok(true)
    }

    fn try_emit_decimal_fast_runtime_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: &CallLowering<'_>,
    ) -> Result<bool, Error> {
        let canonical = match call.func {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => canonical_symbol_name(name),
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        let (hook, returns_struct) = match canonical.as_str() {
            "Std::Async::RuntimeIntrinsics::chic_rt_async_token_new"
            | "chic_rt_async_token_new" => (RuntimeHook::AsyncTokenNew, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_token_cancel"
            | "chic_rt_async_token_cancel" => (RuntimeHook::AsyncTokenCancel, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_task_header"
            | "chic_rt_async_task_header" => (RuntimeHook::AsyncTaskHeader, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn_local"
            | "chic_rt_async_spawn_local" => (RuntimeHook::AsyncSpawnLocal, false),
            "Std::Async::RuntimeIntrinsics::chic_rt_async_scope" | "chic_rt_async_scope" => {
                (RuntimeHook::AsyncScope, false)
            }
            "Std::Async::RuntimeIntrinsics::chic_rt_async_spawn" | "chic_rt_async_spawn" => {
                (RuntimeHook::AsyncSpawn, false)
            }
            "Std::Async::RuntimeIntrinsics::chic_rt_async_block_on" | "chic_rt_async_block_on" => {
                (RuntimeHook::AsyncScope, false)
            }
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_sum"
            | "chic_rt_decimal_sum" => (RuntimeHook::DecimalSum, true),
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_dot"
            | "chic_rt_decimal_dot" => (RuntimeHook::DecimalDot, true),
            "Std::Numeric::Decimal::RuntimeIntrinsics::chic_rt_decimal_matmul"
            | "chic_rt_decimal_matmul" => (RuntimeHook::DecimalMatMul, false),
            "chic_rt_closure_env_alloc" => (RuntimeHook::ClosureEnvAlloc, false),
            "chic_rt_closure_env_clone" => (RuntimeHook::ClosureEnvClone, false),
            "chic_rt_closure_env_free" => (RuntimeHook::ClosureEnvFree, false),
            _ => return Ok(false),
        };

        if returns_struct {
            let destination = call.destination.ok_or_else(|| {
                Error::Codegen(
                    "decimal runtime call must assign its result in the WASM backend".into(),
                )
            })?;
            let result_access = self.resolve_memory_access(destination)?;
            self.emit_pointer_expression(buf, &result_access)?;
            for (index, arg) in call.args.iter().enumerate() {
                let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
                self.emit_call_argument_for_mode(buf, arg, mode)?;
            }
            let hook_index = self.runtime_hook_index(hook)?;
            emit_instruction(buf, Op::Call(hook_index));
            self.release_call_borrows(buf, call.args, call.modes)?;
            self.emit_goto(buf, call.target);
            return Ok(true);
        }

        for (index, arg) in call.args.iter().enumerate() {
            let mode = call.modes.get(index).copied().unwrap_or(ParamMode::Value);
            self.emit_call_argument_for_mode(buf, arg, mode)?;
        }
        let hook_index = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::Call(hook_index));
        self.release_call_borrows(buf, call.args, call.modes)?;
        if let Some(place) = call.destination {
            self.store_call_result(buf, place)?;
        } else {
            let signature = hook.signature();
            match signature.results.len() {
                0 => {}
                1 => emit_instruction(buf, Op::Drop),
                n => {
                    for _ in 0..n {
                        emit_instruction(buf, Op::Drop);
                    }
                }
            }
        }
        self.emit_goto(buf, call.target);
        Ok(true)
    }

    fn store_call_result(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<(), Error> {
        let representation = self
            .representations
            .get(place.local.0)
            .copied()
            .unwrap_or(LocalRepresentation::Scalar);
        if place.projection.is_empty() && matches!(representation, LocalRepresentation::Scalar) {
            if let Some(index) = self.local_index(place.local) {
                emit_instruction(buf, Op::LocalSet(index));
            } else {
                emit_instruction(buf, Op::Drop);
            }
            return Ok(());
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));
        let access = self.resolve_memory_access(place)?;
        if access.load_pointer_from_slot && place.projection.is_empty() {
            emit_instruction(buf, Op::LocalGet(access.pointer_local));
            if access.offset != 0 {
                emit_instruction(buf, Op::I32Const(access.offset as i32));
                emit_instruction(buf, Op::I32Add);
            }
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            let value_ty = map_type(&access.value_ty);
            self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
            return Ok(());
        }
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        let value_ty = map_type(&access.value_ty);
        self.emit_store_to_access_for_ty(buf, &access.value_ty, value_ty);
        Ok(())
    }

    fn store_multi_call_result(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        results: &[ValueType],
    ) -> Result<(), Error> {
        if results.len() != 2 || results.iter().any(|ty| *ty != ValueType::I32) {
            return Err(Error::Codegen(
                "multi-value call results are only supported for i32 pointer/length pairs".into(),
            ));
        }
        let dest_ty = self.mir_place_ty(place)?;
        if matches!(dest_ty, Ty::Str) {
            // `str` is a packed scalar in wasm locals/values (len << 32 | ptr). Pack the
            // `(ptr, len)` multivalue return into the scalar representation before storing.
            emit_instruction(buf, Op::LocalSet(self.temp_local)); // len
            emit_instruction(buf, Op::LocalSet(self.stack_temp_local)); // ptr

            emit_instruction(buf, Op::LocalGet(self.temp_local));
            emit_instruction(buf, Op::I64ExtendI32U);
            emit_instruction(buf, Op::I64Const(32));
            emit_instruction(buf, Op::I64Shl);
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::I64ExtendI32U);
            emit_instruction(buf, Op::I64Or);

            self.store_value_into_place(buf, place, ValueType::I64)?;
            return Ok(());
        }
        let (ptr_offset, len_offset) = self.ptr_len_field_offsets(&dest_ty)?;

        emit_instruction(buf, Op::LocalSet(self.temp_local));
        emit_instruction(buf, Op::LocalSet(self.stack_temp_local));

        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::I32Store(ptr_offset));

        self.emit_pointer_expression(buf, &access)?;
        emit_instruction(buf, Op::LocalGet(self.temp_local));
        emit_instruction(buf, Op::I32Store(len_offset));

        Ok(())
    }

    fn ptr_len_field_offsets(&self, ty: &Ty) -> Result<(u32, u32), Error> {
        let (_, ptr_offset) = self.resolve_field_by_name(ty, None, "ptr")?;
        let (_, len_offset) = self.resolve_field_by_name(ty, None, "len")?;
        let ptr_u32 = ensure_u32(
            ptr_offset,
            "ptr field offset exceeds 32-bit range in WASM backend",
        )?;
        let len_u32 = ensure_u32(
            len_offset,
            "len field offset exceeds 32-bit range in WASM backend",
        )?;
        Ok((ptr_u32, len_u32))
    }

    fn set_block(&self, buf: &mut Vec<u8>, block: BlockId) {
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

    fn resolve_callee(&self, operand: &Operand, args: &[Operand]) -> Result<u32, Error> {
        match operand {
            Operand::Pending(pending) => {
                if let Some(info) = &pending.info {
                    let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                    let arg_types: Vec<Option<ValueType>> = args
                        .iter()
                        .map(|arg| match arg {
                            Operand::Const(constant) => match constant.value() {
                                ConstValue::Bool(_) => Some(ValueType::I32),
                                ConstValue::Int(value) | ConstValue::Int32(value) => {
                                    let bits = constant
                                        .literal()
                                        .and_then(|meta| match meta.literal_type {
                                            NumericLiteralType::Signed(width)
                                            | NumericLiteralType::Unsigned(width) => {
                                                Some(width.bit_width(self.pointer_width_bits()))
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            if *value >= i32::MIN as i128
                                                && *value <= i32::MAX as i128
                                            {
                                                32
                                            } else {
                                                64
                                            }
                                        });
                                    Some(if bits <= 32 {
                                        ValueType::I32
                                    } else {
                                        ValueType::I64
                                    })
                                }
                                ConstValue::UInt(value) => {
                                    let bits = constant
                                        .literal()
                                        .and_then(|meta| match meta.literal_type {
                                            NumericLiteralType::Unsigned(width)
                                            | NumericLiteralType::Signed(width) => {
                                                Some(width.bit_width(self.pointer_width_bits()))
                                            }
                                            _ => None,
                                        })
                                        .unwrap_or_else(|| {
                                            if *value <= u32::MAX as u128 { 32 } else { 64 }
                                        });
                                    Some(if bits <= 32 {
                                        ValueType::I32
                                    } else {
                                        ValueType::I64
                                    })
                                }
                                ConstValue::Char(_) => Some(ValueType::I32),
                                ConstValue::Float(value) => Some(if value.width.bits() <= 32 {
                                    ValueType::F32
                                } else {
                                    ValueType::F64
                                }),
                                ConstValue::Null => Some(ValueType::I32),
                                ConstValue::Str { .. } => Some(ValueType::I64),
                                ConstValue::RawStr(_) => Some(ValueType::I32),
                                ConstValue::Unit => Some(ValueType::I32),
                                _ => None,
                            },
                            other => self
                                .operand_ty(other)
                                .map(|ty| map_type(&self.resolve_self_ty(&ty))),
                        })
                        .collect();

                    let mut best: Option<(u32, usize)> = None;
                    for candidate in candidates {
                        let Some(index) = self.lookup_function_index(&candidate.qualified) else {
                            continue;
                        };
                        let canonical = candidate.qualified.replace('.', "::");
                        let signature = self.function_signatures.get(&canonical).or_else(|| {
                            canonical
                                .split('<')
                                .next()
                                .and_then(|base| self.function_signatures.get(base))
                        });
                        let Some(signature) = signature else {
                            continue;
                        };
                        let offset = if signature.params.len() == arg_types.len() {
                            0
                        } else if signature.params.len() == arg_types.len() + 1
                            && matches!(signature.params.first(), Some(ValueType::I32))
                            && signature.results.len() == 1
                            && signature.results[0] == ValueType::I32
                        {
                            1
                        } else {
                            continue;
                        };

                        let mut score = 0usize;
                        let mut mismatch = false;
                        for (idx, arg_ty) in arg_types.iter().enumerate() {
                            let Some(arg_ty) = arg_ty else {
                                continue;
                            };
                            let Some(expected) = signature.params.get(idx + offset) else {
                                mismatch = true;
                                break;
                            };
                            if *expected == *arg_ty {
                                score += 1;
                            } else {
                                mismatch = true;
                                break;
                            }
                        }
                        if mismatch {
                            continue;
                        }
                        if best.map_or(true, |(_, best_score)| score > best_score) {
                            best = Some((index, score));
                        }
                    }
                    if let Some((index, _)) = best {
                        return Ok(index);
                    }
                    if let Some(index) = candidates
                        .iter()
                        .find_map(|candidate| self.lookup_function_index(&candidate.qualified))
                    {
                        return Ok(index);
                    }
                }
                let repr = pending.repr.replace('.', "::");
                if !repr.ends_with("::init#super") && !repr.ends_with("::init#self") {
                    if let Some(idx) = self.lookup_function_index(&repr) {
                        return Ok(idx);
                    }
                }
                if repr.ends_with("::init#super") {
                    if let Some((owner, _)) = repr.rsplit_once("::init#super") {
                        if let Some(class) = self.layouts.class_layout_info(owner) {
                            if let Some(base) = class.bases.first() {
                                let base_key = base.replace('.', "::");
                                let canonical_base = self
                                    .layouts
                                    .resolve_type_key(base_key.as_str())
                                    .unwrap_or(base_key.as_str())
                                    .to_string();
                                let expected = args.len();
                                let matches_base = |name: &str| {
                                    name.starts_with(&format!("{base_key}::init#"))
                                        || name.starts_with(&format!("{canonical_base}::init#"))
                                };
                                if let Some((_, idx)) = self.functions.iter().find(|(name, _)| {
                                    matches_base(name)
                                        && self
                                            .function_signatures
                                            .get(*name)
                                            .map(|sig| sig.params.len() == expected)
                                            .unwrap_or(false)
                                }) {
                                    return Ok(*idx);
                                }
                                if let Some((_, idx)) =
                                    self.functions.iter().find(|(name, _)| matches_base(name))
                                {
                                    return Ok(*idx);
                                }
                                if let Some(idx) =
                                    self.functions.get(&format!("{canonical_base}::init"))
                                {
                                    return Ok(*idx);
                                }
                            }
                        }
                    }
                }
                if repr.ends_with("::init#self") {
                    if let Some((owner, _)) = repr.rsplit_once("::init#self") {
                        let expected = args.len();
                        if let Some((_, idx)) = self.functions.iter().find(|(name, _)| {
                            name.starts_with(&format!("{owner}::init#"))
                                && self
                                    .function_signatures
                                    .get(*name)
                                    .map(|sig| sig.params.len() == expected)
                                    .unwrap_or(false)
                        }) {
                            return Ok(*idx);
                        }
                        if let Some((_, idx)) = self
                            .functions
                            .iter()
                            .find(|(name, _)| name.starts_with(&format!("{owner}::init#")))
                        {
                            return Ok(*idx);
                        }
                    }
                }
                if let Some(idx) = self.functions.iter().find_map(|(name, index)| {
                    if name == &repr || name.ends_with(&format!("::{repr}")) {
                        Some(*index)
                    } else {
                        None
                    }
                }) {
                    Ok(idx)
                } else if let Some(method) = repr.rsplit("::").next() {
                    if let Some(idx) = self.functions.iter().find_map(|(name, index)| {
                        if name.ends_with(&format!("::{method}")) {
                            Some(*index)
                        } else {
                            None
                        }
                    }) {
                        Ok(idx)
                    } else {
                        Err(Error::Codegen(format!(
                            "unable to resolve call target '{repr}' in WASM backend"
                        )))
                    }
                } else {
                    Err(Error::Codegen(format!(
                        "unable to resolve call target '{repr}' in WASM backend"
                    )))
                }
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let ty = self
                    .local_tys
                    .get(place.local.0)
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<unknown>".into());
                Err(Error::Codegen(format!(
                    "first-class function values are not yet supported by the WASM backend (local {} type {} projection {:?})",
                    place.local.0, ty, place.projection
                )))
            }
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => {
                    if let Some(index) = self.lookup_function_index(name) {
                        Ok(index)
                    } else if name.contains("AsUtf8Span") || name.contains("AsUtf8") {
                        // Treat missing UTF-8 span helpers as runtime string slice accessors.
                        self.runtime_hook_index(RuntimeHook::StringAsSlice)
                    } else if name.contains("TryCopyUtf8") {
                        self.runtime_hook_index(RuntimeHook::StringTryCopyUtf8)
                    } else if name.contains("AsSpan") {
                        // Treat missing char-span helpers as runtime string UTF-16 views.
                        if name.starts_with("str::") || name.contains("::str::") {
                            self.runtime_hook_index(RuntimeHook::StrAsChars)
                        } else {
                            self.runtime_hook_index(RuntimeHook::StringAsChars)
                        }
                    } else {
                        Err(Error::Codegen(format!(
                            "unable to resolve function `{name}` in WASM backend"
                        )))
                    }
                }
                _ => Err(Error::Codegen(
                    "unsupported constant call operand for WASM backend".into(),
                )),
            },
            _ => Err(Error::Codegen(
                "unsupported call operand for WASM backend".into(),
            )),
        }
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
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
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
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
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
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
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

    fn convert_switch_value(value: i128) -> Result<i32, Error> {
        i32::try_from(value).map_err(|_| {
            Error::Codegen(
                "switch literal outside 32-bit range is unsupported by the WASM backend".into(),
            )
        })
    }
}

fn canonical_symbol_name(name: &str) -> String {
    name.replace('.', "::")
}

fn ensure_std_runtime_intrinsic_owner(callee: &str) -> Result<(), Error> {
    let symbol = callee.rsplit("::").next().unwrap_or(callee);
    const SPAN_PREFIX: &str = concat!("chic_rt_", "span_");
    const VEC_PREFIX: &str = concat!("chic_rt_", "vec_");
    const ARRAY_PREFIX: &str = concat!("chic_rt_", "array_");
    if symbol.starts_with(SPAN_PREFIX) {
        if !callee.starts_with("Std::Span::SpanIntrinsics::")
            && !callee.starts_with("Std::Runtime::Native::SpanRuntime::")
            && !callee.starts_with("Std::Runtime::Native::")
        {
            return Err(Error::Codegen(format!(
                "span runtime intrinsic `{symbol}` must be routed through Std.Span.SpanIntrinsics or a runtime-native entrypoint (callee `{callee}`)"
            )));
        }
    }
    if symbol.starts_with(VEC_PREFIX) {
        let allowed = callee.starts_with("Foundation::Collections::VecIntrinsics::")
            || callee.starts_with("Std::Collections::VecIntrinsics::")
            || callee.starts_with("Std::Runtime::Native::VecRuntime::")
            || callee == symbol;
        if !allowed {
            return Err(Error::Codegen(format!(
                "vec runtime intrinsic `{symbol}` must be routed through Std/Foundations VecIntrinsics (callee `{callee}`)"
            )));
        }
    }
    if symbol.starts_with(ARRAY_PREFIX) {
        let allowed = callee.starts_with("Foundation::Collections::VecIntrinsics::")
            || callee.starts_with("Std::Collections::VecIntrinsics::")
            || callee.starts_with("Std::Runtime::Native::VecRuntime::");
        if !allowed {
            return Err(Error::Codegen(format!(
                "array runtime intrinsic `{symbol}` must be routed through a Std collections entrypoint (callee `{callee}`)"
            )));
        }
    }
    Ok(())
}
