use crate::codegen::wasm::RuntimeHook;
use crate::drop_glue::drop_glue_symbol_for;
use crate::error::Error;
use crate::mir::{BorrowId, BorrowKind, LocalId, Place, Ty};

use crate::codegen::wasm::ensure_u32;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_storage_dead(
        &mut self,
        buf: &mut Vec<u8>,
        local: LocalId,
    ) -> Result<(), Error> {
        if let Some(meta) = self.borrow_destinations.get(&local.0)
            && meta.kind != BorrowKind::Raw
            && self.initialised_borrow_locals.remove(&local.0)
        {
            self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
        }
        Ok(())
    }

    pub(super) fn emit_deinit_statement(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<(), Error> {
        let base_ty = self
            .local_tys
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?
            .clone();
        let value_ty = self
            .compute_projection_offset(&base_ty, &place.projection)?
            .value_ty;
        let Some(symbol) = self.dispose_symbol_for_ty(&value_ty) else {
            return Ok(());
        };
        let Some(index) = self.functions.get(symbol).copied() else {
            return Err(Error::Codegen(format!(
                "unable to resolve function `{symbol}` in WASM backend"
            )));
        };
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        super::emit_instruction(buf, super::Op::Call(index));
        Ok(())
    }

    pub(super) fn emit_drop_statement(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<(), Error> {
        if place.projection.is_empty() {
            if let Some(meta) = self.borrow_destinations.get(&place.local.0)
                && meta.kind != BorrowKind::Raw
                && self.initialised_borrow_locals.remove(&place.local.0)
            {
                self.emit_runtime_borrow_release(buf, meta.borrow_id)?;
            }
        }

        let base_ty = self
            .local_tys
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?
            .clone();
        let value_ty = self
            .compute_projection_offset(&base_ty, &place.projection)?
            .value_ty;

        match value_ty {
            Ty::String => self.emit_string_drop(buf, place)?,
            Ty::Vec(_) | Ty::Array(_) | Ty::Span(_) | Ty::ReadOnlySpan(_) => {
                self.emit_vec_drop(buf, place)?;
            }
            Ty::Rc(_) => self.emit_rc_drop(buf, place)?,
            Ty::Arc(_) => self.emit_arc_drop(buf, place)?,
            _ => {
                if !self.emit_drop_glue_for(buf, place, &value_ty)? {
                    self.emit_drop_missing(buf, place)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn emit_borrow_statement(
        &mut self,
        buf: &mut Vec<u8>,
        borrow_id: BorrowId,
        kind: BorrowKind,
        place: &Place,
    ) -> Result<(), Error> {
        match kind {
            BorrowKind::Raw => return Ok(()),
            BorrowKind::Shared | BorrowKind::Unique => {}
        }
        let hook = self.runtime_hook_index(match kind {
            BorrowKind::Shared => RuntimeHook::BorrowShared,
            BorrowKind::Unique => RuntimeHook::BorrowUnique,
            BorrowKind::Raw => unreachable!("raw borrows do not emit runtime hooks"),
        })?;
        let id = self.borrow_id_immediate(borrow_id)?;
        super::emit_instruction(buf, super::Op::I32Const(id));
        let access = self.resolve_memory_access(place)?;
        self.emit_pointer_expression(buf, &access)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_runtime_borrow_release(
        &mut self,
        buf: &mut Vec<u8>,
        borrow_id: BorrowId,
    ) -> Result<(), Error> {
        let hook = self.runtime_hook_index(RuntimeHook::BorrowRelease)?;
        let id = self.borrow_id_immediate(borrow_id)?;
        super::emit_instruction(buf, super::Op::I32Const(id));
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn dispose_symbol_for_ty(&self, ty: &Ty) -> Option<&String> {
        match ty {
            Ty::Named(name) => match self.layouts.types.get(name.as_str())? {
                crate::mir::TypeLayout::Struct(layout) | crate::mir::TypeLayout::Class(layout) => {
                    layout.dispose.as_ref()
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn borrow_id_immediate(&self, borrow_id: BorrowId) -> Result<i32, Error> {
        let raw = ensure_u32(
            borrow_id.0,
            "borrow identifier exceeds WebAssembly operand limits",
        )?;
        i32::try_from(raw).map_err(|_| {
            Error::Codegen(
                "borrow identifier exceeds 32-bit operand limits for WASM backend".into(),
            )
        })
    }

    pub(super) fn emit_string_drop(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<(), Error> {
        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::StringDrop)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_vec_drop(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<(), Error> {
        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::VecDrop)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_rc_drop(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<(), Error> {
        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::RcDrop)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_arc_drop(&mut self, buf: &mut Vec<u8>, place: &Place) -> Result<(), Error> {
        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::ArcDrop)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn emit_drop_missing(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<(), Error> {
        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }
        let hook = self.runtime_hook_index(RuntimeHook::DropMissing)?;
        super::emit_instruction(buf, super::Op::Call(hook));
        Ok(())
    }

    pub(super) fn drop_glue_symbol_for_ty(&self, ty: &Ty) -> Option<String> {
        match ty {
            Ty::Unknown | Ty::Unit => return None,
            Ty::String
            | Ty::Vec(_)
            | Ty::Array(_)
            | Ty::Span(_)
            | Ty::ReadOnlySpan(_)
            | Ty::Rc(_)
            | Ty::Arc(_) => return None,
            _ => {}
        }

        let canonical = ty.canonical_name();
        if self.layouts.ty_requires_drop(ty) || self.layouts.type_requires_drop(&canonical) {
            Some(drop_glue_symbol_for(&canonical))
        } else {
            None
        }
    }

    pub(super) fn emit_drop_glue_for(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        ty: &Ty,
    ) -> Result<bool, Error> {
        let Some(symbol) = self.drop_glue_symbol_for_ty(ty) else {
            return Ok(false);
        };

        let Some(index) = self.functions.get(&symbol) else {
            return Ok(false);
        };

        if place.projection.is_empty() {
            let ptr = self.pointer_local_index(place.local)?;
            super::emit_instruction(buf, super::Op::LocalGet(ptr));
        } else {
            let access = self.resolve_memory_access(place)?;
            self.emit_pointer_expression(buf, &access)?;
        }

        super::emit_instruction(buf, super::Op::Call(*index));
        Ok(true)
    }
}
