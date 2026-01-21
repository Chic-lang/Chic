use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_const_operand(
        &mut self,
        buf: &mut Vec<u8>,
        constant: &ConstOperand,
    ) -> Result<ValueType, Error> {
        match constant.value() {
            ConstValue::Str { id, .. } => self.emit_str_literal(buf, *id),
            ConstValue::Symbol(name) => {
                if let Some(index) = self.lookup_function_index(name) {
                    if std::env::var_os("CHIC_DEBUG_WASM_FN_ASSIGN").is_some() {
                        eprintln!(
                            "[wasm-const-symbol] func={} symbol={} index={}",
                            self.function.name, name, index
                        );
                    }
                    emit_instruction(
                        buf,
                        Op::I32Const(i32::try_from(index).map_err(|_| {
                            Error::Codegen(
                                "function index exceeds i32 range in WASM backend".into(),
                            )
                        })?),
                    );
                    Ok(ValueType::I32)
                } else if let Some(offset) = self.trait_vtable_offsets.get(name) {
                    emit_instruction(
                        buf,
                        Op::I32Const(i32::try_from(*offset).map_err(|_| {
                            Error::Codegen(
                                "trait vtable offset exceeds i32 range in WASM backend".into(),
                            )
                        })?),
                    );
                    Ok(ValueType::I32)
                } else if let Some(offset) = self.class_vtable_offsets.get(name) {
                    emit_instruction(
                        buf,
                        Op::I32Const(i32::try_from(*offset).map_err(|_| {
                            Error::Codegen(
                                "class vtable offset exceeds i32 range in WASM backend".into(),
                            )
                        })?),
                    );
                    Ok(ValueType::I32)
                } else {
                    // Fall back to a null-ish pointer for missing vtable/class symbols so
                    // wasm lowering can continue even when metadata is stripped.
                    emit_instruction(buf, Op::I32Const(0));
                    Ok(ValueType::I32)
                }
            }
            ConstValue::Null => {
                emit_instruction(buf, Op::I32Const(0));
                Ok(ValueType::I32)
            }
            ConstValue::Bool(value) => {
                emit_instruction(buf, Op::I32Const(i32::from(*value)));
                Ok(ValueType::I32)
            }
            ConstValue::Char(value) => {
                emit_instruction(buf, Op::I32Const(*value as i32));
                Ok(ValueType::I32)
            }
            ConstValue::Int(value) | ConstValue::Int32(value) => {
                let literal = constant.literal.as_ref();
                let declared_bits = literal.and_then(|meta| match meta.literal_type {
                    NumericLiteralType::Signed(width) | NumericLiteralType::Unsigned(width) => {
                        Some(width.bit_width(self.pointer_width_bits()))
                    }
                    _ => None,
                });
                let requires_int128 = declared_bits.is_some_and(|bits| bits > 64)
                    || *value < i64::MIN as i128
                    || *value > i64::MAX as i128;
                if requires_int128 {
                    let (lo, hi) = self.int128_const_parts(&constant.value, true)?;
                    self.allocate_int128_temp(buf, lo, hi, self.stack_temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    return Ok(ValueType::I32);
                }
                self.emit_signed_int_literal(buf, *value, literal)
            }
            ConstValue::UInt(value) => {
                let literal = constant.literal.as_ref();
                let declared_bits = literal.and_then(|meta| match meta.literal_type {
                    NumericLiteralType::Unsigned(width) | NumericLiteralType::Signed(width) => {
                        Some(width.bit_width(self.pointer_width_bits()))
                    }
                    _ => None,
                });
                let requires_int128 =
                    declared_bits.is_some_and(|bits| bits > 64) || *value > u64::MAX as u128;
                if requires_int128 {
                    let (lo, hi) = self.int128_const_parts(&constant.value, false)?;
                    self.allocate_int128_temp(buf, lo, hi, self.stack_temp_local)?;
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    return Ok(ValueType::I32);
                }
                self.emit_unsigned_int_literal(buf, *value, literal)
            }
            ConstValue::Float(value) => {
                self.emit_float_literal(buf, *value, constant.literal.as_ref())
            }
            ConstValue::Decimal(decimal) => {
                let (size, align) = self
                    .layouts
                    .size_and_align_for_ty(&Ty::named("decimal"))
                    .ok_or_else(|| {
                        Error::Codegen("missing `decimal` layout for WASM lowering".into())
                    })?;
                let padded = if align == 0 {
                    size
                } else {
                    let rem = size % align;
                    if rem == 0 {
                        size
                    } else {
                        size.checked_add(align - rem).ok_or_else(|| {
                            Error::Codegen("decimal literal size exceeds addressable range".into())
                        })?
                    }
                };
                let padded_i32 = i32::try_from(padded).map_err(|_| {
                    Error::Codegen(
                        "decimal literal footprint exceeds wasm i32 range for stack allocation"
                            .into(),
                    )
                })?;
                emit_instruction(buf, Op::LocalGet(self.stack_adjust_local));
                emit_instruction(buf, Op::I32Const(padded_i32));
                emit_instruction(buf, Op::I32Add);
                emit_instruction(buf, Op::LocalSet(self.stack_adjust_local));
                emit_instruction(buf, Op::GlobalGet(STACK_POINTER_GLOBAL_INDEX));
                emit_instruction(buf, Op::I32Const(padded_i32));
                emit_instruction(buf, Op::I32Sub);
                emit_instruction(buf, Op::LocalTee(self.stack_temp_local));
                emit_instruction(buf, Op::GlobalSet(STACK_POINTER_GLOBAL_INDEX));
                let parts = decimal.to_bits();
                for (index, part) in parts.iter().enumerate() {
                    emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                    let offset = (index * 4) as i32;
                    if offset != 0 {
                        emit_instruction(buf, Op::I32Const(offset));
                        emit_instruction(buf, Op::I32Add);
                    }
                    emit_instruction(buf, Op::I32Const(*part as i32));
                    emit_instruction(buf, Op::I32Store(0));
                }
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                Ok(ValueType::I32)
            }
            ConstValue::Enum { .. }
            | ConstValue::Struct { .. }
            | ConstValue::RawStr(_)
            | ConstValue::Unit
            | ConstValue::Unknown => {
                emit_instruction(buf, Op::I32Const(0));
                Ok(ValueType::I32)
            }
        }
    }

    pub(super) fn emit_signed_int_literal(
        &self,
        buf: &mut Vec<u8>,
        value: i128,
        literal: Option<&NumericLiteralMetadata>,
    ) -> Result<ValueType, Error> {
        let bits = match literal.map(|meta| &meta.literal_type) {
            Some(NumericLiteralType::Signed(width)) => {
                let bits = width.bit_width(self.pointer_width_bits());
                self.ensure_supported_int_width(bits)?;
                self.ensure_signed_range(value, bits)?;
                bits
            }
            Some(NumericLiteralType::Unsigned(width)) => {
                let bits = width.bit_width(self.pointer_width_bits());
                self.ensure_supported_int_width(bits)?;
                self.ensure_signed_range(value, bits)?;
                bits
            }
            Some(
                NumericLiteralType::Float16
                | NumericLiteralType::Float32
                | NumericLiteralType::Float64
                | NumericLiteralType::Float128
                | NumericLiteralType::Decimal,
            ) => {
                return Err(Error::Codegen(
                    "numeric literal metadata does not match integer constant".into(),
                ));
            }
            None => {
                if value >= i32::MIN as i128 && value <= i32::MAX as i128 {
                    32
                } else if value >= i64::MIN as i128 && value <= i64::MAX as i128 {
                    64
                } else {
                    return Err(Error::Codegen(format!(
                        "integer literal exceeds 64-bit range in WASM backend (value={value}, function={})",
                        self.function.name
                    )));
                }
            }
        };

        if bits <= 32 {
            let narrowed = i32::try_from(value).map_err(|_| {
                Error::Codegen("integer literal exceeds 32-bit range in WASM backend".into())
            })?;
            emit_instruction(buf, Op::I32Const(narrowed));
            Ok(ValueType::I32)
        } else if bits <= 64 {
            let narrowed = i64::try_from(value).map_err(|_| {
                Error::Codegen("integer literal exceeds 64-bit range in WASM backend".into())
            })?;
            emit_instruction(buf, Op::I64Const(narrowed));
            Ok(ValueType::I64)
        } else {
            Err(Error::Codegen(
                "128-bit integer literals are not supported by the WASM backend yet".into(),
            ))
        }
    }

    pub(super) fn emit_unsigned_int_literal(
        &self,
        buf: &mut Vec<u8>,
        value: u128,
        literal: Option<&NumericLiteralMetadata>,
    ) -> Result<ValueType, Error> {
        let bits = match literal.map(|meta| &meta.literal_type) {
            Some(NumericLiteralType::Unsigned(width)) => {
                let bits = width.bit_width(self.pointer_width_bits());
                self.ensure_supported_int_width(bits)?;
                self.ensure_unsigned_range(value, bits)?;
                bits
            }
            Some(NumericLiteralType::Signed(width)) => {
                let bits = width.bit_width(self.pointer_width_bits());
                self.ensure_supported_int_width(bits)?;
                self.ensure_unsigned_range(value, bits)?;
                bits
            }
            Some(
                NumericLiteralType::Float16
                | NumericLiteralType::Float32
                | NumericLiteralType::Float64
                | NumericLiteralType::Float128
                | NumericLiteralType::Decimal,
            ) => {
                return Err(Error::Codegen(
                    "numeric literal metadata does not match integer constant".into(),
                ));
            }
            None => {
                if value <= u32::MAX as u128 {
                    32
                } else if value <= u64::MAX as u128 {
                    64
                } else {
                    return Err(Error::Codegen(format!(
                        "unsigned integer literal exceeds 64-bit range in WASM backend (value={value}, function={})",
                        self.function.name
                    )));
                }
            }
        };

        if bits <= 32 {
            let max = 1u128 << bits;
            if value >= max {
                return Err(Error::Codegen(
                    "unsigned integer literal exceeds declared width in WASM backend".into(),
                ));
            }
            let narrowed = u32::try_from(value).map_err(|_| {
                Error::Codegen(
                    "unsigned integer literal exceeds 32-bit range in WASM backend".into(),
                )
            })?;
            let repr = i32::from_le_bytes(narrowed.to_le_bytes());
            emit_instruction(buf, Op::I32Const(repr));
            Ok(ValueType::I32)
        } else if bits <= 64 {
            let max = 1u128 << bits;
            if value >= max {
                return Err(Error::Codegen(
                    "unsigned integer literal exceeds declared width in WASM backend".into(),
                ));
            }
            let narrowed = u64::try_from(value).map_err(|_| {
                Error::Codegen(
                    "unsigned integer literal exceeds 64-bit range in WASM backend".into(),
                )
            })?;
            let repr = i64::from_le_bytes(narrowed.to_le_bytes());
            emit_instruction(buf, Op::I64Const(repr));
            Ok(ValueType::I64)
        } else {
            Err(Error::Codegen(
                "128-bit integer literals are not supported by the WASM backend yet".into(),
            ))
        }
    }

    pub(super) fn emit_float_literal(
        &self,
        buf: &mut Vec<u8>,
        value: FloatValue,
        literal: Option<&NumericLiteralMetadata>,
    ) -> Result<ValueType, Error> {
        let meta_width = match literal.map(|meta| &meta.literal_type) {
            Some(NumericLiteralType::Float16) => Some(FloatWidth::F16),
            Some(NumericLiteralType::Float32) => Some(FloatWidth::F32),
            Some(NumericLiteralType::Float64) => Some(FloatWidth::F64),
            Some(NumericLiteralType::Float128) => Some(FloatWidth::F128),
            Some(
                NumericLiteralType::Signed(_)
                | NumericLiteralType::Unsigned(_)
                | NumericLiteralType::Decimal,
            ) => {
                return Err(Error::Codegen(
                    "numeric literal metadata does not match floating-point constant".into(),
                ));
            }
            None => None,
        };
        let width = meta_width.unwrap_or(value.width);
        match width {
            FloatWidth::F16 => Err(Error::Codegen(
                "float16 literals are not supported in the WASM backend (no half-precision instruction support)"
                    .into(),
            )),
            FloatWidth::F32 => {
                emit_instruction(buf, Op::F32Const(value.to_f32()));
                Ok(ValueType::F32)
            }
            FloatWidth::F64 => {
                emit_instruction(buf, Op::F64Const(value.to_f64()));
                Ok(ValueType::F64)
            }
            FloatWidth::F128 => match float::float128_mode() {
                Float128Mode::Unsupported => Err(Error::Codegen(
                    "float128 literals are disabled for this target; set CHIC_FLOAT128=emulate to downcast to f64 in WASM"
                        .into(),
                )),
                _ => {
                    emit_instruction(buf, Op::F64Const(value.to_f64()));
                    Ok(ValueType::F64)
                }
            },
        }
    }

    pub(super) fn ensure_supported_int_width(&self, bits: u32) -> Result<(), Error> {
        if bits == 0 {
            return Err(Error::Codegen(
                "integer literals must specify a non-zero bit width".into(),
            ));
        }
        if bits > 128 {
            return Err(Error::Codegen(
                "integer literals wider than 128 bits are not supported by the WASM backend".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn ensure_signed_range(&self, value: i128, bits: u32) -> Result<(), Error> {
        let max = (1i128 << (bits - 1)) - 1;
        let min = -(1i128 << (bits - 1));
        if value < min || value > max {
            Err(Error::Codegen(
                "integer literal exceeds declared width in WASM backend".into(),
            ))
        } else {
            Ok(())
        }
    }

    pub(super) fn ensure_unsigned_range(&self, value: u128, bits: u32) -> Result<(), Error> {
        if bits >= 128 {
            return Ok(()); // handled elsewhere
        }
        let max = 1u128 << bits;
        if value < max {
            Ok(())
        } else {
            Err(Error::Codegen(
                "unsigned integer literal exceeds declared width in WASM backend".into(),
            ))
        }
    }
}
