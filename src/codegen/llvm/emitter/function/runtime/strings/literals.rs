use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;
use crate::error::Error;
use crate::mir::StrId;

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_const_str(&mut self, id: StrId) -> Result<ValueRef, Error> {
        let info = self.str_literals.get(&id).ok_or_else(|| {
            Error::Codegen(format!(
                "missing interned string segment for literal {}",
                id.index()
            ))
        })?;
        let base = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {base} = getelementptr inbounds [{len} x i8], ptr {global}, i32 0, i32 0",
            len = info.array_len,
            global = info.global
        )
        .ok();
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = insertvalue {LLVM_STR_TYPE} undef, ptr {base}, 0"
        )
        .ok();
        let tmp2 = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp2} = insertvalue {LLVM_STR_TYPE} {tmp}, i64 {len}, 1",
            len = info.data_len
        )
        .ok();
        Ok(ValueRef::new(tmp2, LLVM_STR_TYPE))
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::mir::StrId;

    #[test]
    fn emits_const_str_when_literal_present() {
        let literal = (
            StrId::new(3),
            StrLiteralInfo {
                global: "@__str3".into(),
                array_len: 5,
                data_len: 4,
            },
        );
        let (value, _) = with_emitter(Vec::new(), [literal], |emitter, _| {
            emitter
                .emit_const_str(StrId::new(3))
                .expect("literal should exist")
        });

        assert!(value.repr().starts_with("%"));
    }

    #[test]
    fn missing_literal_errors() {
        let (result, _) = with_emitter(Vec::new(), [], |emitter, _| {
            emitter.emit_const_str(StrId::new(99))
        });
        let err = result.expect_err("missing literal should error");
        assert!(err.to_string().contains("missing interned string segment"));
    }
}
