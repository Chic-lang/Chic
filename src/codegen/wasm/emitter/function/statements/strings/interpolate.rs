use crate::codegen::wasm::emitter::function::FunctionEmitter;
use crate::codegen::wasm::emitter::function::ops::{Op, emit_instruction};
use crate::codegen::wasm::emitter::function::statements::{
    InterpolatedOperandKind, float_constant_bits, minimal_signed_bits, minimal_unsigned_bits,
    short_type_name,
};
use crate::codegen::wasm::{RuntimeHook, ValueType};
use crate::error::Error;
use crate::mir::{ConstOperand, ConstValue, InterpolatedStringSegment, Operand, Place, StrId, Ty};
#[cfg(test)]
use crate::syntax::numeric::IntegerWidth;
use crate::syntax::numeric::{NumericLiteralMetadata, NumericLiteralType};

fn integer_bits_from_literal(
    literal: Option<&NumericLiteralMetadata>,
    pointer_bits: u32,
) -> Option<u32> {
    let meta = literal?;
    match meta.literal_type {
        NumericLiteralType::Signed(width) | NumericLiteralType::Unsigned(width) => {
            Some(width.bit_width(pointer_bits))
        }
        _ => None,
    }
}

fn float_bits_from_literal(literal: Option<&NumericLiteralMetadata>) -> Option<u32> {
    let meta = literal?;
    match meta.literal_type {
        NumericLiteralType::Float32 => Some(32),
        NumericLiteralType::Float64 => Some(64),
        _ => None,
    }
}

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_string_interpolate(
        &mut self,
        buf: &mut Vec<u8>,
        place: &Place,
        segments: &[InterpolatedStringSegment],
    ) -> Result<(), Error> {
        self.emit_zero_init(buf, place)?;
        let dest_ptr = self.pointer_local_index(place.local)?;
        for segment in segments {
            match segment {
                InterpolatedStringSegment::Text { id } => {
                    self.emit_interpolation_text_segment(buf, dest_ptr, *id)?;
                }
                InterpolatedStringSegment::Expr {
                    operand,
                    alignment,
                    format,
                    ..
                } => {
                    self.emit_interpolation_expr_segment(
                        buf, dest_ptr, operand, *alignment, *format,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn emit_interpolation_text_segment(
        &mut self,
        buf: &mut Vec<u8>,
        dest_ptr: u32,
        id: StrId,
    ) -> Result<(), Error> {
        let literal = self.string_literals.get(&id).ok_or_else(|| {
            Error::Codegen(format!(
                "missing interned string literal {} for interpolation segment",
                id.index()
            ))
        })?;
        if literal.len == 0 {
            return Ok(());
        }
        emit_instruction(buf, Op::LocalGet(dest_ptr));
        emit_instruction(buf, Op::I32Const(literal.offset as i32));
        emit_instruction(buf, Op::I32Const(literal.len as i32));
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::I32Const(0));
        let hook = self.runtime_hook_index(RuntimeHook::StringAppendSlice)?;
        emit_instruction(buf, Op::Call(hook));
        emit_instruction(buf, Op::Drop);
        Ok(())
    }

    pub(crate) fn emit_interpolation_expr_segment(
        &mut self,
        buf: &mut Vec<u8>,
        dest_ptr: u32,
        operand: &Operand,
        alignment: Option<i32>,
        format: Option<StrId>,
    ) -> Result<(), Error> {
        let alignment_value = alignment.unwrap_or(0);
        let has_alignment = if alignment.is_some() { 1 } else { 0 };
        let kind = self.classify_interpolated_operand(operand)?;

        match kind {
            InterpolatedOperandKind::Str => {
                if format.is_some() {
                    return Err(Error::Codegen(
                        "format specifiers are not supported for `str` interpolation segments"
                            .into(),
                    ));
                }
                let value_ty = self.emit_operand(buf, operand)?;
                if value_ty != ValueType::I64 {
                    return Err(Error::Codegen(
                        "expected `str` operands to lower to i64 values in WASM backend".into(),
                    ));
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(32));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                let hook = self.runtime_hook_index(RuntimeHook::StringAppendSlice)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::String => {
                if format.is_some() {
                    return Err(Error::Codegen(
                        "format specifiers are not supported for `string` interpolation segments"
                            .into(),
                    ));
                }
                let ptr_local = self.string_operand_pointer(buf, operand)?;
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                emit_instruction(buf, Op::LocalGet(ptr_local));
                let slice_hook = self.runtime_hook_index(RuntimeHook::StringAsSlice)?;
                emit_instruction(buf, Op::Call(slice_hook));
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                let append_hook = self.runtime_hook_index(RuntimeHook::StringAppendSlice)?;
                emit_instruction(buf, Op::Call(append_hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::Bool => {
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::I32 => {}
                    ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
                    other => {
                        return Err(Error::Codegen(format!(
                            "boolean interpolation expects integer value, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                self.emit_format_arguments(buf, format)?;
                let hook = self.runtime_hook_index(RuntimeHook::StringAppendBool)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::Char => {
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::I32 => {}
                    ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
                    other => {
                        return Err(Error::Codegen(format!(
                            "char interpolation expects integer value, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                self.emit_format_arguments(buf, format)?;
                let hook = self.runtime_hook_index(RuntimeHook::StringAppendChar)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::SignedInt { bits } => {
                if bits > 128 {
                    return Err(Error::Codegen(format!(
                        "integer interpolation wider than 128 bits is not supported by the WASM backend ({bits} bits requested)"
                    )));
                }
                if bits > 64 {
                    let (low, high) = self.emit_i128_halves(buf, operand, true)?;
                    emit_instruction(buf, Op::LocalGet(dest_ptr));
                    emit_instruction(buf, Op::LocalGet(low));
                    emit_instruction(buf, Op::LocalGet(high));
                    emit_instruction(buf, Op::I32Const(bits as i32));
                    emit_instruction(buf, Op::I32Const(alignment_value));
                    emit_instruction(buf, Op::I32Const(has_alignment));
                    self.emit_format_arguments(buf, format)?;
                    let hook = self.runtime_hook_index(RuntimeHook::StringAppendSigned)?;
                    emit_instruction(buf, Op::Call(hook));
                    emit_instruction(buf, Op::Drop);
                    return Ok(());
                }
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I64ExtendI32S),
                    ValueType::I64 => {}
                    other => {
                        return Err(Error::Codegen(format!(
                            "signed integer interpolation expects scalar integer value, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(63));
                emit_instruction(buf, Op::I64ShrS);
                emit_instruction(buf, Op::I32Const(bits as i32));
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                self.emit_format_arguments(buf, format)?;
                let hook = self.runtime_hook_index(RuntimeHook::StringAppendSigned)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::UnsignedInt { bits } => {
                if bits > 128 {
                    return Err(Error::Codegen(format!(
                        "integer interpolation wider than 128 bits is not supported by the WASM backend ({bits} bits requested)"
                    )));
                }
                if bits > 64 {
                    let (low, high) = self.emit_i128_halves(buf, operand, false)?;
                    emit_instruction(buf, Op::LocalGet(dest_ptr));
                    emit_instruction(buf, Op::LocalGet(low));
                    emit_instruction(buf, Op::LocalGet(high));
                    emit_instruction(buf, Op::I32Const(bits as i32));
                    emit_instruction(buf, Op::I32Const(alignment_value));
                    emit_instruction(buf, Op::I32Const(has_alignment));
                    self.emit_format_arguments(buf, format)?;
                    let hook = self.runtime_hook_index(RuntimeHook::StringAppendUnsigned)?;
                    emit_instruction(buf, Op::Call(hook));
                    emit_instruction(buf, Op::Drop);
                    return Ok(());
                }
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::I32 => emit_instruction(buf, Op::I64ExtendI32U),
                    ValueType::I64 => {}
                    other => {
                        return Err(Error::Codegen(format!(
                            "unsigned integer interpolation expects scalar integer value, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(0));
                emit_instruction(buf, Op::I32Const(bits as i32));
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                self.emit_format_arguments(buf, format)?;
                let hook = self.runtime_hook_index(RuntimeHook::StringAppendUnsigned)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
            InterpolatedOperandKind::Float { bits } => {
                let hook = if bits == 32 {
                    RuntimeHook::StringAppendF32
                } else {
                    RuntimeHook::StringAppendF64
                };
                emit_instruction(buf, Op::LocalGet(dest_ptr));
                let value_ty = self.emit_operand(buf, operand)?;
                match (bits, value_ty) {
                    (32, ValueType::F32) => {}
                    (64, ValueType::F64) => {}
                    (32, ValueType::F64) => {
                        emit_instruction(buf, Op::F32DemoteF64);
                    }
                    (64, ValueType::F32) => {
                        emit_instruction(buf, Op::F64PromoteF32);
                    }
                    (_, other) => {
                        return Err(Error::Codegen(format!(
                            "float interpolation expects scalar float value, found {other:?}"
                        )));
                    }
                }
                emit_instruction(buf, Op::I32Const(alignment_value));
                emit_instruction(buf, Op::I32Const(has_alignment));
                self.emit_format_arguments(buf, format)?;
                let hook_index = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::Call(hook_index));
                emit_instruction(buf, Op::Drop);
                Ok(())
            }
        }
    }

    fn emit_format_arguments(
        &mut self,
        buf: &mut Vec<u8>,
        format: Option<StrId>,
    ) -> Result<(), Error> {
        if let Some(id) = format {
            let literal = self.string_literals.get(&id).ok_or_else(|| {
                Error::Codegen(format!(
                    "missing interned format literal {} for interpolation segment",
                    id.index()
                ))
            })?;
            emit_instruction(buf, Op::I32Const(literal.offset as i32));
            emit_instruction(buf, Op::I32Const(literal.len as i32));
        } else {
            emit_instruction(buf, Op::I32Const(0));
            emit_instruction(buf, Op::I32Const(0));
        }
        Ok(())
    }

    fn string_operand_pointer(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<u32, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    return self.pointer_local_index(place.local);
                }
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::LocalSet(self.temp_local));
                Ok(self.temp_local)
            }
            _ => Err(Error::Codegen(
                "string interpolation requires direct `string` locals in the WASM backend".into(),
            )),
        }
    }

    pub(crate) fn emit_str_literal(
        &self,
        buf: &mut Vec<u8>,
        id: StrId,
    ) -> Result<ValueType, Error> {
        let literal = self.string_literals.get(&id).ok_or_else(|| {
            Error::Codegen(format!("missing interned string literal {}", id.index()))
        })?;
        let packed = ((u64::from(literal.len)) << 32) | u64::from(literal.offset);
        emit_instruction(buf, Op::I64Const(packed as i64));
        Ok(ValueType::I64)
    }

    fn emit_i128_halves(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        _signed: bool,
    ) -> Result<(u32, u32), Error> {
        match operand {
            Operand::Const(constant) => {
                let (low, high) = match &constant.value {
                    ConstValue::Int(v) => ((*v as u64) as i64, (*v >> 64) as i64),
                    ConstValue::UInt(v) => ((*v as u64) as i64, (*v >> 64) as u64 as i64),
                    other => {
                        return Err(Error::Codegen(format!(
                            "128-bit interpolation expects an integer constant, found {other:?}"
                        )));
                    }
                };
                emit_instruction(buf, Op::I64Const(low));
                let low_local = self.wide_temp_local;
                emit_instruction(buf, Op::LocalSet(low_local));
                emit_instruction(buf, Op::I64Const(high));
                let high_local = self.wide_temp_local_hi;
                emit_instruction(buf, Op::LocalSet(high_local));
                Ok((low_local, high_local))
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let access = self.resolve_memory_access(place)?;
                let base_ty = self
                    .local_tys
                    .get(place.local.0)
                    .cloned()
                    .unwrap_or(Ty::Unknown);
                let canonical = base_ty.canonical_name().to_ascii_lowercase();
                let is_i128 = matches!(
                    canonical.as_str(),
                    "i128" | "int128" | "system::int128" | "std::int128"
                );
                let is_u128 = matches!(
                    canonical.as_str(),
                    "u128" | "uint128" | "system::uint128" | "std::uint128"
                );
                if !is_i128 && !is_u128 {
                    return Err(Error::Codegen(
                        "128-bit interpolation only supports i128/u128 operands in WASM backend"
                            .into(),
                    ));
                }
                // Load low half
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::I64Load(0));
                let low_local = self.wide_temp_local;
                emit_instruction(buf, Op::LocalSet(low_local));
                // Load high half
                self.emit_pointer_expression(buf, &access)?;
                emit_instruction(buf, Op::I64Load(8));
                let high_local = self.wide_temp_local_hi;
                emit_instruction(buf, Op::LocalSet(high_local));
                Ok((low_local, high_local))
            }
            _ => Err(Error::Codegen(
                "128-bit interpolation requires value or constant operands".into(),
            )),
        }
    }

    fn classify_interpolated_operand(
        &self,
        operand: &Operand,
    ) -> Result<InterpolatedOperandKind, Error> {
        match operand {
            Operand::Const(constant) => self.classify_const_operand(constant),
            Operand::Copy(_) | Operand::Move(_) => {
                let ty = self.operand_ty(operand).unwrap_or(Ty::Unknown);
                self.classify_ty(&ty, operand)
            }
            Operand::Borrow(_) => Err(Error::Codegen(
                "borrowed values are not yet supported in string interpolation".into(),
            )),
            Operand::Mmio(_) => Err(Error::Codegen(
                "MMIO operands cannot be interpolated into strings".into(),
            )),
            Operand::Pending(pending) => Err(Error::Codegen(format!(
                "pending operand `{}` cannot be interpolated into a string",
                pending.repr
            ))),
        }
    }

    fn classify_const_operand(
        &self,
        constant: &ConstOperand,
    ) -> Result<InterpolatedOperandKind, Error> {
        match &constant.value {
            ConstValue::Str { .. } | ConstValue::RawStr(_) => Ok(InterpolatedOperandKind::Str),
            ConstValue::Bool(_) => Ok(InterpolatedOperandKind::Bool),
            ConstValue::Char(_) => Ok(InterpolatedOperandKind::Char),
            ConstValue::Int(v) | ConstValue::Int32(v) => Ok(InterpolatedOperandKind::SignedInt {
                bits: integer_bits_from_literal(
                    constant.literal.as_ref(),
                    self.pointer_width_bits(),
                )
                .unwrap_or_else(|| minimal_signed_bits(*v)),
            }),
            ConstValue::UInt(v) => Ok(InterpolatedOperandKind::UnsignedInt {
                bits: integer_bits_from_literal(
                    constant.literal.as_ref(),
                    self.pointer_width_bits(),
                )
                .unwrap_or_else(|| minimal_unsigned_bits(*v)),
            }),
            ConstValue::Enum { discriminant, .. } => Ok(InterpolatedOperandKind::SignedInt {
                bits: minimal_signed_bits(*discriminant),
            }),
            ConstValue::Float(v) => Ok(InterpolatedOperandKind::Float {
                bits: float_bits_from_literal(constant.literal.as_ref())
                    .unwrap_or_else(|| float_constant_bits(*v)),
            }),
            ConstValue::Decimal(_) => Err(Error::Codegen(
                "decimal constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Struct { .. } => Err(Error::Codegen(
                "struct constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Null => Err(Error::Codegen(
                "`null` constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Unit | ConstValue::Symbol(_) | ConstValue::Unknown => Err(Error::Codegen(
                "constant cannot be interpolated into a string".into(),
            )),
        }
    }

    fn classify_ty(&self, ty: &Ty, operand: &Operand) -> Result<InterpolatedOperandKind, Error> {
        match ty {
            Ty::String => Ok(InterpolatedOperandKind::String),
            Ty::Str => Ok(InterpolatedOperandKind::Str),
            Ty::Tuple(_) => Err(Error::Codegen(
                "tuple values cannot be interpolated into a string".into(),
            )),
            Ty::Named(name) => self.classify_named_type(name),
            Ty::Unknown => self.classify_unknown_local(operand),
            Ty::Nullable(inner) => self.classify_ty(inner, operand),
            Ty::Array(_)
            | Ty::Vec(_)
            | Ty::Vector(_)
            | Ty::Span(_)
            | Ty::ReadOnlySpan(_)
            | Ty::Fn(_)
            | Ty::Pointer(_)
            | Ty::Ref(_)
            | Ty::Rc(_)
            | Ty::Arc(_)
            | Ty::TraitObject(_) => Err(Error::Codegen(format!(
                "type `{}` is not supported in string interpolation",
                ty.canonical_name()
            ))),
            Ty::Unit => Err(Error::Codegen(
                "`void` values cannot be interpolated into a string".into(),
            )),
        }
    }

    fn classify_named_type(&self, name: &str) -> Result<InterpolatedOperandKind, Error> {
        let short = short_type_name(name);
        if short.eq_ignore_ascii_case("string") {
            return Ok(InterpolatedOperandKind::String);
        }
        if short.eq_ignore_ascii_case("str") {
            return Ok(InterpolatedOperandKind::Str);
        }

        let lowered = short.to_ascii_lowercase();
        match lowered.as_str() {
            "bool" | "boolean" => Ok(InterpolatedOperandKind::Bool),
            "char" => Ok(InterpolatedOperandKind::Char),
            "sbyte" | "int8" => Ok(InterpolatedOperandKind::SignedInt { bits: 8 }),
            "short" | "int16" => Ok(InterpolatedOperandKind::SignedInt { bits: 16 }),
            "int" | "int32" => Ok(InterpolatedOperandKind::SignedInt { bits: 32 }),
            "long" | "int64" => Ok(InterpolatedOperandKind::SignedInt { bits: 64 }),
            "i128" | "int128" => Ok(InterpolatedOperandKind::SignedInt { bits: 128 }),
            "nint" | "isize" => Ok(InterpolatedOperandKind::SignedInt {
                bits: self.pointer_width_bits(),
            }),
            "byte" | "uint8" => Ok(InterpolatedOperandKind::UnsignedInt { bits: 8 }),
            "ushort" | "uint16" => Ok(InterpolatedOperandKind::UnsignedInt { bits: 16 }),
            "uint" | "uint32" => Ok(InterpolatedOperandKind::UnsignedInt { bits: 32 }),
            "ulong" | "uint64" => Ok(InterpolatedOperandKind::UnsignedInt { bits: 64 }),
            "u128" | "uint128" => Ok(InterpolatedOperandKind::UnsignedInt { bits: 128 }),
            "nuint" | "usize" => Ok(InterpolatedOperandKind::UnsignedInt {
                bits: self.pointer_width_bits(),
            }),
            "float" | "single" | "system::single" | "system::float" => {
                Ok(InterpolatedOperandKind::Float { bits: 32 })
            }
            "double" | "system::double" => Ok(InterpolatedOperandKind::Float { bits: 64 }),
            _ => Err(Error::Codegen(format!(
                "interpolated expression uses unsupported type `{}`",
                name
            ))),
        }
    }

    fn classify_unknown_local(&self, operand: &Operand) -> Result<InterpolatedOperandKind, Error> {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => place,
            _ => {
                return Err(Error::Codegen(
                    "unable to infer interpolation type for operand".into(),
                ));
            }
        };
        let value_ty = self
            .local_types
            .get(place.local.0)
            .copied()
            .unwrap_or(ValueType::I32);
        match value_ty {
            ValueType::I32 => Ok(InterpolatedOperandKind::SignedInt { bits: 32 }),
            ValueType::I64 => Ok(InterpolatedOperandKind::SignedInt { bits: 64 }),
            ValueType::F32 => Ok(InterpolatedOperandKind::Float { bits: 32 }),
            ValueType::F64 => Ok(InterpolatedOperandKind::Float { bits: 64 }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_meta(kind: NumericLiteralType) -> NumericLiteralMetadata {
        NumericLiteralMetadata {
            literal_type: kind,
            suffix_text: None,
            explicit_suffix: true,
        }
    }

    #[test]
    fn integer_bits_from_literal_honours_width() {
        let meta = literal_meta(NumericLiteralType::Unsigned(IntegerWidth::W16));
        assert_eq!(integer_bits_from_literal(Some(&meta), 32), Some(16));

        let signed = literal_meta(NumericLiteralType::Signed(IntegerWidth::W64));
        assert_eq!(integer_bits_from_literal(Some(&signed), 64), Some(64));
    }

    #[test]
    fn float_bits_from_literal_honours_precision() {
        let meta = literal_meta(NumericLiteralType::Float32);
        assert_eq!(float_bits_from_literal(Some(&meta)), Some(32));

        let double = literal_meta(NumericLiteralType::Float64);
        assert_eq!(float_bits_from_literal(Some(&double)), Some(64));

        let other = literal_meta(NumericLiteralType::Decimal);
        assert_eq!(float_bits_from_literal(Some(&other)), None);
    }
}
