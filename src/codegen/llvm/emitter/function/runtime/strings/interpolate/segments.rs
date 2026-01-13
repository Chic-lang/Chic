use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::runtime::interpolation::InterpolationOperandKind;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::error::Error;
use crate::mir::{Operand, StrId};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_interpolated_segment(
        &mut self,
        dest_ptr: &str,
        operand: &Operand,
        alignment: Option<i32>,
        format: Option<StrId>,
    ) -> Result<(), Error> {
        let kind = self.classify_interpolation_operand(operand)?;
        let alignment_value = alignment.unwrap_or(0);
        let has_alignment = alignment.is_some();
        let format_value = self.optional_format_literal(format)?;

        match kind {
            InterpolationOperandKind::Str => {
                self.append_str_segment(
                    dest_ptr,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::String => {
                self.append_owned_string_segment(
                    dest_ptr,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::Bool { llvm_ty } => {
                self.append_bool_segment(
                    dest_ptr,
                    &llvm_ty,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::Char { llvm_ty } => {
                self.append_char_segment(
                    dest_ptr,
                    &llvm_ty,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::SignedInt { bits, llvm_ty } => {
                self.append_signed_integer_segment(
                    dest_ptr,
                    bits,
                    &llvm_ty,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::UnsignedInt { bits, llvm_ty } => {
                self.append_unsigned_integer_segment(
                    dest_ptr,
                    bits,
                    &llvm_ty,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
            InterpolationOperandKind::Float { bits, llvm_ty } => {
                self.append_float_segment(
                    dest_ptr,
                    bits,
                    &llvm_ty,
                    operand,
                    alignment_value,
                    has_alignment,
                    format_value.as_ref(),
                )?;
            }
        }

        Ok(())
    }

    fn optional_format_literal(
        &mut self,
        format: Option<StrId>,
    ) -> Result<Option<ValueRef>, Error> {
        format.map(|id| self.emit_const_str(id)).transpose()
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::mir::FloatValue;
    use crate::mir::{ConstOperand, ConstValue, LocalId, Operand, Place, StrId, Ty};

    fn literals() -> [(StrId, StrLiteralInfo); 2] {
        [
            (
                StrId::new(10),
                StrLiteralInfo {
                    global: "@__str10".into(),
                    array_len: 4,
                    data_len: 3,
                },
            ),
            (
                StrId::new(11),
                StrLiteralInfo {
                    global: "@__str11".into(),
                    array_len: 3,
                    data_len: 2,
                },
            ),
        ]
    }

    #[test]
    fn dispatches_all_operand_kinds() {
        let (ir, externals) = with_emitter(vec![Ty::String], literals(), |emitter, _| {
            // Str literal branch
            let str_operand = Operand::Const(ConstOperand::new(ConstValue::Str {
                id: StrId::new(10),
                value: "hey".into(),
            }));
            emitter
                .emit_interpolated_segment("%dest", &str_operand, Some(2), Some(StrId::new(11)))
                .expect("str segment should succeed");

            // String branch
            let string_operand = Operand::Copy(Place {
                local: LocalId(0),
                projection: Vec::new(),
            });
            emitter
                .emit_interpolated_segment("%dest", &string_operand, None, None)
                .expect("string segment should succeed");

            // Bool branch
            let bool_operand = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
            emitter
                .emit_interpolated_segment("%dest", &bool_operand, None, None)
                .expect("bool segment should succeed");

            // Char branch
            let char_operand = Operand::Const(ConstOperand::new(ConstValue::Char('A' as u16)));
            emitter
                .emit_interpolated_segment("%dest", &char_operand, None, None)
                .expect("char segment should succeed");

            // Signed integer branch
            let signed_operand = Operand::Const(ConstOperand::new(ConstValue::Int(7)));
            emitter
                .emit_interpolated_segment("%dest", &signed_operand, Some(0), None)
                .expect("signed segment should succeed");

            // Unsigned integer branch
            let unsigned_operand = Operand::Const(ConstOperand::new(ConstValue::UInt(7)));
            emitter
                .emit_interpolated_segment("%dest", &unsigned_operand, None, None)
                .expect("unsigned segment should succeed");

            // Float branch
            let float_operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f64(1.5),
            )));
            emitter
                .emit_interpolated_segment("%dest", &float_operand, Some(4), Some(StrId::new(11)))
                .expect("float segment should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_append_slice"));
        assert!(ir.contains("string_append_signed"));
        assert!(ir.contains("string_append_unsigned"));
        assert!(ir.contains("string_append_f64"));
        assert!(externals.contains("chic_rt_string_append_slice"));
    }
}
