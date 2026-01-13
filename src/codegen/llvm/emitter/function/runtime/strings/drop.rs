use std::fmt::Write;

use crate::error::Error;
use crate::mir::Place;

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_string_drop(&mut self, place: &Place) -> Result<(), Error> {
        let ptr = self.place_ptr(place)?;
        self.externals.insert("chic_rt_string_drop");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_string_drop(ptr {ptr})"
        )
        .ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::mir::{LocalId, Place, Ty};

    #[test]
    fn emits_string_drop_call_and_records_external() {
        let ((), externals) = with_emitter(vec![Ty::String], [], |emitter, _| {
            let place = Place {
                local: LocalId(0),
                projection: Vec::new(),
            };

            emitter
                .emit_string_drop(&place)
                .expect("drop should succeed");

            assert!(emitter.ir().contains("chic_rt_string_drop"));
        });
        assert!(externals.contains("chic_rt_string_drop"));
    }
}
