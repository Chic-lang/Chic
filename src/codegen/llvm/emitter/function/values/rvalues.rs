use std::fmt::Write;

use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::{
    AtomicFenceScope, AtomicOrdering, AtomicRmwOp, BinOp, NumericIntrinsicKind, NumericWidth,
    Operand, PendingRvalue, Place, ProjectionElem, Rvalue, Ty, UnOp, pointer_align,
};

use super::super::builder::FunctionEmitter;
use super::value_ref::ValueRef;

const DECIMAL_INTRINSIC_RESULT_TY: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";
const LLVM_SPAN_VALUE_TY: &str = "{ { i8*, i64, i64 }, i64, i64, i64 }";

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_rvalue(
        &mut self,
        value: &Rvalue,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        match value {
            Rvalue::Use(op) => self.emit_operand(op, expected),
            Rvalue::Unary {
                op,
                operand,
                rounding,
            } => self.with_rounding(*rounding, |emitter| {
                emitter.emit_unary(*op, operand, expected)
            }),
            Rvalue::Binary {
                op,
                lhs,
                rhs,
                rounding,
            } => self.with_rounding(*rounding, |emitter| {
                emitter.emit_binary(*op, lhs, rhs, expected)
            }),
            Rvalue::Cast {
                kind,
                operand,
                source,
                target,
                rounding,
            } => self.with_rounding(*rounding, |emitter| {
                emitter.emit_cast(*kind, operand, source, target, expected)
            }),
            Rvalue::Pending(PendingRvalue { repr, .. }) => {
                let ty = expected.unwrap_or("i32");
                if std::env::var("CHIC_DEBUG_PENDING").is_ok() {
                    eprintln!(
                        "[pending-rvalue] repr=`{repr}` expected_ty={ty} func={}",
                        self.function.name
                    );
                }
                let literal = if ty == "ptr" {
                    "null".to_string()
                } else {
                    "0".to_string()
                };
                Ok(ValueRef::new_literal(literal, ty))
            }
            Rvalue::StringInterpolate { .. } => Err(Error::Codegen(
                "string interpolation rvalues are lowered directly via string assignments"
                    .to_string(),
            )),
            Rvalue::DecimalIntrinsic(decimal) => {
                let result = self.emit_decimal_intrinsic_value(decimal)?;
                Ok(result.into_value_ref())
            }
            Rvalue::Len(place) => self.emit_len(place),
            Rvalue::SpanStackAlloc {
                element,
                length,
                source,
            } => self.emit_span_stack_alloc_value(element, length, source.as_ref()),
            Rvalue::AtomicLoad { target, order } => {
                let ty = self.place_type(target)?.ok_or_else(|| {
                    Error::Codegen("atomic load requires addressable type".into())
                })?;
                Self::ensure_atomic_ty(&ty)?;
                let ptr = self.place_ptr(target)?;
                let mir_ty = self.mir_ty_of_place(target)?;
                let align = self.atomic_alignment_for_ty(&mir_ty);
                let ordering = Self::llvm_atomic_ordering(*order);
                let tmp = self.new_temp();
                let alias_suffix = self.alias_suffix_for_place(target).unwrap_or_default();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = load atomic {ty}, ptr {ptr} {ordering}, align {align}{alias_suffix}"
                )
                .ok();
                Ok(ValueRef::new(tmp, &ty))
            }
            Rvalue::AtomicRmw {
                op,
                target,
                value,
                order,
            } => {
                let ty = self
                    .place_type(target)?
                    .ok_or_else(|| Error::Codegen("atomic RMW requires addressable type".into()))?;
                Self::ensure_atomic_ty(&ty)?;
                let operand = self.emit_operand(value, Some(&ty))?;
                let ptr = self.place_ptr(target)?;
                let op_str = Self::llvm_atomic_rmw_op(*op)?;
                let ordering = Self::llvm_atomic_ordering(*order);
                let tmp = self.new_temp();
                let alias_suffix = self.alias_suffix_for_place(target).unwrap_or_default();
                writeln!(
                    &mut self.builder,
                    "  {tmp} = atomicrmw {op_str} ptr {ptr}, {ty} {}, {ordering}{alias_suffix}",
                    operand.repr(),
                )
                .ok();
                Ok(ValueRef::new(tmp, &ty))
            }
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                success,
                failure,
                weak,
            } => {
                let ty = self.place_type(target)?.ok_or_else(|| {
                    Error::Codegen("atomic compare-exchange requires addressable type".into())
                })?;
                Self::ensure_atomic_ty(&ty)?;
                let ptr = self.place_ptr(target)?;
                let expected_val = self.emit_operand(expected, Some(&ty))?;
                let desired_val = self.emit_operand(desired, Some(&ty))?;
                let weak_kw = if *weak { "weak " } else { "" };
                let success_str = Self::llvm_atomic_ordering(*success);
                let failure_str = Self::llvm_atomic_ordering(*failure);
                let pair_tmp = self.new_temp();
                let alias_suffix = self.alias_suffix_for_place(target).unwrap_or_default();
                writeln!(
                    &mut self.builder,
                    "  {pair_tmp} = cmpxchg {weak_kw}ptr {ptr}, {ty} {}, {ty} {}, {success_str}, {failure_str}{alias_suffix}",
                    expected_val.repr(),
                    desired_val.repr(),
                )
                .ok();
                let success_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {success_tmp} = extractvalue {{ {ty}, i1 }} {pair_tmp}, 1"
                )
                .ok();
                let bool_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {bool_tmp} = zext i1 {success_tmp} to i8"
                )
                .ok();
                Ok(ValueRef::new(bool_tmp, "i8"))
            }
            Rvalue::StaticLoad { id } => {
                let llvm_ty = self.static_llvm_type(*id)?;
                let global = self.static_symbol(*id)?;
                let tmp = self.new_temp();
                writeln!(&mut self.builder, "  {tmp} = load {llvm_ty}, ptr {global}").ok();
                Ok(ValueRef::new(tmp, &llvm_ty))
            }
            Rvalue::StaticRef { id } => {
                let _ = self.static_llvm_type(*id)?;
                let global = self.static_symbol(*id)?;
                let ty = expected.unwrap_or("ptr");
                if ty == "ptr" {
                    Ok(ValueRef::new(global, ty))
                } else {
                    let tmp = self.new_temp();
                    writeln!(&mut self.builder, "  {tmp} = bitcast ptr {global} to {ty}").ok();
                    Ok(ValueRef::new(tmp, ty))
                }
            }
            Rvalue::AddressOf { place, .. } => {
                let ptr = self.place_ptr(place)?;
                let ty = expected.unwrap_or("ptr");
                if ty == "ptr" {
                    Ok(ValueRef::new(ptr, ty))
                } else {
                    let tmp = self.new_temp();
                    writeln!(&mut self.builder, "  {tmp} = bitcast ptr {ptr} to {ty}").ok();
                    Ok(ValueRef::new(tmp, ty))
                }
            }
            _ => Err(Error::Codegen(format!(
                "rvalue variant {value:?} not yet supported in LLVM backend"
            ))),
        }
    }

    pub(crate) fn emit_len(&mut self, place: &Place) -> Result<ValueRef, Error> {
        let mut seq_ty = self.mir_ty_of_place(place)?;
        while let Ty::Nullable(inner) = seq_ty {
            seq_ty = *inner;
        }

        let field_name = match seq_ty {
            Ty::Vec(_)
            | Ty::Array(_)
            | Ty::Span(_)
            | Ty::ReadOnlySpan(_)
            | Ty::String
            | Ty::Str => "len",
            _ => {
                return Err(Error::Codegen(
                    "length operator is only supported on sequence types in the LLVM backend"
                        .into(),
                ));
            }
        };

        let mut len_place = place.clone();
        len_place
            .projection
            .push(ProjectionElem::FieldNamed(field_name.to_string()));
        let field_ptr = self.place_ptr(&len_place)?;
        let field_ty = self.mir_ty_of_place(&len_place)?;
        let llvm_ty = map_type_owned(&field_ty, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "length field type `{}` is not supported in LLVM backend",
                field_ty.canonical_name()
            ))
        })?;
        let tmp = self.new_temp();
        let alias_suffix = self.alias_suffix_for_place(&len_place).unwrap_or_default();
        writeln!(
            &mut self.builder,
            "  {tmp} = load {llvm_ty}, ptr {field_ptr}{alias_suffix}"
        )
        .ok();
        Ok(ValueRef::new(tmp, &llvm_ty))
    }

    pub(crate) fn atomic_alignment_for_ty(&self, ty: &Ty) -> usize {
        self.type_layouts
            .size_and_align_for_ty(ty)
            .map(|(_, align)| align)
            .unwrap_or(pointer_align())
    }

    pub(crate) fn llvm_atomic_ordering(order: AtomicOrdering) -> &'static str {
        match order {
            AtomicOrdering::Relaxed => "monotonic",
            AtomicOrdering::Acquire => "acquire",
            AtomicOrdering::Release => "release",
            AtomicOrdering::AcqRel => "acq_rel",
            AtomicOrdering::SeqCst => "seq_cst",
        }
    }

    pub(crate) fn llvm_fence_order(scope: AtomicFenceScope, order: AtomicOrdering) -> &'static str {
        match scope {
            AtomicFenceScope::Full => Self::llvm_atomic_ordering(order),
            AtomicFenceScope::BlockEnter => match order {
                AtomicOrdering::Relaxed => "monotonic",
                AtomicOrdering::Release => "acquire",
                _ => Self::llvm_atomic_ordering(order),
            },
            AtomicFenceScope::BlockExit => match order {
                AtomicOrdering::Relaxed => "monotonic",
                AtomicOrdering::Acquire => "release",
                _ => Self::llvm_atomic_ordering(order),
            },
        }
    }

    fn emit_span_stack_alloc_value(
        &mut self,
        element: &Ty,
        length: &Operand,
        source: Option<&Operand>,
    ) -> Result<ValueRef, Error> {
        let (elem_size, elem_align) = self
            .type_layouts
            .size_and_align_for_ty(element)
            .unwrap_or((0, 1));

        let len_value = self.emit_operand(length, Some("i64"))?;

        let raw_ptr = if elem_size == 0 {
            let ptr_tmp = self.new_temp();
            writeln!(&mut self.builder, "  {ptr_tmp} = inttoptr i64 1 to ptr").ok();
            ptr_tmp
        } else {
            let total_bytes = if elem_size == 1 {
                len_value.repr().to_string()
            } else {
                let mul_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {mul_tmp} = mul i64 {}, {elem_size}",
                    len_value.repr()
                )
                .ok();
                mul_tmp
            };
            let alloca_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {alloca_tmp} = alloca i8, i64 {total_bytes}, align {elem_align}"
            )
            .ok();
            alloca_tmp
        };

        let value_ptr_ty = self.value_mut_ptr_ty()?;
        let ptr_insert = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {ptr_insert} = insertvalue {value_ptr_ty} undef, ptr {raw_ptr}, 0"
        )
        .ok();
        let size_insert = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {size_insert} = insertvalue {value_ptr_ty} {ptr_insert}, i64 {elem_size}, 1"
        )
        .ok();
        let align_insert = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {align_insert} = insertvalue {value_ptr_ty} {size_insert}, i64 {elem_align}, 2"
        )
        .ok();

        self.externals.insert("chic_rt_span_from_raw_mut");
        let result = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {result} = call {LLVM_SPAN_VALUE_TY} @chic_rt_span_from_raw_mut({value_ptr_ty} {align_insert}, i64 {})",
            len_value.repr()
        )
        .ok();
        if let Some(source) = source {
            let source_value = match source {
                Operand::Borrow(borrow) => {
                    let copy = Operand::Copy(borrow.place.clone());
                    self.emit_operand(&copy, Some(LLVM_SPAN_VALUE_TY))?
                }
                _ => self.emit_operand(source, Some(LLVM_SPAN_VALUE_TY))?,
            };
            self.externals.insert("chic_rt_span_copy_to");
            let status_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {status_tmp} = call i32 @chic_rt_span_copy_to({LLVM_SPAN_VALUE_TY} {}, {LLVM_SPAN_VALUE_TY} {result})",
                source_value.repr()
            )
            .ok();
        }
        Ok(ValueRef::new(result, LLVM_SPAN_VALUE_TY))
    }

    fn value_mut_ptr_ty(&self) -> Result<String, Error> {
        let candidates = [
            Ty::named("Std::Runtime::Collections::ValueMutPtr"),
            Ty::named("Std::Runtime::Native::ValueMutPtr"),
        ];
        for candidate in candidates {
            if let Some(mapped) = map_type_owned(&candidate, Some(self.type_layouts))? {
                return Ok(mapped);
            }
        }
        Err(Error::Codegen(
            "ValueMutPtr type missing LLVM mapping".into(),
        ))
    }

    fn llvm_atomic_rmw_op(op: AtomicRmwOp) -> Result<&'static str, Error> {
        match op {
            AtomicRmwOp::Add => Ok("add"),
            AtomicRmwOp::Sub => Ok("sub"),
            AtomicRmwOp::And => Ok("and"),
            AtomicRmwOp::Or => Ok("or"),
            AtomicRmwOp::Xor => Ok("xor"),
            AtomicRmwOp::Exchange => Ok("xchg"),
            AtomicRmwOp::Min => Ok("min"),
            AtomicRmwOp::Max => Ok("max"),
        }
    }

    pub(crate) fn ensure_atomic_ty(ty: &str) -> Result<(), Error> {
        if matches!(ty, "i8" | "i16" | "i32" | "i64" | "ptr") {
            Ok(())
        } else {
            Err(Error::Codegen(format!(
                "atomic operations require integer-like type, found `{ty}`"
            )))
        }
    }

    pub(crate) fn infer_rvalue_type(
        &self,
        value: &Rvalue,
        locals: &[Option<String>],
    ) -> Result<Option<String>, Error> {
        match value {
            Rvalue::Use(op) => self.operand_type_hint(op, locals),
            Rvalue::Unary { op, operand, .. } => match op {
                UnOp::Neg | UnOp::UnaryPlus | UnOp::Increment | UnOp::Decrement => {
                    if let Some(ty) = self.operand_type_hint(operand, locals)? {
                        Ok(Some(ty))
                    } else {
                        Ok(Some("i32".into()))
                    }
                }
                UnOp::Not | UnOp::BitNot => {
                    let hint = self.operand_type_hint(operand, locals)?;
                    if let Some(ty) = hint {
                        if ty == "i8" {
                            Ok(Some("i8".into()))
                        } else {
                            Ok(Some(ty))
                        }
                    } else {
                        Ok(Some("i8".into()))
                    }
                }
                UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut => Ok(None),
            },
            Rvalue::Binary { op, lhs, rhs, .. } => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Ok(Some("i8".into())),
                _ => {
                    if let Some(ty) = self.operand_type_hint(lhs, locals)? {
                        return Ok(Some(ty));
                    }
                    if let Some(ty) = self.operand_type_hint(rhs, locals)? {
                        return Ok(Some(ty));
                    }
                    Ok(Some("i32".into()))
                }
            },
            Rvalue::Cast { target, .. } => map_type_owned(target, Some(self.type_layouts)),
            Rvalue::Len(_) => Ok(Some("i64".into())),
            Rvalue::SpanStackAlloc { .. } => Ok(Some(LLVM_SPAN_VALUE_TY.to_string())),
            Rvalue::StringInterpolate { .. } => Ok(Some("string".into())),
            Rvalue::NumericIntrinsic(numeric) => {
                let int_ty = match numeric.width {
                    NumericWidth::W8 => "i8",
                    NumericWidth::W16 => "i16",
                    NumericWidth::W32 => "i32",
                    NumericWidth::W64 | NumericWidth::Pointer => "i64",
                    NumericWidth::W128 => "i128",
                };
                let ty = match numeric.kind {
                    NumericIntrinsicKind::TryAdd
                    | NumericIntrinsicKind::TrySub
                    | NumericIntrinsicKind::TryMul
                    | NumericIntrinsicKind::TryNeg
                    | NumericIntrinsicKind::IsPowerOfTwo => "i8".into(),
                    NumericIntrinsicKind::LeadingZeroCount
                    | NumericIntrinsicKind::TrailingZeroCount
                    | NumericIntrinsicKind::PopCount => "i32".into(),
                    NumericIntrinsicKind::RotateLeft
                    | NumericIntrinsicKind::RotateRight
                    | NumericIntrinsicKind::ReverseEndianness => int_ty.into(),
                };
                Ok(Some(ty))
            }
            Rvalue::DecimalIntrinsic(_) => map_type_owned(
                &Ty::named(DECIMAL_INTRINSIC_RESULT_TY),
                Some(self.type_layouts),
            ),
            Rvalue::AddressOf { .. } => Ok(Some("ptr".into())),
            Rvalue::StaticRef { .. } => Ok(Some("ptr".into())),
            Rvalue::Pending(_) | Rvalue::Aggregate { .. } | Rvalue::StaticLoad { .. } => Ok(None),
            Rvalue::AtomicLoad { target, .. } | Rvalue::AtomicRmw { target, .. } => {
                self.place_type_hint(target, locals)
            }
            Rvalue::AtomicCompareExchange { .. } => Ok(Some("i8".into())),
        }
    }

    fn place_type_hint(
        &self,
        place: &Place,
        locals: &[Option<String>],
    ) -> Result<Option<String>, Error> {
        if place.projection.is_empty() {
            return locals
                .get(place.local.0)
                .cloned()
                .ok_or_else(|| Error::Codegen("place referenced unknown local".into()));
        }
        let ty = self.mir_ty_of_place(place)?;
        map_type_owned(&ty, Some(self.type_layouts))
    }
}
