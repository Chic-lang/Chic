use std::fmt::Write;

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
use crate::error::Error;
use crate::mir::Operand;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn append_bool_segment(
        &mut self,
        dest_ptr: &str,
        llvm_ty: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let value = self.emit_operand(operand, Some(llvm_ty))?;
        self.externals.insert("chic_rt_string_append_bool");
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_bool(ptr {dest_ptr}, {llvm_ty} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                value.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_bool(ptr {dest_ptr}, {llvm_ty} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                value.repr()
            )
            .ok();
        }
        Ok(())
    }

    pub(super) fn append_char_segment(
        &mut self,
        dest_ptr: &str,
        llvm_ty: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let value = self.emit_operand(operand, Some(llvm_ty))?;
        let repr = if llvm_ty != "i16" {
            let tmp = self.new_temp();
            let op = match self.parse_integer_bits(&llvm_ty) {
                Some(bits) if bits > 16 => "trunc",
                _ => "zext",
            };
            writeln!(
                &mut self.builder,
                "  {tmp} = {op} {llvm_ty} {} to i16",
                value.repr()
            )
            .ok();
            ValueRef::new(tmp, "i16")
        } else {
            value
        };

        self.externals.insert("chic_rt_string_append_char");
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_char(ptr {dest_ptr}, i16 {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                repr.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_char(ptr {dest_ptr}, i16 {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                repr.repr()
            )
            .ok();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::function::values::ValueRef;
    use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
    use crate::mir::{ConstOperand, ConstValue, Operand};

    #[test]
    fn appends_bool_with_format() {
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
            let format =
                ValueRef::new_literal(format!("{LLVM_STR_TYPE} zeroinitializer"), LLVM_STR_TYPE);

            emitter
                .append_bool_segment("%dest", "i8", &operand, 2, true, Some(&format))
                .expect("bool append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_append_bool"));
        assert!(ir.contains("i32 2"));
        assert!(externals.contains("chic_rt_string_append_bool"));
    }

    #[test]
    fn appends_char_zero_extends_when_needed() {
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Char('A' as u16)));

            emitter
                .append_char_segment("%dest", "i8", &operand, 0, false, None)
                .expect("char append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("zext i8"));
        assert!(ir.contains("string_append_char"));
        assert!(externals.contains("chic_rt_string_append_char"));
    }
}
