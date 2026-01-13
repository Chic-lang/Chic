use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
use crate::error::Error;
use crate::mir::{InterpolatedStringSegment, Place, StrId};

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;

mod numbers;
mod scalars;
mod segments;
mod strings;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_string_interpolate(
        &mut self,
        place: &Place,
        segments: &[InterpolatedStringSegment],
    ) -> Result<(), Error> {
        let dest_ptr = self.prepare_string_destination(place)?;
        for segment in segments {
            match segment {
                InterpolatedStringSegment::Text { id } => {
                    self.push_string_literal(dest_ptr.as_str(), *id)?;
                }
                InterpolatedStringSegment::Expr {
                    operand,
                    alignment,
                    format,
                    ..
                } => {
                    self.emit_interpolated_segment(
                        dest_ptr.as_str(),
                        operand,
                        *alignment,
                        *format,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn push_string_literal(&mut self, dest_ptr: &str, id: StrId) -> Result<(), Error> {
        let slice = self.emit_const_str(id)?;
        self.externals.insert("chic_rt_string_push_slice");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_string_push_slice(ptr {dest_ptr}, {LLVM_STR_TYPE} {})",
            slice.repr()
        )
        .ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::mir::{InterpolatedStringSegment, LocalId, Operand, Place, StrId, Ty};

    #[test]
    fn interpolates_text_and_expression_segments() {
        let lit_a = (
            StrId::new(21),
            StrLiteralInfo {
                global: "@__str21".into(),
                array_len: 4,
                data_len: 3,
            },
        );
        let lit_format = (
            StrId::new(22),
            StrLiteralInfo {
                global: "@__str22".into(),
                array_len: 3,
                data_len: 2,
            },
        );
        let (ir, externals) = with_emitter(vec![Ty::String], [lit_a, lit_format], |emitter, _| {
            let dest = Place {
                local: LocalId(0),
                projection: Vec::new(),
            };
            let segments = vec![
                InterpolatedStringSegment::Text { id: StrId::new(21) },
                InterpolatedStringSegment::Expr {
                    operand: Operand::Copy(Place {
                        local: LocalId(0),
                        projection: Vec::new(),
                    }),
                    alignment: Some(2),
                    format: Some(StrId::new(22)),
                    expr_text: "{value}".into(),
                    span: None,
                },
            ];

            emitter
                .emit_string_interpolate(&dest, &segments)
                .expect("interpolation should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_push_slice"));
        assert!(ir.contains("string_append_slice"));
        assert!(externals.contains("chic_rt_string_push_slice"));
    }
}
