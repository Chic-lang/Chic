use std::fmt::Write;

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
use crate::error::Error;
use crate::mir::Operand;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn append_str_segment(
        &mut self,
        dest_ptr: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let slice = self.emit_operand(operand, Some(LLVM_STR_TYPE))?;
        self.append_slice_operand(dest_ptr, &slice, alignment, has_alignment, format);
        Ok(())
    }

    pub(super) fn append_owned_string_segment(
        &mut self,
        dest_ptr: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let slice = self.materialise_string_slice(operand)?;
        self.append_slice_operand(dest_ptr, &slice, alignment, has_alignment, format);
        Ok(())
    }

    pub(super) fn string_operand_ptr(&mut self, operand: &Operand) -> Result<String, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if !place.projection.is_empty() {
                    return Err(Error::Codegen(
                        "string interpolation does not yet support projected string values".into(),
                    ));
                }
                self.place_ptr(place)
            }
            _ => Err(Error::Codegen(
                "string interpolation requires addressing a string place".into(),
            )),
        }
    }

    fn append_slice_operand(
        &mut self,
        dest_ptr: &str,
        slice: &ValueRef,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) {
        self.externals.insert("chic_rt_string_append_slice");
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_slice(ptr {dest_ptr}, {LLVM_STR_TYPE} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                slice.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_slice(ptr {dest_ptr}, {LLVM_STR_TYPE} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                slice.repr()
            )
            .ok();
        }
    }

    fn materialise_string_slice(&mut self, operand: &Operand) -> Result<ValueRef, Error> {
        let src_ptr = self.string_operand_ptr(operand)?;
        self.externals.insert("chic_rt_string_as_slice");
        let slice_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {slice_tmp} = call {LLVM_STR_TYPE} @chic_rt_string_as_slice(ptr {src_ptr})"
        )
        .ok();
        Ok(ValueRef::new(slice_tmp, LLVM_STR_TYPE))
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::function::values::ValueRef;
    use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::mir::{
        ConstOperand, ConstValue, LocalId, Operand, Place, ProjectionElem, StrId, Ty,
    };

    #[test]
    fn appends_str_segment_with_format_alignment() {
        let literal = (
            StrId::new(5),
            StrLiteralInfo {
                global: "@__str5".into(),
                array_len: 4,
                data_len: 3,
            },
        );
        let format_literal = (
            StrId::new(6),
            StrLiteralInfo {
                global: "@__str6".into(),
                array_len: 3,
                data_len: 2,
            },
        );
        let (ir, externals) = with_emitter(Vec::new(), [literal, format_literal], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Str {
                id: StrId::new(5),
                value: "hey".into(),
            }));
            let format = emitter.emit_const_str(StrId::new(6)).unwrap();

            emitter
                .append_str_segment("%dest", &operand, 4, true, Some(&format))
                .expect("append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_append_slice"));
        assert!(ir.contains("i32 4"));
        assert!(externals.contains("chic_rt_string_append_slice"));
    }

    #[test]
    fn appends_owned_string_segment_and_materialises_slice() {
        let (ir, externals) = with_emitter(vec![Ty::String], [], |emitter, _| {
            let operand = Operand::Copy(Place {
                local: LocalId(0),
                projection: Vec::new(),
            });

            emitter
                .append_owned_string_segment("%dest", &operand, 0, false, None)
                .expect("append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_as_slice"));
        assert!(ir.contains("string_append_slice"));
        assert!(externals.contains("chic_rt_string_as_slice"));
    }

    #[test]
    fn string_operand_ptr_rejects_projection() {
        let (err, _) = with_emitter(vec![Ty::String], [], |emitter, _| {
            let operand = Operand::Copy(Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Field(0)],
            });

            emitter.string_operand_ptr(&operand)
        });
        let err = err.expect_err("projection should be rejected");
        assert!(
            err.to_string()
                .contains("does not yet support projected string values")
        );
    }

    #[test]
    fn append_slice_operand_handles_alignment_flag() {
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let slice =
                ValueRef::new_literal(format!("{LLVM_STR_TYPE} zeroinitializer"), LLVM_STR_TYPE);

            emitter.append_slice_operand("%dest", &slice, 0, false, None);

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_append_slice"));
        assert!(ir.contains("i32 0"));
        assert!(ir.contains("zeroinitializer"));
        assert!(externals.contains("chic_rt_string_append_slice"));
    }
}
