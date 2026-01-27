use std::fmt::Write;

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
use crate::error::Error;
use crate::mir::Operand;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn append_signed_integer_segment(
        &mut self,
        dest_ptr: &str,
        bits: u32,
        llvm_ty: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let value = self.emit_operand(operand, Some(llvm_ty))?;
        let repr = if llvm_ty != "i128" {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = sext {llvm_ty} {} to i128",
                value.repr()
            )
            .ok();
            ValueRef::new(tmp, "i128")
        } else {
            value
        };
        self.externals.insert("chic_rt_string_append_signed");
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_signed(ptr {dest_ptr}, i128 {}, i32 {bits}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                repr.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_signed(ptr {dest_ptr}, i128 {}, i32 {bits}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                repr.repr()
            )
            .ok();
        }
        Ok(())
    }

    pub(super) fn append_unsigned_integer_segment(
        &mut self,
        dest_ptr: &str,
        bits: u32,
        llvm_ty: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let value = self.emit_operand(operand, Some(llvm_ty))?;
        let repr = if llvm_ty != "i128" {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = zext {llvm_ty} {} to i128",
                value.repr()
            )
            .ok();
            ValueRef::new(tmp, "i128")
        } else {
            value
        };
        self.externals.insert("chic_rt_string_append_unsigned");
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_unsigned(ptr {dest_ptr}, i128 {}, i32 {bits}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                repr.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @chic_rt_string_append_unsigned(ptr {dest_ptr}, i128 {}, i32 {bits}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                repr.repr()
            )
            .ok();
        }
        Ok(())
    }

    pub(super) fn append_float_segment(
        &mut self,
        dest_ptr: &str,
        bits: u32,
        llvm_ty: &str,
        operand: &Operand,
        alignment: i32,
        has_alignment: bool,
        format: Option<&ValueRef>,
    ) -> Result<(), Error> {
        let value = self.emit_operand(operand, Some(llvm_ty))?;
        let (extern_name, arg, arg_ty) = match bits {
            16 => {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = bitcast {llvm_ty} {} to i16",
                    value.repr()
                )
                .ok();
                (
                    "chic_rt_string_append_f16",
                    ValueRef::new(tmp, "i16"),
                    "i16",
                )
            }
            32 => ("chic_rt_string_append_f32", value, llvm_ty),
            64 => ("chic_rt_string_append_f64", value, llvm_ty),
            128 => {
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = bitcast {llvm_ty} {} to i128",
                    value.repr()
                )
                .ok();
                (
                    "chic_rt_string_append_f128",
                    ValueRef::new(tmp, "i128"),
                    "i128",
                )
            }
            other => {
                return Err(Error::Codegen(format!(
                    "unsupported float interpolation width {other}"
                )));
            }
        };
        self.externals.insert(extern_name);
        let align_flag = if has_alignment { 1 } else { 0 };
        if let Some(format_ref) = format {
            writeln!(
                &mut self.builder,
                "  call i32 @{extern_name}(ptr {dest_ptr}, {arg_ty} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} {})",
                arg.repr(),
                format_ref.repr()
            )
            .ok();
        } else {
            writeln!(
                &mut self.builder,
                "  call i32 @{extern_name}(ptr {dest_ptr}, {arg_ty} {}, i32 {alignment}, i32 {align_flag}, {LLVM_STR_TYPE} zeroinitializer)",
                arg.repr()
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
    use crate::mir::FloatValue;
    use crate::mir::{ConstOperand, ConstValue, Operand};

    #[test]
    fn appends_signed_integer_with_sext() {
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int(7)));

            emitter
                .append_signed_integer_segment("%dest", 32, "i32", &operand, 1, true, None)
                .expect("signed append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("sext i32"));
        assert!(ir.contains("string_append_signed"));
        assert!(externals.contains("chic_rt_string_append_signed"));
    }

    #[test]
    fn appends_unsigned_integer_without_format() {
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::UInt(9)));

            emitter
                .append_unsigned_integer_segment("%dest", 64, "i64", &operand, 0, false, None)
                .expect("unsigned append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("zext i64"));
        assert!(ir.contains("string_append_unsigned"));
        assert!(externals.contains("chic_rt_string_append_unsigned"));
    }

    #[test]
    fn appends_integer_without_extension_when_128() {
        let ir = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int(1)));

            emitter
                .append_signed_integer_segment("%dest", 128, "i128", &operand, 0, false, None)
                .expect("i128 append should succeed");

            emitter.ir().to_string()
        })
        .0;

        assert!(!ir.contains("sext i128"));
    }

    #[test]
    fn appends_float_segments_for_all_widths() {
        let format =
            ValueRef::new_literal(format!("{LLVM_STR_TYPE} zeroinitializer"), LLVM_STR_TYPE);
        let (ir, externals) = with_emitter(Vec::new(), [], |emitter, _| {
            let f16_operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f16(1.5),
            )));
            emitter
                .append_float_segment("%dest", 16, "half", &f16_operand, 0, false, Some(&format))
                .expect("f16 append should succeed");
            let f32_operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f32(1.5),
            )));
            emitter
                .append_float_segment("%dest", 32, "float", &f32_operand, 0, false, Some(&format))
                .expect("f32 append should succeed");
            let f64_operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f64(2.5),
            )));
            emitter
                .append_float_segment("%dest", 64, "double", &f64_operand, 4, true, None)
                .expect("f64 append should succeed");
            let f128_operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f64_as(3.75, crate::mir::FloatWidth::F128),
            )));
            emitter
                .append_float_segment("%dest", 128, "fp128", &f128_operand, 0, false, None)
                .expect("f128 append should succeed");

            emitter.ir().to_string()
        });

        assert!(ir.contains("string_append_f16"));
        assert!(ir.contains("bitcast half"));
        assert!(ir.contains("string_append_f32"));
        assert!(ir.contains("string_append_f64"));
        assert!(ir.contains("string_append_f128"));
        assert!(externals.contains("chic_rt_string_append_f32"));
        assert!(externals.contains("chic_rt_string_append_f64"));
        assert!(externals.contains("chic_rt_string_append_f16"));
        assert!(externals.contains("chic_rt_string_append_f128"));
    }

    #[test]
    fn rejects_unknown_float_width() {
        let (err, _) = with_emitter(Vec::new(), [], |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f64(3.0),
            )));
            emitter.append_float_segment("%dest", 80, "x86_fp80", &operand, 0, false, None)
        });
        let err = err.expect_err("unsupported width should error");
        assert!(
            err.to_string()
                .contains("unsupported float interpolation width")
        );
    }
}
