use super::super::FunctionEmitter;
use crate::codegen::wasm::{ValueType, ensure_u32};
use crate::error::Error;
use crate::mir::{LocalId, Operand, Place, Ty};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_place_value(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
    ) -> Result<ValueType, Error> {
        let operand = Operand::Copy(place.clone());
        self.emit_operand(buf, &operand)
    }

    pub(super) fn async_context_pointer(&self) -> Option<u32> {
        let idx = self
            .function
            .body
            .locals
            .iter()
            .position(|decl| decl.name.as_deref() == Some("__async_ctx"))?;
        let local = LocalId(idx);
        self.pointer_local_index(local).ok()
    }

    pub(in super::super) fn mir_place_ty(&self, place: &Place) -> Result<Ty, Error> {
        let base_ty = self
            .local_tys
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?;
        let base_ty = self.resolve_self_ty(base_ty);
        Ok(self
            .compute_projection_offset(&base_ty, &place.projection)?
            .value_ty)
    }

    pub(super) fn ptr_len_field_offsets(&self, ty: &Ty) -> Result<(u32, u32), Error> {
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
}
