use crate::codegen::wasm::RuntimeHook;
use crate::error::Error;
use crate::mir::{BinOp, ConstValue, Operand, Place, Rvalue, StrId, Ty};

use crate::codegen::wasm::emitter::function::FunctionEmitter;
use crate::codegen::wasm::emitter::function::LocalRepresentation;
use crate::codegen::wasm::emitter::function::ops::{Op, emit_instruction};

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_string_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Binary {
                op: bin_op,
                lhs,
                rhs,
                ..
            } if matches!(bin_op, BinOp::Add) => {
                self.emit_string_concat(buf, place, lhs, rhs)?;
                Ok(true)
            }
            Rvalue::Use(Operand::Copy(src)) => match self.mir_place_ty(src)? {
                Ty::String => {
                    self.emit_string_clone(buf, place, src)?;
                    Ok(true)
                }
                Ty::Str => {
                    self.emit_string_from_str(buf, place, &Operand::Copy(src.clone()))?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            Rvalue::Use(Operand::Move(src)) => match self.mir_place_ty(src)? {
                Ty::String => {
                    self.emit_string_clone(buf, place, src)?;
                    self.emit_string_drop(buf, src)?;
                    Ok(true)
                }
                Ty::Str => {
                    self.emit_string_from_str(buf, place, &Operand::Move(src.clone()))?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            Rvalue::Use(Operand::Const(constant)) => {
                if let ConstValue::Str { id, .. } = &constant.value {
                    self.emit_string_from_literal(buf, place, *id)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Rvalue::StringInterpolate { segments } => {
                self.emit_string_interpolate(buf, place, segments)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn emit_string_from_str(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Operand,
    ) -> Result<(), Error> {
        if dest.projection.is_empty() {
            let dest_ptr = self.pointer_local_index(dest.local)?;
            emit_instruction(buf, Op::LocalGet(dest_ptr));
        } else {
            let access = self.resolve_memory_access(dest)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let src_ty = self.emit_operand(buf, src)?;
        Self::ensure_operand_type(
            src_ty,
            crate::codegen::wasm::types::ValueType::I64,
            "str operand",
        )?;
        emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

        self.allocate_stack_block(buf, 8, 8)?;
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
        emit_instruction(buf, Op::I64Store(0));

        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        let hook = self.runtime_hook_index(RuntimeHook::StringFromSlice)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    pub(crate) fn emit_vec_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => match self.mir_place_ty(src)? {
                Ty::Vec(_) | Ty::Array(_) => {
                    self.emit_vec_clone(buf, place, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            Rvalue::Use(Operand::Move(src)) => match self.mir_place_ty(src)? {
                Ty::Vec(_) | Ty::Array(_) => {
                    self.emit_vec_clone(buf, place, src)?;
                    self.emit_vec_drop(buf, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    pub(crate) fn emit_rc_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => match self.mir_place_ty(src)? {
                Ty::Rc(_) => {
                    self.emit_rc_clone(buf, place, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            Rvalue::Use(Operand::Move(src)) => match self.mir_place_ty(src)? {
                Ty::Rc(_) => {
                    self.emit_rc_clone(buf, place, src)?;
                    self.emit_rc_drop(buf, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    pub(crate) fn emit_arc_assignment(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => match self.mir_place_ty(src)? {
                Ty::Arc(_) => {
                    self.emit_arc_clone(buf, place, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            Rvalue::Use(Operand::Move(src)) => match self.mir_place_ty(src)? {
                Ty::Arc(_) => {
                    self.emit_arc_clone(buf, place, src)?;
                    self.emit_arc_drop(buf, src)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn emit_string_clone(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
    ) -> Result<(), Error> {
        if dest.projection.is_empty() {
            let dest_ptr = self.pointer_local_index(dest.local)?;
            emit_instruction(buf, Op::LocalGet(dest_ptr));
        } else {
            let access = self.resolve_memory_access(dest)?;
            self.emit_pointer_expression(buf, &access)?;
        }

        if src.projection.is_empty() {
            let src_ptr = self.pointer_local_index(src.local)?;
            emit_instruction(buf, Op::LocalGet(src_ptr));
        } else {
            let access = self.resolve_memory_access(src)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::StringClone)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn emit_vec_clone(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
    ) -> Result<(), Error> {
        if dest.projection.is_empty() {
            let dest_ptr = self.pointer_local_index(dest.local)?;
            emit_instruction(buf, Op::LocalGet(dest_ptr));
        } else {
            let access = self.resolve_memory_access(dest)?;
            self.emit_pointer_expression(buf, &access)?;
        }

        if src.projection.is_empty() {
            let src_ptr = self.pointer_local_index(src.local)?;
            emit_instruction(buf, Op::LocalGet(src_ptr));
        } else {
            let access = self.resolve_memory_access(src)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::VecClone)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn emit_rc_clone(&mut self, buf: &mut Vec<u8>, dest: &Place, src: &Place) -> Result<(), Error> {
        if dest.projection.is_empty() {
            let dest_ptr = self.pointer_local_index(dest.local)?;
            emit_instruction(buf, Op::LocalGet(dest_ptr));
        } else {
            let access = self.resolve_memory_access(dest)?;
            self.emit_pointer_expression(buf, &access)?;
        }

        if src.projection.is_empty() {
            let src_ptr = self.pointer_local_index(src.local)?;
            emit_instruction(buf, Op::LocalGet(src_ptr));
        } else {
            let access = self.resolve_memory_access(src)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::RcClone)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn emit_arc_clone(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        src: &Place,
    ) -> Result<(), Error> {
        if !dest.projection.is_empty() || !src.projection.is_empty() {
            let dest_access = self.resolve_memory_access(dest)?;
            let src_access = self.resolve_memory_access(src)?;
            self.emit_pointer_expression(buf, &dest_access)?;
            self.emit_pointer_expression(buf, &src_access)?;
            let hook = self.runtime_hook_index(RuntimeHook::ArcClone)?;
            emit_instruction(buf, Op::Call(hook));
            emit_instruction(buf, Op::Drop);
            return Ok(());
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ARC_ASSIGN").is_some() {
            let message = format!(
                "[wasm-arc-clone] func={} dest_local={} dest_proj={:?} src_local={} src_proj={:?} dest_repr={:?} src_repr={:?}",
                self.function.name,
                dest.local.0,
                dest.projection,
                src.local.0,
                src.projection,
                self.representations.get(dest.local.0),
                self.representations.get(src.local.0),
            );
            eprintln!("{message}");
        }
        let dest_repr = self.representations.get(dest.local.0).copied();
        let src_repr = self.representations.get(src.local.0).copied();
        let dest_ptr_local = self.pointer_local_index(dest.local)?;
        let mut src_ptr = self.pointer_local_index(src.local)?;
        let (dest_size, dest_align) = self
            .local_tys
            .get(dest.local.0)
            .and_then(|ty| self.layouts.size_and_align_for_ty(ty))
            .map(|(size, align)| (size.max(1) as u32, align.max(1) as u32))
            .unwrap_or((4, 4));
        if !matches!(
            dest_repr,
            Some(LocalRepresentation::FrameAllocated | LocalRepresentation::PointerParam)
        ) {
            self.allocate_stack_block(buf, dest_size, dest_align)?;
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::LocalSet(dest_ptr_local));
        }

        if !matches!(
            src_repr,
            Some(LocalRepresentation::FrameAllocated | LocalRepresentation::PointerParam)
        ) {
            let (src_size, src_align) = self
                .local_tys
                .get(src.local.0)
                .and_then(|ty| self.layouts.size_and_align_for_ty(ty))
                .map(|(size, align)| (size.max(1) as u32, align.max(1) as u32))
                .unwrap_or((dest_size, dest_align));
            self.allocate_stack_block(buf, src_size, src_align)?;
            emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
            emit_instruction(buf, Op::LocalSet(self.temp_local));
            emit_instruction(buf, Op::LocalGet(self.temp_local));
            emit_instruction(buf, Op::LocalGet(src_ptr));
            emit_instruction(buf, Op::I32Store(0));
            src_ptr = self.temp_local;
        }

        emit_instruction(buf, Op::LocalGet(dest_ptr_local));
        emit_instruction(buf, Op::LocalGet(src_ptr));
        let hook = self.runtime_hook_index(RuntimeHook::ArcClone)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn emit_string_from_literal(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        id: StrId,
    ) -> Result<(), Error> {
        let literal = self.string_literals.get(&id).ok_or_else(|| {
            Error::Codegen(format!("missing interned string literal {}", id.index()))
        })?;
        if place.projection.is_empty() {
            let dest_ptr = self.pointer_local_index(place.local)?;
            emit_instruction(buf, Op::LocalGet(dest_ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        emit_instruction(buf, Op::I32Const(literal.offset as i32));
        emit_instruction(buf, Op::I32Const(literal.len as i32));
        let hook = self.runtime_hook_index(RuntimeHook::StringCloneSlice)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    fn emit_string_concat(
        &mut self,
        buf: &mut Vec<u8>,
        dest: &Place,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<(), Error> {
        self.emit_zero_init(buf, dest)?;
        let dest_ptr = if dest.projection.is_empty() {
            self.pointer_local_index(dest.local)?
        } else {
            let access = self.resolve_memory_access(dest)?;
            self.emit_pointer_expression(buf, &access)?;
            emit_instruction(buf, Op::LocalSet(self.temp_local));
            self.temp_local
        };
        self.emit_interpolation_expr_segment(buf, dest_ptr, lhs, None, None)?;
        self.emit_interpolation_expr_segment(buf, dest_ptr, rhs, None, None)?;
        Ok(())
    }
}
