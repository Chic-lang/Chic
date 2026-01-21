use super::super::ops::{Op, emit_instruction};
use super::super::{FunctionEmitter, LocalRepresentation};
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::codegen::wasm::{
    RuntimeHook, STACK_POINTER_GLOBAL_INDEX, ValueType, compute_aggregate_allocation, ensure_u32,
    local_requires_memory, map_type,
};
use crate::drop_glue::drop_type_identity;
use crate::error::Error;
use crate::mir::{
    BlockId, BorrowKind, CallDispatch, ConstOperand, ConstValue, FnTy, Operand, ParamMode,
    PendingOperandInfo, Place, StructLayout, TraitObjectDispatch, Ty, VirtualDispatch,
};
use crate::syntax::numeric::NumericLiteralType;
use std::convert::TryFrom;
mod callee;
mod direct;
mod dispatch;
mod indirect;
mod results;
mod runtime;

#[derive(Clone, Copy)]
pub(super) struct CallLowering<'a> {
    pub(super) func: &'a Operand,
    pub(super) args: &'a [Operand],
    pub(super) modes: &'a [ParamMode],
    pub(super) destination: Option<&'a Place>,
    pub(super) target: BlockId,
    pub(super) unwind: Option<BlockId>,
    pub(super) dispatch: Option<&'a CallDispatch>,
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

    pub(super) fn emit_call(
        &mut self,
        buf: &mut Vec<u8>,
        call: CallLowering<'_>,
    ) -> Result<(), Error> {
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
