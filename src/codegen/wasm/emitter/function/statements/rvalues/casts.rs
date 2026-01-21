use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_cast(
        &mut self,
        buf: &mut Vec<u8>,
        kind: CastKind,
        operand: &Operand,
        source: &Ty,
        target: &Ty,
    ) -> Result<ValueType, Error> {
        let pointer_size = self.pointer_width_bits() / 8;
        let int_info_for = |name: &str| -> Option<IntInfo> {
            if let Some(info) = int_info(&self.layouts.primitive_registry, name, pointer_size) {
                return Some(info);
            }
            let layout = self.layouts.layout_for_name(name)?;
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
        };

        match kind {
            CastKind::IntToInt => {
                let source_name = source.canonical_name();
                let target_name = target.canonical_name();
                let source_info = int_info_for(&source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let target_info = int_info_for(&target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                self.emit_integer_cast(buf, operand, source_info, target_info)
            }
            CastKind::PointerToInt => {
                let target_name = target.canonical_name();
                let target_info = int_info_for(&target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                let pointer_info = IntInfo {
                    bits: self.pointer_width_bits() as u16,
                    signed: false,
                };
                self.emit_integer_cast(buf, operand, pointer_info, target_info)
            }
            CastKind::IntToPointer => {
                let source_name = source.canonical_name();
                let source_info = int_info_for(&source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`"
                    ))
                })?;
                let pointer_info = IntInfo {
                    bits: self.pointer_width_bits() as u16,
                    signed: false,
                };
                self.emit_integer_cast(buf, operand, source_info, pointer_info)
            }
            CastKind::IntToFloat => {
                let source_name = source.canonical_name();
                let source_info = int_info_for(&source_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{source_name}`",
                    ))
                })?;
                let target_ty = map_type(target);
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::I32 => {
                        self.canonicalise_i32(buf, source_info);
                        match target_ty {
                            ValueType::F32 => emit_instruction(
                                buf,
                                if source_info.signed {
                                    Op::F32ConvertI32S
                                } else {
                                    Op::F32ConvertI32U
                                },
                            ),
                            ValueType::F64 => emit_instruction(
                                buf,
                                if source_info.signed {
                                    Op::F64ConvertI32S
                                } else {
                                    Op::F64ConvertI32U
                                },
                            ),
                            other => {
                                return Err(Error::Codegen(format!(
                                    "cannot cast integer to {:?} in WASM backend",
                                    other
                                )));
                            }
                        }
                    }
                    ValueType::I64 => {
                        self.canonicalise_i64(buf, source_info);
                        match target_ty {
                            ValueType::F32 => emit_instruction(
                                buf,
                                if source_info.signed {
                                    Op::F32ConvertI64S
                                } else {
                                    Op::F32ConvertI64U
                                },
                            ),
                            ValueType::F64 => emit_instruction(
                                buf,
                                if source_info.signed {
                                    Op::F64ConvertI64S
                                } else {
                                    Op::F64ConvertI64U
                                },
                            ),
                            other => {
                                return Err(Error::Codegen(format!(
                                    "cannot cast integer to {:?} in WASM backend",
                                    other
                                )));
                            }
                        }
                    }
                    other => {
                        return Err(Error::Codegen(format!(
                            "expected integer operand for cast, found {:?}",
                            other
                        )));
                    }
                }
                Ok(target_ty)
            }
            CastKind::FloatToInt => {
                let target_name = target.canonical_name();
                let target_info = int_info_for(&target_name).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine integer metadata for `{target_name}`"
                    ))
                })?;
                let value_ty = self.emit_operand(buf, operand)?;
                match value_ty {
                    ValueType::F32 => {
                        let op = if target_info.signed {
                            Op::I32TruncF32S
                        } else {
                            Op::I32TruncF32U
                        };
                        if target_info.bits > 32 {
                            let op64 = if target_info.signed {
                                Op::I64TruncF32S
                            } else {
                                Op::I64TruncF32U
                            };
                            emit_instruction(buf, op64);
                            self.canonicalise_i64(buf, target_info);
                            Ok(ValueType::I64)
                        } else {
                            emit_instruction(buf, op);
                            self.canonicalise_i32(buf, target_info);
                            Ok(ValueType::I32)
                        }
                    }
                    ValueType::F64 => {
                        if target_info.bits > 32 {
                            let op = if target_info.signed {
                                Op::I64TruncF64S
                            } else {
                                Op::I64TruncF64U
                            };
                            emit_instruction(buf, op);
                            self.canonicalise_i64(buf, target_info);
                            Ok(ValueType::I64)
                        } else {
                            let op = if target_info.signed {
                                Op::I32TruncF64S
                            } else {
                                Op::I32TruncF64U
                            };
                            emit_instruction(buf, op);
                            self.canonicalise_i32(buf, target_info);
                            Ok(ValueType::I32)
                        }
                    }
                    other => Err(Error::Codegen(format!(
                        "expected floating-point operand for cast, found {:?}",
                        other
                    ))),
                }
            }
            CastKind::FloatToFloat => {
                let value_ty = self.emit_operand(buf, operand)?;
                let target_ty = map_type(target);
                match (value_ty, target_ty) {
                    (ValueType::F32, ValueType::F32) | (ValueType::F64, ValueType::F64) => {
                        Ok(target_ty)
                    }
                    (ValueType::F32, ValueType::F64) => {
                        emit_instruction(buf, Op::F64PromoteF32);
                        Ok(ValueType::F64)
                    }
                    (ValueType::F64, ValueType::F32) => {
                        emit_instruction(buf, Op::F32DemoteF64);
                        Ok(ValueType::F32)
                    }
                    other => Err(Error::Codegen(format!(
                        "unsupported float cast combination: {:?}",
                        other
                    ))),
                }
            }
            CastKind::Unknown => {
                let source_ty = map_type(source);
                let target_ty = map_type(target);
                let value_ty = self.emit_operand(buf, operand)?;
                if source_ty == target_ty {
                    if value_ty != target_ty {
                        return Err(Error::Codegen(format!(
                            "operand type {:?} does not match expected {:?} for identity cast",
                            value_ty, target_ty
                        )));
                    }
                    return Ok(target_ty);
                }
                Err(Error::Codegen(format!(
                    "unsupported unknown cast from `{}` ({:?}) to `{}` ({:?}) in WASM backend",
                    source.canonical_name(),
                    source_ty,
                    target.canonical_name(),
                    target_ty
                )))
            }
            CastKind::DynTrait => Err(Error::Codegen(format!(
                "cast kind `{kind:?}` is not supported by WASM backend"
            ))),
        }
    }

    pub(super) fn emit_integer_cast(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        source_info: IntInfo,
        target_info: IntInfo,
    ) -> Result<ValueType, Error> {
        if source_info.bits == 0 {
            return Err(Error::Codegen(format!(
                "source integer width {} is not supported in WASM backend",
                source_info.bits
            )));
        }
        if source_info.bits > 64 || target_info.bits > 64 {
            if source_info.bits == 128 && target_info.bits <= 64 {
                if std::env::var_os("CHIC_DEBUG_WASM_I128_CAST").is_some() {
                    eprintln!(
                        "[wasm-i128-cast] func={} operand_ty={:?} target_bits={}",
                        self.function.name,
                        self.operand_ty(operand)
                            .map(|ty| ty.canonical_name())
                            .unwrap_or_else(|| "<unknown>".into()),
                        target_info.bits
                    );
                }
                self.materialize_int128_operand(
                    buf,
                    operand,
                    source_info.signed,
                    self.block_local,
                )?;
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::I64Load(0));
                if target_info.bits > 32 {
                    self.canonicalise_i64(buf, target_info);
                    return Ok(ValueType::I64);
                }
                emit_instruction(buf, Op::I32WrapI64);
                self.canonicalise_i32(buf, target_info);
                return Ok(ValueType::I32);
            }
            return Err(Error::Codegen(format!(
                "source/target integer width combination ({} -> {}) is not supported in WASM backend",
                source_info.bits, target_info.bits
            )));
        }
        if target_info.bits == 0 {
            return Err(Error::Codegen(format!(
                "target integer width {} is not supported in WASM backend",
                target_info.bits
            )));
        }

        let value_ty = self.emit_operand(buf, operand)?;
        match value_ty {
            ValueType::I32 => self.canonicalise_i32(buf, source_info),
            ValueType::I64 => self.canonicalise_i64(buf, source_info),
            other => {
                return Err(Error::Codegen(format!(
                    "expected integer operand for cast, found {:?}",
                    other
                )));
            }
        }

        let desired_ty = if target_info.bits > 32 {
            ValueType::I64
        } else {
            ValueType::I32
        };

        if desired_ty == ValueType::I64 && value_ty == ValueType::I32 {
            let op = if source_info.signed {
                Op::I64ExtendI32S
            } else {
                Op::I64ExtendI32U
            };
            emit_instruction(buf, op);
        } else if desired_ty == ValueType::I32 && value_ty == ValueType::I64 {
            emit_instruction(buf, Op::I32WrapI64);
        }

        if desired_ty == ValueType::I32 {
            self.canonicalise_i32(buf, target_info);
            Ok(ValueType::I32)
        } else {
            self.canonicalise_i64(buf, target_info);
            Ok(ValueType::I64)
        }
    }

    pub(super) fn canonicalise_i32(&self, buf: &mut Vec<u8>, info: IntInfo) {
        if info.bits == 0 || info.bits >= 32 {
            return;
        }
        if info.signed {
            let shift = 32 - info.bits;
            emit_instruction(buf, Op::I32Const(shift as i32));
            emit_instruction(buf, Op::I32Shl);
            emit_instruction(buf, Op::I32Const(shift as i32));
            emit_instruction(buf, Op::I32ShrS);
        } else {
            let mask = ((1u64 << info.bits) - 1) as i32;
            emit_instruction(buf, Op::I32Const(mask));
            emit_instruction(buf, Op::I32And);
        }
    }

    pub(super) fn canonicalise_i64(&self, buf: &mut Vec<u8>, info: IntInfo) {
        if info.bits == 0 || info.bits >= 64 {
            return;
        }
        if info.signed {
            let shift = 64 - info.bits;
            emit_instruction(buf, Op::I64Const(shift as i64));
            emit_instruction(buf, Op::I64Shl);
            emit_instruction(buf, Op::I64Const(shift as i64));
            emit_instruction(buf, Op::I64ShrS);
        } else {
            let mask = if info.bits == 64 {
                u64::MAX
            } else {
                ((1u128 << info.bits) - 1) as u64
            };
            emit_instruction(buf, Op::I64Const(mask as i64));
            emit_instruction(buf, Op::I64And);
        }
    }
}
