use super::*;
use crate::mir::pointer_align;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn place_alignment(&self, place: &Place) -> Result<usize, Error> {
        if place
            .projection
            .iter()
            .any(|elem| matches!(elem, ProjectionElem::Deref | ProjectionElem::Index(_)))
        {
            return Ok(1);
        }

        let decl = self
            .function
            .body
            .locals
            .get(place.local.0)
            .ok_or_else(|| Error::Codegen("place referenced unknown local".into()))?;
        let base_ty = self.infer_unknown_local_ty(place.local.0, decl.ty.clone());

        let offset = if place.projection.is_empty() {
            0
        } else {
            self.projection_offset(&base_ty, &place.projection)?.0
        };

        let base_align = self.align_for_ty(&base_ty);
        let addr_align = alignment_after_offset(base_align, offset);

        let value_ty = self.infer_unknown_local_ty(place.local.0, self.mir_ty_of_place(place)?);
        let value_align = self.align_for_ty(&value_ty);

        Ok(addr_align.min(value_align).max(1))
    }

    pub(crate) fn align_for_ty(&self, ty: &Ty) -> usize {
        self.type_layouts
            .size_and_align_for_ty(ty)
            .map(|(_, align)| align)
            .unwrap_or(pointer_align())
            .max(1)
    }
}

fn alignment_after_offset(base_align: usize, offset: usize) -> usize {
    if offset == 0 {
        return base_align.max(1);
    }
    let low_bit = offset & offset.wrapping_neg();
    base_align.min(low_bit).max(1)
}
