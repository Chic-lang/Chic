use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn deref_target_ty(&self, pointer: &Ty) -> Option<Ty> {
        match pointer {
            Ty::Pointer(inner) => Some(inner.element.clone()),
            Ty::Ref(inner) => Some(inner.element.clone()),
            Ty::Nullable(inner) => self.deref_target_ty(inner),
            _ => None,
        }
    }

    pub(super) fn operand_float_ty(&self, operand: &Operand) -> Option<ValueType> {
        match operand {
            Operand::Const(constant) => match constant.value() {
                ConstValue::Float(value) => match value.width {
                    FloatWidth::F16 | FloatWidth::F32 => Some(ValueType::F32),
                    FloatWidth::F64 | FloatWidth::F128 => Some(ValueType::F64),
                },
                _ => None,
            },
            _ => self
                .operand_ty(operand)
                .map(|ty| map_type(&ty))
                .filter(|ty| matches!(ty, ValueType::F32 | ValueType::F64)),
        }
    }

    pub(super) fn operand_int_info(&self, operand: &Operand) -> Option<IntInfo> {
        match operand {
            Operand::Const(constant) => self.const_int_info(constant),
            _ => self
                .operand_ty(operand)
                .and_then(|ty| self.int_info_for_ty(&ty)),
        }
    }

    pub(super) fn operand_int128_signed(&self, operand: &Operand) -> Option<bool> {
        self.operand_int_info(operand)
            .filter(|info| info.bits > 64)
            .map(|info| info.signed)
    }

    pub(super) fn int_info_for_ty(&self, ty: &Ty) -> Option<IntInfo> {
        let pointer_size = self.pointer_width_bits() / 8;
        let canonical = ty.canonical_name();
        if let Some(info) = int_info(&self.layouts.primitive_registry, &canonical, pointer_size) {
            return Some(info);
        }
        let short = canonical
            .rsplit("::")
            .next()
            .unwrap_or_else(|| canonical.as_str());
        if short != canonical {
            if let Some(info) = int_info(&self.layouts.primitive_registry, short, pointer_size) {
                return Some(info);
            }
        }
        let layout = self.layouts.layout_for_name(&canonical)?;
        match layout {
            TypeLayout::Enum(enum_layout) => {
                if let Some(info) = enum_layout.underlying_info {
                    return Some(info);
                }
                let bits = enum_layout.size.map(|size| size.saturating_mul(8) as u16)?;
                if bits == 0 {
                    return None;
                }
                Some(IntInfo {
                    bits,
                    signed: !enum_layout.is_flags,
                })
            }
            _ => None,
        }
    }

    pub(super) fn const_int_info(&self, constant: &ConstOperand) -> Option<IntInfo> {
        let literal = constant.literal.as_ref();
        let default_bits = |value: i128| {
            if value >= i32::MIN as i128 && value <= i32::MAX as i128 {
                32
            } else {
                64
            }
        };
        let default_unsigned_bits = |value: u128| if value <= u32::MAX as u128 { 32 } else { 64 };
        match constant.value() {
            ConstValue::Bool(_) => Some(IntInfo {
                bits: 32,
                signed: false,
            }),
            ConstValue::Char(_) => Some(IntInfo {
                bits: 32,
                signed: false,
            }),
            ConstValue::Int(value) | ConstValue::Int32(value) => {
                let bits = literal
                    .and_then(|meta| match meta.literal_type {
                        NumericLiteralType::Signed(width) | NumericLiteralType::Unsigned(width) => {
                            Some(width.bit_width(self.pointer_width_bits()))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| default_bits(*value));
                Some(IntInfo {
                    bits: bits as u16,
                    signed: true,
                })
            }
            ConstValue::UInt(value) => {
                let bits = literal
                    .and_then(|meta| match meta.literal_type {
                        NumericLiteralType::Unsigned(width) | NumericLiteralType::Signed(width) => {
                            Some(width.bit_width(self.pointer_width_bits()))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| default_unsigned_bits(*value));
                Some(IntInfo {
                    bits: bits as u16,
                    signed: false,
                })
            }
            _ => None,
        }
    }
}
