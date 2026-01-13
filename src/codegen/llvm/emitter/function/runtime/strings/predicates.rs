use crate::error::Error;
use crate::mir::{Place, Ty};

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn place_is_string(&self, place: &Place) -> Result<bool, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(matches!(ty, Ty::String))
    }

    pub(crate) fn place_is_str(&self, place: &Place) -> Result<bool, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(matches!(ty, Ty::Str))
    }
}
