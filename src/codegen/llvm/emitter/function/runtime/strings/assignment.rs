use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::{LLVM_STR_TYPE, LLVM_STRING_TYPE};
use crate::error::Error;
use crate::mir::{BinOp, ConstValue, Operand, Place, Rvalue, pointer_align};

use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_string_assignment(
        &mut self,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Binary {
                op: bin_op,
                lhs,
                rhs,
                ..
            } if matches!(bin_op, BinOp::Add) => {
                self.emit_string_concat(place, lhs, rhs)?;
                Ok(true)
            }
            Rvalue::Use(operand) => match operand {
                Operand::Const(constant) => {
                    if matches!(constant.value, ConstValue::Str { .. }) {
                        self.emit_string_clone_from_operand(place, operand)?;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Operand::Copy(src_place) => {
                    if self.place_is_string(src_place)? {
                        self.emit_string_clone(place, src_place)?;
                        Ok(true)
                    } else if self.place_is_str(src_place)? {
                        self.emit_string_clone_from_operand(place, operand)?;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Operand::Move(src_place) => {
                    if self.place_is_str(src_place)? {
                        self.emit_string_clone_from_operand(place, operand)?;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                _ => Ok(false),
            },
            Rvalue::StringInterpolate { segments } => {
                self.emit_string_interpolate(place, segments)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn emit_string_concat(
        &mut self,
        dest: &Place,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<(), Error> {
        let dest_ptr = self.prepare_string_destination(dest)?;
        let lhs_slice = self.string_slice_operand(lhs)?;
        let rhs_slice = self.string_slice_operand(rhs)?;
        self.append_slice_to_string(dest_ptr.as_str(), &lhs_slice);
        self.append_slice_to_string(dest_ptr.as_str(), &rhs_slice);
        Ok(())
    }

    fn string_slice_operand(&mut self, operand: &Operand) -> Result<ValueRef, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) if self.place_is_string(place)? => {
                let ptr = self.string_place_ptr(place)?;
                self.externals.insert("chic_rt_string_as_slice");
                let tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = call {LLVM_STR_TYPE} @chic_rt_string_as_slice(ptr {ptr})"
                )
                .ok();
                Ok(ValueRef::new(tmp, LLVM_STR_TYPE))
            }
            _ => self.emit_operand(operand, Some(LLVM_STR_TYPE)),
        }
    }

    fn append_slice_to_string(&mut self, dest_ptr: &str, slice: &ValueRef) {
        self.externals.insert("chic_rt_string_append_slice");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_string_append_slice(ptr {dest_ptr}, {LLVM_STR_TYPE} {}, i32 0, i32 0, {LLVM_STR_TYPE} zeroinitializer)",
            slice.repr()
        )
        .ok();
    }

    pub(crate) fn prepare_string_destination(&mut self, place: &Place) -> Result<String, Error> {
        let ptr = self.string_place_ptr(place)?;
        writeln!(
            &mut self.builder,
            "  store {LLVM_STRING_TYPE} zeroinitializer, ptr {ptr}"
        )
        .ok();
        Ok(ptr)
    }

    pub(crate) fn emit_string_clone(&mut self, dest: &Place, src: &Place) -> Result<(), Error> {
        let dest_ptr = self.prepare_string_destination(dest)?;
        let src_ptr = self.string_place_ptr(src)?;
        self.externals.insert("chic_rt_string_clone");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_string_clone(ptr {dest_ptr}, ptr {src_ptr})"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_string_clone_from_operand(
        &mut self,
        dest: &Place,
        operand: &Operand,
    ) -> Result<(), Error> {
        let dest_ptr = self.prepare_string_destination(dest)?;
        let slice = self.emit_operand(operand, Some(LLVM_STR_TYPE))?;
        self.externals.insert("chic_rt_string_clone_slice");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_string_clone_slice(ptr {dest_ptr}, {LLVM_STR_TYPE} {})",
            slice.repr()
        )
        .ok();
        Ok(())
    }

    fn string_place_ptr(&mut self, place: &Place) -> Result<String, Error> {
        let ptr = self.place_ptr(place)?;
        if self.is_reference_param(place.local.0) && place.projection.is_empty() {
            let tmp = self.new_temp();
            let align = pointer_align();
            writeln!(&mut self.builder, "  {tmp} = load ptr, ptr {ptr}, align {align}").ok();
            return Ok(tmp);
        }
        Ok(ptr)
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::llvm::emitter::function::runtime::strings::test_support::with_emitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::mir::{
        ConstOperand, ConstValue, InterpolatedStringSegment, LocalId, Operand, Place, Rvalue,
        StrId, Ty,
    };

    #[test]
    fn clones_from_str_constant() {
        let literal = (
            StrId::new(1),
            StrLiteralInfo {
                global: "@__str1".into(),
                array_len: 6,
                data_len: 5,
            },
        );
        let ((assigned, ir), externals) =
            with_emitter(vec![Ty::String], [literal], |emitter, _| {
                let dest = Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                };
                let operand = Operand::Const(ConstOperand::new(ConstValue::Str {
                    id: StrId::new(1),
                    value: "hello".into(),
                }));
                let assigned = emitter
                    .emit_string_assignment(&dest, &Rvalue::Use(operand))
                    .expect("assignment should succeed");

                (assigned, emitter.ir().to_string())
            });

        assert!(assigned);
        assert!(ir.contains("string_clone_slice"));
        assert!(externals.contains("chic_rt_string_clone_slice"));
    }

    #[test]
    fn copies_owned_string_place() {
        let ((assigned, ir), externals) =
            with_emitter(vec![Ty::String, Ty::String], [], |emitter, _| {
                let dest = Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                };
                let src = Place {
                    local: LocalId(1),
                    projection: Vec::new(),
                };
                let assigned = emitter
                    .emit_string_assignment(&dest, &Rvalue::Use(Operand::Copy(src)))
                    .expect("copy should succeed");

                (assigned, emitter.ir().to_string())
            });

        assert!(assigned);
        assert!(ir.contains("string_clone"));
        assert!(externals.contains("chic_rt_string_clone"));
    }

    #[test]
    fn copies_str_slice_place() {
        let ((assigned, ir), externals) =
            with_emitter(vec![Ty::String, Ty::Str], [], |emitter, _| {
                let dest = Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                };
                let src = Place {
                    local: LocalId(1),
                    projection: Vec::new(),
                };
                let assigned = emitter
                    .emit_string_assignment(&dest, &Rvalue::Use(Operand::Copy(src)))
                    .expect("str copy should succeed");

                (assigned, emitter.ir().to_string())
            });

        assert!(assigned);
        assert!(ir.contains("string_clone_slice"));
        assert!(externals.contains("chic_rt_string_clone_slice"));
    }

    #[test]
    fn moves_str_slice_place() {
        let ((assigned, ir), externals) =
            with_emitter(vec![Ty::String, Ty::Str], [], |emitter, _| {
                let dest = Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                };
                let src = Place {
                    local: LocalId(1),
                    projection: Vec::new(),
                };
                let assigned = emitter
                    .emit_string_assignment(&dest, &Rvalue::Use(Operand::Move(src)))
                    .expect("str move should succeed");

                (assigned, emitter.ir().to_string())
            });

        assert!(assigned);
        assert!(ir.contains("string_clone_slice"));
        assert!(externals.contains("chic_rt_string_clone_slice"));
    }

    #[test]
    fn non_string_operand_returns_false() {
        let ((assigned, ir), _) = with_emitter(vec![Ty::String], [], |emitter, _| {
            let dest = Place {
                local: LocalId(0),
                projection: Vec::new(),
            };
            let assigned = emitter
                .emit_string_assignment(
                    &dest,
                    &Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
                )
                .expect("should not error");

            (assigned, emitter.ir().to_string())
        });

        assert!(!assigned);
        assert!(ir.is_empty());
    }

    #[test]
    fn delegates_to_interpolation() {
        let text_literal = (
            StrId::new(2),
            StrLiteralInfo {
                global: "@__str2".into(),
                array_len: 4,
                data_len: 3,
            },
        );
        let ((assigned, ir), externals) =
            with_emitter(vec![Ty::String], [text_literal], |emitter, _| {
                let dest = Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                };
                let segments = vec![InterpolatedStringSegment::Text { id: StrId::new(2) }];

                let assigned = emitter
                    .emit_string_assignment(&dest, &Rvalue::StringInterpolate { segments })
                    .expect("interpolation should succeed");

                (assigned, emitter.ir().to_string())
            });

        assert!(assigned);
        assert!(ir.contains("string_push_slice"));
        assert!(externals.contains("chic_rt_string_push_slice"));
    }
}
