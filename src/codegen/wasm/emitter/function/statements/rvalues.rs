use crate::codegen::wasm::{
    RuntimeHook, STACK_POINTER_GLOBAL_INDEX, ValueType, ensure_u32, local_requires_memory, map_type,
};
use crate::error::Error;
use crate::mir::casts::{IntInfo, int_info};
use crate::mir::{
    BorrowKind, CastKind, ConstOperand, ConstValue, Operand, PendingOperand, PendingRvalue,
    PointerTy, ProjectionElem, Rvalue, Ty, TypeLayout, UnOp,
};
use crate::mir::{FloatValue, FloatWidth};
use crate::runtime::error::exception_type_identity;
use crate::support::float::{self, Float128Mode};
use crate::syntax::numeric::{NumericLiteralMetadata, NumericLiteralType};
use std::env;

use super::super::ops::{Op, emit_instruction};
use super::super::{FunctionEmitter, LocalRepresentation};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_rvalue(
        &mut self,
        buf: &mut Vec<u8>,
        value: &Rvalue,
    ) -> Result<ValueType, Error> {
        match value {
            Rvalue::Use(op) => self.emit_operand(buf, op),
            Rvalue::Unary {
                op,
                operand,
                rounding,
            } => {
                let _ = rounding;
                if let Some(signed) = self.operand_int128_signed(operand) {
                    return self.emit_int128_unary(buf, *op, operand, signed);
                }
                let ty = self.emit_operand(buf, operand)?;
                match op {
                    UnOp::Neg => match ty {
                        ValueType::I32 => {
                            emit_instruction(buf, Op::I32Const(-1));
                            emit_instruction(buf, Op::I32Mul);
                        }
                        ValueType::I64 => {
                            emit_instruction(buf, Op::I64Const(-1));
                            emit_instruction(buf, Op::I64Mul);
                        }
                        ValueType::F32 => {
                            emit_instruction(buf, Op::F32Const(-1.0));
                            emit_instruction(buf, Op::F32Mul);
                        }
                        ValueType::F64 => {
                            emit_instruction(buf, Op::F64Const(-1.0));
                            emit_instruction(buf, Op::F64Mul);
                        }
                    },
                    UnOp::UnaryPlus => return Ok(ty),
                    UnOp::Not => match ty {
                        ValueType::I32 => emit_instruction(buf, Op::I32Eqz),
                        ValueType::I64 => emit_instruction(buf, Op::I64Eqz),
                        other => {
                            return Err(Error::Codegen(format!(
                                "logical not is not supported for {:?} in WASM backend",
                                other
                            )));
                        }
                    },
                    UnOp::BitNot => match ty {
                        ValueType::I32 => {
                            emit_instruction(buf, Op::I32Const(-1));
                            emit_instruction(buf, Op::I32Xor);
                        }
                        ValueType::I64 => {
                            emit_instruction(buf, Op::I64Const(-1));
                            emit_instruction(buf, Op::I64Xor);
                        }
                        other => {
                            return Err(Error::Codegen(format!(
                                "ones-complement is not supported for {:?} in WASM backend",
                                other
                            )));
                        }
                    },
                    UnOp::Increment => match ty {
                        ValueType::I32 => {
                            emit_instruction(buf, Op::I32Const(1));
                            emit_instruction(buf, Op::I32Add);
                        }
                        ValueType::I64 => {
                            emit_instruction(buf, Op::I64Const(1));
                            emit_instruction(buf, Op::I64Add);
                        }
                        ValueType::F32 => {
                            emit_instruction(buf, Op::F32Const(1.0));
                            emit_instruction(buf, Op::F32Add);
                        }
                        ValueType::F64 => {
                            emit_instruction(buf, Op::F64Const(1.0));
                            emit_instruction(buf, Op::F64Add);
                        }
                    },
                    UnOp::Decrement => match ty {
                        ValueType::I32 => {
                            emit_instruction(buf, Op::I32Const(1));
                            emit_instruction(buf, Op::I32Sub);
                        }
                        ValueType::I64 => {
                            emit_instruction(buf, Op::I64Const(1));
                            emit_instruction(buf, Op::I64Sub);
                        }
                        ValueType::F32 => {
                            emit_instruction(buf, Op::F32Const(1.0));
                            emit_instruction(buf, Op::F32Sub);
                        }
                        ValueType::F64 => {
                            emit_instruction(buf, Op::F64Const(1.0));
                            emit_instruction(buf, Op::F64Sub);
                        }
                    },
                    UnOp::Deref => {
                        let operand_ty = self.operand_ty(operand).ok_or_else(|| {
                            Error::Codegen(
                                "unable to determine operand type for pointer dereference in WASM backend"
                                    .into(),
                            )
                        })?;
                        let referent = self.deref_target_ty(&operand_ty).ok_or_else(|| {
                            Error::Codegen(format!(
                                "cannot dereference non-pointer type `{}` in WASM backend (function `{}`)",
                                operand_ty.canonical_name(),
                                self.function.name
                            ))
                        })?;
                        let pointer_ty = self.emit_operand(buf, operand)?;
                        Self::ensure_operand_type(
                            pointer_ty,
                            ValueType::I32,
                            "pointer dereference",
                        )?;
                        if local_requires_memory(&referent, self.layouts) {
                            return Ok(ValueType::I32);
                        }
                        let value_ty = map_type(&referent);
                        emit_instruction(
                            buf,
                            match value_ty {
                                ValueType::I32 => Op::I32Load(0),
                                ValueType::I64 => Op::I64Load(0),
                                ValueType::F32 => Op::F32Load(0),
                                ValueType::F64 => Op::F64Load(0),
                            },
                        );
                        return Ok(value_ty);
                    }
                    UnOp::AddrOf | UnOp::AddrOfMut => {
                        return Err(Error::Codegen(
                            "address-of expressions should lower via Rvalue::AddressOf".into(),
                        ));
                    }
                }
                Ok(ty)
            }
            Rvalue::Binary {
                op,
                lhs: left,
                rhs: right,
                rounding,
            } => {
                let _ = rounding;
                let lhs_ty = self.operand_ty(left);
                let rhs_ty = self.operand_ty(right);
                let lhs_ty_hint = lhs_ty.as_ref().map(|ty| map_type(ty));
                let rhs_ty_hint = rhs_ty.as_ref().map(|ty| map_type(ty));
                if matches!(op, crate::mir::BinOp::Eq | crate::mir::BinOp::Ne) {
                    let lhs_is_const_str = matches!(left, Operand::Const(constant) if matches!(constant.value(), ConstValue::Str { .. }));
                    let rhs_is_const_str = matches!(right, Operand::Const(constant) if matches!(constant.value(), ConstValue::Str { .. }));
                    let lhs_is_null = matches!(left, Operand::Const(constant) if matches!(constant.value(), ConstValue::Null));
                    let rhs_is_null = matches!(right, Operand::Const(constant) if matches!(constant.value(), ConstValue::Null));

                    if lhs_is_null ^ rhs_is_null {
                        let (nullable_operand, nullable_ty) = if lhs_is_null {
                            (right, rhs_ty.as_ref())
                        } else {
                            (left, lhs_ty.as_ref())
                        };
                        if let Some(Ty::Nullable(inner)) = nullable_ty {
                            let nullable_ty = Ty::Nullable(inner.clone());
                            let (_, has_value_offset) =
                                self.resolve_field_by_name(&nullable_ty, None, "HasValue")?;
                            let ptr_ty = self.emit_operand(buf, nullable_operand)?;
                            Self::ensure_operand_type(
                                ptr_ty,
                                ValueType::I32,
                                "nullable null check",
                            )?;
                            emit_instruction(buf, Op::LocalSet(self.temp_local));

                            emit_instruction(buf, Op::LocalGet(self.temp_local));
                            let has_value_offset = ensure_u32(
                                has_value_offset,
                                "nullable HasValue offset exceeds wasm32 addressable range",
                            )?;
                            if has_value_offset != 0 {
                                emit_instruction(buf, Op::I32Const(has_value_offset as i32));
                                emit_instruction(buf, Op::I32Add);
                            }
                            emit_instruction(buf, Op::I32Load8U(0));
                            match op {
                                crate::mir::BinOp::Eq => {
                                    emit_instruction(buf, Op::I32Eqz);
                                }
                                crate::mir::BinOp::Ne => {
                                    emit_instruction(buf, Op::I32Eqz);
                                    emit_instruction(buf, Op::I32Eqz);
                                }
                                _ => {}
                            }
                            return Ok(ValueType::I32);
                        }
                    }
                    let lhs_is_string_like =
                        matches!(lhs_ty.as_ref(), Some(Ty::String | Ty::Str)) || lhs_is_const_str;
                    let rhs_is_string_like =
                        matches!(rhs_ty.as_ref(), Some(Ty::String | Ty::Str)) || rhs_is_const_str;

                    if (lhs_is_string_like && rhs_is_string_like)
                        || (lhs_is_string_like && rhs_is_null)
                        || (rhs_is_string_like && lhs_is_null)
                    {
                        return self.emit_string_like_equality(
                            buf,
                            *op,
                            left,
                            right,
                            lhs_ty.as_ref(),
                            rhs_ty.as_ref(),
                        );
                    }
                }
                let lhs_int128 = self.operand_int128_signed(left);
                let rhs_int128 = self.operand_int128_signed(right);
                if matches!(
                    op,
                    crate::mir::BinOp::Eq
                        | crate::mir::BinOp::Ne
                        | crate::mir::BinOp::Lt
                        | crate::mir::BinOp::Le
                        | crate::mir::BinOp::Gt
                        | crate::mir::BinOp::Ge
                ) && (lhs_int128.is_some() || rhs_int128.is_some())
                {
                    let signed = lhs_int128.or(rhs_int128).unwrap_or(true);
                    return self.emit_int128_comparison(buf, *op, left, right, signed);
                }
                if (lhs_int128.is_some() || rhs_int128.is_some())
                    && matches!(
                        op,
                        crate::mir::BinOp::Add
                            | crate::mir::BinOp::Sub
                            | crate::mir::BinOp::Mul
                            | crate::mir::BinOp::Div
                            | crate::mir::BinOp::Rem
                            | crate::mir::BinOp::BitAnd
                            | crate::mir::BinOp::BitOr
                            | crate::mir::BinOp::BitXor
                            | crate::mir::BinOp::Shl
                            | crate::mir::BinOp::Shr
                    )
                {
                    let signed = lhs_int128.or(rhs_int128).unwrap_or(true);
                    return self.emit_int128_binary(buf, *op, left, right, signed);
                }
                let lhs_float = self.operand_float_ty(left);
                let rhs_float = self.operand_float_ty(right);
                let hinted_value_ty = lhs_ty_hint.or(rhs_ty_hint).unwrap_or(ValueType::I32);
                let is_float = lhs_float.is_some()
                    || rhs_float.is_some()
                    || matches!(hinted_value_ty, ValueType::F32 | ValueType::F64);
                if is_float {
                    let lhs_value_ty = self.emit_operand(buf, left)?;
                    let rhs_value_ty = self.emit_operand(buf, right)?;
                    let float_ty = if matches!(hinted_value_ty, ValueType::F64)
                        || matches!(lhs_value_ty, ValueType::F64)
                        || matches!(rhs_value_ty, ValueType::F64)
                        || matches!(lhs_float, Some(ValueType::F64))
                        || matches!(rhs_float, Some(ValueType::F64))
                    {
                        ValueType::F64
                    } else {
                        ValueType::F32
                    };
                    if matches!(op, crate::mir::BinOp::Rem) {
                        let hook = match float_ty {
                            ValueType::F32 => RuntimeHook::F32Rem,
                            ValueType::F64 => RuntimeHook::F64Rem,
                            other => {
                                return Err(Error::Codegen(format!(
                                    "float remainder unsupported for {:?} in WASM backend (func={}, lhs={:?}, rhs={:?}, value_ty={:?})",
                                    other,
                                    self.function.name,
                                    lhs_value_ty,
                                    rhs_value_ty,
                                    hinted_value_ty
                                )));
                            }
                        };
                        let call = self.runtime_hook_index(hook)?;
                        emit_instruction(buf, Op::Call(call));
                        return Ok(float_ty);
                    }
                    let op_code = Op::from_float_bin_op(*op, float_ty).ok_or_else(|| {
                        Error::Codegen(format!(
                            "unsupported float binary op {:?} in WASM backend (func={}, lhs={:?}, rhs={:?}, value_ty={:?})",
                            op,
                            self.function.name,
                            lhs_value_ty,
                            rhs_value_ty,
                            hinted_value_ty
                        ))
                    })?;
                    emit_instruction(buf, op_code);
                    let result_ty = if matches!(
                        op,
                        crate::mir::BinOp::Eq
                            | crate::mir::BinOp::Ne
                            | crate::mir::BinOp::Lt
                            | crate::mir::BinOp::Le
                            | crate::mir::BinOp::Gt
                            | crate::mir::BinOp::Ge
                    ) {
                        ValueType::I32
                    } else {
                        float_ty
                    };
                    return Ok(result_ty);
                }
                let lhs_info = self.operand_int_info(left);
                let rhs_info = self.operand_int_info(right);
                let value_info = match (lhs_info, rhs_info) {
                    (Some(lhs), Some(rhs)) => {
                        if lhs.bits >= rhs.bits {
                            lhs
                        } else {
                            rhs
                        }
                    }
                    (Some(info), None) | (None, Some(info)) => info,
                    (None, None) => IntInfo {
                        bits: 32,
                        signed: true,
                    },
                };
                let value_ty = if value_info.bits > 32 {
                    ValueType::I64
                } else {
                    ValueType::I32
                };
                let signed = value_info.signed;
                let lhs_signed = lhs_info.map(|info| info.signed).unwrap_or(signed);
                let rhs_signed = rhs_info.map(|info| info.signed).unwrap_or(signed);

                let lhs_value_ty = self.emit_operand(buf, left)?;
                match (value_ty, lhs_value_ty) {
                    (ValueType::I64, ValueType::I32) => {
                        emit_instruction(
                            buf,
                            if lhs_signed {
                                Op::I64ExtendI32S
                            } else {
                                Op::I64ExtendI32U
                            },
                        );
                    }
                    (ValueType::I32, ValueType::I64) => {
                        emit_instruction(buf, Op::I32WrapI64);
                    }
                    _ => {}
                }

                let rhs_value_ty = self.emit_operand(buf, right)?;
                match (value_ty, rhs_value_ty) {
                    (ValueType::I64, ValueType::I32) => {
                        emit_instruction(
                            buf,
                            if rhs_signed {
                                Op::I64ExtendI32S
                            } else {
                                Op::I64ExtendI32U
                            },
                        );
                    }
                    (ValueType::I32, ValueType::I64) => {
                        emit_instruction(buf, Op::I32WrapI64);
                    }
                    _ => {}
                }

                let op_code = Op::from_int_bin_op(*op, value_ty, signed).ok_or_else(|| {
                    Error::Codegen(format!(
                        "unsupported integer binary op {:?} in WASM backend (func={}, lhs={:?}, rhs={:?}, value_ty={:?})",
                        op,
                        self.function.name,
                        lhs_value_ty,
                        rhs_value_ty,
                        value_ty
                    ))
                })?;
                emit_instruction(buf, op_code);
                let result_ty = if matches!(
                    op,
                    crate::mir::BinOp::Eq
                        | crate::mir::BinOp::Ne
                        | crate::mir::BinOp::Lt
                        | crate::mir::BinOp::Le
                        | crate::mir::BinOp::Gt
                        | crate::mir::BinOp::Ge
                ) {
                    ValueType::I32
                } else {
                    value_ty
                };
                Ok(result_ty)
            }
            Rvalue::Cast {
                kind,
                operand,
                source,
                target,
                ..
            } => self.emit_cast(buf, *kind, operand, source, target),
            Rvalue::Len(place) => self.emit_len_rvalue(buf, place),
            Rvalue::AddressOf { place, .. } => {
                let access = self.resolve_memory_access(place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(ValueType::I32)
            }
            Rvalue::Pending(PendingRvalue { repr, .. }) => {
                if env::var_os("CHIC_DEBUG_PENDING").is_some() {
                    eprintln!("[pending-rvalue] repr=`{repr}` func={}", self.function.name);
                }
                emit_instruction(buf, Op::I32Const(0));
                Ok(ValueType::I32)
            }
            Rvalue::AtomicLoad { target, order } => self.emit_atomic_load(buf, target, *order),
            Rvalue::AtomicRmw {
                op,
                target,
                value,
                order,
            } => self.emit_atomic_rmw(buf, *op, target, value, *order),
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                success,
                failure,
                weak,
            } => {
                let _ = weak;
                self.emit_atomic_compare_exchange(
                    buf, target, expected, desired, *success, *failure,
                )
            }
            Rvalue::StaticLoad { id } => self.emit_static_load(buf, *id),
            Rvalue::StaticRef { id } => self.emit_static_ref(buf, *id),
            other => Err(Error::Codegen(format!(
                "WASM backend cannot lower rvalue: {other:?}"
            ))),
        }
    }

    fn emit_cast(
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

    fn emit_integer_cast(
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

    fn canonicalise_i32(&self, buf: &mut Vec<u8>, info: IntInfo) {
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

    fn canonicalise_i64(&self, buf: &mut Vec<u8>, info: IntInfo) {
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

    pub(crate) fn emit_operand(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
    ) -> Result<ValueType, Error> {
        wasm_debug!("        emit_operand {:?}", operand);
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        if let Some(ty) = self.local_tys.get(place.local.0) {
                            let base_ty = self.resolve_self_ty(ty);
                            if local_requires_memory(&base_ty, self.layouts) {
                                let access = self.resolve_memory_access(place)?;
                                self.emit_pointer_expression(buf, &access)?;
                                return Ok(ValueType::I32);
                            }
                        }
                    }
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(
                            LocalRepresentation::PointerParam | LocalRepresentation::FrameAllocated
                        )
                    ) {
                        if let Some(ty) = self.local_tys.get(place.local.0) {
                            if matches!(
                                ty,
                                Ty::Fn(fn_ty) if !matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
                            ) || self.ty_is_trait_object_like(ty)
                            {
                                if let Some(index) = self.local_index(place.local) {
                                    if std::env::var_os("CHIC_DEBUG_WASM_FN_OPERAND").is_some() {
                                        eprintln!(
                                            "[wasm-fn-op] func={} local={} wasm_index={}",
                                            self.function.name, place.local.0, index
                                        );
                                    }
                                    emit_instruction(buf, Op::LocalGet(index));
                                    return Ok(ValueType::I32);
                                }
                            }
                            if self.ty_is_reference(ty) {
                                let access = self.resolve_memory_access(place)?;
                                self.emit_pointer_expression(buf, &access)?;
                                if matches!(
                                    self.representations[place.local.0],
                                    LocalRepresentation::FrameAllocated
                                ) && !access.load_pointer_from_slot
                                {
                                    emit_instruction(buf, Op::I32Load(0));
                                }
                                return Ok(ValueType::I32);
                            }
                        }
                        return self.emit_load_from_place(buf, place);
                    }
                    if let Some(index) = self.local_index(place.local) {
                        emit_instruction(buf, Op::LocalGet(index));
                        Ok(map_type(&self.local_tys[place.local.0]))
                    } else {
                        emit_instruction(buf, Op::I32Const(0));
                        Ok(ValueType::I32)
                    }
                } else {
                    if matches!(
                        self.representations.get(place.local.0),
                        Some(LocalRepresentation::Scalar)
                    ) {
                        let base_ty = self.resolve_self_ty(&self.local_tys[place.local.0]);
                        if matches!(base_ty, Ty::Str) && place.projection.len() == 1 {
                            if let Some(index) = self.local_index(place.local) {
                                let field = &place.projection[0];
                                let key = match field {
                                    ProjectionElem::Field(0) => Some("ptr"),
                                    ProjectionElem::Field(1) => Some("len"),
                                    ProjectionElem::FieldNamed(name) => {
                                        let lowered = name.to_ascii_lowercase();
                                        if lowered == "ptr" || lowered == "pointer" {
                                            Some("ptr")
                                        } else if lowered == "len" || lowered == "length" {
                                            Some("len")
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };

                                if let Some(key) = key {
                                    emit_instruction(buf, Op::LocalGet(index));
                                    match key {
                                        "ptr" => {
                                            emit_instruction(buf, Op::I32WrapI64);
                                        }
                                        "len" => {
                                            emit_instruction(buf, Op::I64Const(32));
                                            emit_instruction(buf, Op::I64ShrU);
                                            emit_instruction(buf, Op::I32WrapI64);
                                        }
                                        _ => {}
                                    }
                                    return Ok(ValueType::I32);
                                }
                            }
                        }
                    }
                    if matches!(
                        self.representations[place.local.0],
                        LocalRepresentation::Scalar
                    ) && !self.ty_is_reference(&self.local_tys[place.local.0])
                    {
                        let has_deref = place
                            .projection
                            .iter()
                            .any(|elem| matches!(elem, ProjectionElem::Deref));
                        if !has_deref {
                            if let Some(index) = self.local_index(place.local) {
                                let base_ty = self.resolve_self_ty(&self.local_tys[place.local.0]);
                                if let Ok(plan) =
                                    self.compute_projection_offset(&base_ty, &place.projection)
                                {
                                    if plan.vec_index.is_none() {
                                        emit_instruction(buf, Op::LocalGet(index));
                                        return Ok(map_type(&plan.value_ty));
                                    }
                                }
                            }
                        }
                    }
                    self.emit_load_from_place(buf, place)
                }
            }
            Operand::Const(constant) => self.emit_const_operand(buf, constant),
            Operand::Borrow(borrow) => {
                let access = self.resolve_memory_access(&borrow.place)?;
                self.emit_pointer_expression(buf, &access)?;
                Ok(ValueType::I32)
            }
            Operand::Mmio(spec) => self.emit_mmio_read(buf, spec),
            Operand::Pending(PendingOperand { repr, .. }) => {
                if env::var_os("CHIC_DEBUG_PENDING").is_some() {
                    eprintln!(
                        "[pending-operand] repr=`{repr}` func={}",
                        self.function.name
                    );
                }
                emit_instruction(buf, Op::I32Const(0));
                Ok(ValueType::I32)
            }
        }
    }

    pub(crate) fn operand_ty(&self, operand: &Operand) -> Option<Ty> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let base_ty = self.local_tys.get(place.local.0)?.clone();
                Some(
                    self.compute_projection_offset(&base_ty, &place.projection)
                        .ok()?
                        .value_ty,
                )
            }
            Operand::Borrow(borrow) => {
                let base_ty = self.local_tys.get(borrow.place.local.0)?.clone();
                let ty = self
                    .compute_projection_offset(&base_ty, &borrow.place.projection)
                    .ok()?
                    .value_ty;
                let mutable = borrow.kind != BorrowKind::Shared;
                Some(Ty::Pointer(Box::new(PointerTy::new(ty, mutable))))
            }
            Operand::Mmio(spec) => Some(spec.ty.clone()),
            Operand::Const(_) | Operand::Pending(_) => None,
        }
    }

    fn deref_target_ty(&self, pointer: &Ty) -> Option<Ty> {
        match pointer {
            Ty::Pointer(inner) => Some(inner.element.clone()),
            Ty::Ref(inner) => Some(inner.element.clone()),
            Ty::Nullable(inner) => self.deref_target_ty(inner),
            _ => None,
        }
    }

    fn operand_float_ty(&self, operand: &Operand) -> Option<ValueType> {
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

    fn operand_int_info(&self, operand: &Operand) -> Option<IntInfo> {
        match operand {
            Operand::Const(constant) => self.const_int_info(constant),
            _ => self
                .operand_ty(operand)
                .and_then(|ty| self.int_info_for_ty(&ty)),
        }
    }

    fn operand_int128_signed(&self, operand: &Operand) -> Option<bool> {
        self.operand_int_info(operand)
            .filter(|info| info.bits > 64)
            .map(|info| info.signed)
    }

    fn int_info_for_ty(&self, ty: &Ty) -> Option<IntInfo> {
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

    fn const_int_info(&self, constant: &ConstOperand) -> Option<IntInfo> {
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

    fn emit_const_operand(
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

    fn emit_signed_int_literal(
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

    fn emit_unsigned_int_literal(
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

    fn emit_float_literal(
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

    fn ensure_supported_int_width(&self, bits: u32) -> Result<(), Error> {
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

    fn ensure_signed_range(&self, value: i128, bits: u32) -> Result<(), Error> {
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

    fn ensure_unsigned_range(&self, value: u128, bits: u32) -> Result<(), Error> {
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

    fn emit_int128_comparison(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
        self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
        let hook = if signed {
            RuntimeHook::I128Cmp
        } else {
            RuntimeHook::U128Cmp
        };
        let call = self.runtime_hook_index(hook)?;
        emit_instruction(buf, Op::LocalGet(self.block_local));
        emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
        emit_instruction(buf, Op::Call(call));
        match op {
            crate::mir::BinOp::Eq => {
                emit_instruction(buf, Op::I32Eqz);
            }
            crate::mir::BinOp::Ne => {
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::I32Eqz);
            }
            crate::mir::BinOp::Lt => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32LtS);
            }
            crate::mir::BinOp::Le => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32LeS);
            }
            crate::mir::BinOp::Gt => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32GtS);
            }
            crate::mir::BinOp::Ge => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::I32GeS);
            }
            _ => {
                return Err(Error::Codegen(
                    "unsupported int128 comparison operator in WASM backend".into(),
                ));
            }
        }
        Ok(ValueType::I32)
    }

    fn emit_string_like_equality(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        lhs_ty: Option<&Ty>,
        rhs_ty: Option<&Ty>,
    ) -> Result<ValueType, Error> {
        if !matches!(op, crate::mir::BinOp::Eq | crate::mir::BinOp::Ne) {
            return Err(Error::Codegen(
                "string equality helper invoked for non-equality operator".into(),
            ));
        }

        let left_ptr_local = self.block_local;
        let right_ptr_local = self.temp_local;
        let remaining_len_local = self.scratch_local;
        let result_local = self.stack_temp_local;

        // Temporarily stores the right length while we compare lengths.
        let right_len_tmp_local = result_local;

        // compare_done:
        emit_instruction(buf, Op::Block);

        // Null handling: string equality must treat null as distinct from empty.
        // Use `block_local`/`temp_local` to store null flags, then reuse them for ptr locals.
        let left_is_null_local = left_ptr_local;
        let right_is_null_local = right_ptr_local;
        match lhs_ty {
            Some(Ty::Str) => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::LocalSet(left_is_null_local));
            }
            Some(Ty::String) => {
                let value_ty = self.emit_operand(buf, lhs)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string null check")?;
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::LocalSet(left_is_null_local));
            }
            _ => match lhs {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Null => {
                        emit_instruction(buf, Op::I32Const(1));
                        emit_instruction(buf, Op::LocalSet(left_is_null_local));
                    }
                    ConstValue::Str { .. } => {
                        emit_instruction(buf, Op::I32Const(0));
                        emit_instruction(buf, Op::LocalSet(left_is_null_local));
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported LHS operand for string equality in WASM backend".into(),
                        ));
                    }
                },
                _ => {
                    return Err(Error::Codegen(
                        "unsupported LHS operand for string equality in WASM backend".into(),
                    ));
                }
            },
        }

        match rhs_ty {
            Some(Ty::Str) => {
                emit_instruction(buf, Op::I32Const(0));
                emit_instruction(buf, Op::LocalSet(right_is_null_local));
            }
            Some(Ty::String) => {
                let value_ty = self.emit_operand(buf, rhs)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string null check")?;
                emit_instruction(buf, Op::I32Eqz);
                emit_instruction(buf, Op::LocalSet(right_is_null_local));
            }
            _ => match rhs {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Null => {
                        emit_instruction(buf, Op::I32Const(1));
                        emit_instruction(buf, Op::LocalSet(right_is_null_local));
                    }
                    ConstValue::Str { .. } => {
                        emit_instruction(buf, Op::I32Const(0));
                        emit_instruction(buf, Op::LocalSet(right_is_null_local));
                    }
                    _ => {
                        return Err(Error::Codegen(
                            "unsupported RHS operand for string equality in WASM backend".into(),
                        ));
                    }
                },
                _ => {
                    return Err(Error::Codegen(
                        "unsupported RHS operand for string equality in WASM backend".into(),
                    ));
                }
            },
        }

        // If both null -> result = 1; exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_is_null_local));
        emit_instruction(buf, Op::LocalGet(right_is_null_local));
        emit_instruction(buf, Op::I32And);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(0));
        emit_instruction(buf, Op::End);

        // If either null -> result = 0; exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_is_null_local));
        emit_instruction(buf, Op::LocalGet(right_is_null_local));
        emit_instruction(buf, Op::I32Or);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(0));
        emit_instruction(buf, Op::End);

        self.emit_string_like_slice_ptr_len(buf, lhs, lhs_ty, left_ptr_local, remaining_len_local)?;
        self.emit_string_like_slice_ptr_len(
            buf,
            rhs,
            rhs_ty,
            right_ptr_local,
            right_len_tmp_local,
        )?;

        // If lengths differ -> result = 0; exit compare_done.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::LocalGet(right_len_tmp_local));
        emit_instruction(buf, Op::I32Ne);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(1));
        emit_instruction(buf, Op::End);

        // If length == 0 -> result = 1; exit compare_done.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(1));
        emit_instruction(buf, Op::End);

        // Default to equal unless we find a mismatch.
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::LocalSet(result_local));

        // loop_exit:
        emit_instruction(buf, Op::Block);
        emit_instruction(buf, Op::Loop);

        // Compare current bytes; on mismatch set result=0 and exit compare_done.
        emit_instruction(buf, Op::LocalGet(left_ptr_local));
        emit_instruction(buf, Op::I32Load8U(0));
        emit_instruction(buf, Op::LocalGet(right_ptr_local));
        emit_instruction(buf, Op::I32Load8U(0));
        emit_instruction(buf, Op::I32Ne);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::I32Const(0));
        emit_instruction(buf, Op::LocalSet(result_local));
        emit_instruction(buf, Op::Br(2));
        emit_instruction(buf, Op::End);

        // Advance pointers and decrement remaining length.
        emit_instruction(buf, Op::LocalGet(left_ptr_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(left_ptr_local));
        emit_instruction(buf, Op::LocalGet(right_ptr_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Add);
        emit_instruction(buf, Op::LocalSet(right_ptr_local));

        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Const(1));
        emit_instruction(buf, Op::I32Sub);
        emit_instruction(buf, Op::LocalSet(remaining_len_local));

        // If remaining == 0 then exit loop_exit; otherwise continue loop.
        emit_instruction(buf, Op::LocalGet(remaining_len_local));
        emit_instruction(buf, Op::I32Eqz);
        emit_instruction(buf, Op::If);
        emit_instruction(buf, Op::Br(2));
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::Br(0));

        // end loop / loop_exit / compare_done
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::End);
        emit_instruction(buf, Op::End);

        emit_instruction(buf, Op::LocalGet(result_local));
        if matches!(op, crate::mir::BinOp::Ne) {
            emit_instruction(buf, Op::I32Eqz);
        }
        Ok(ValueType::I32)
    }

    fn emit_string_like_slice_ptr_len(
        &mut self,
        buf: &mut Vec<u8>,
        operand: &Operand,
        operand_ty: Option<&Ty>,
        out_ptr_local: u32,
        out_len_local: u32,
    ) -> Result<(), Error> {
        let ty = operand_ty
            .cloned()
            .or_else(|| self.operand_ty(operand))
            .or_else(|| match operand {
                Operand::Const(constant) => match constant.value() {
                    ConstValue::Str { .. } => Some(Ty::Str),
                    ConstValue::Null => Some(Ty::String),
                    _ => None,
                },
                _ => None,
            })
            .ok_or_else(|| {
                Error::Codegen(
                    "unable to determine operand type for string equality in WASM backend".into(),
                )
            })?;

        match ty {
            Ty::String => {
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I32, "string equality")?;
                emit_instruction(buf, Op::LocalSet(out_ptr_local));
                emit_instruction(buf, Op::LocalGet(out_ptr_local));
                let hook = self.runtime_hook_index(RuntimeHook::StringAsSlice)?;
                emit_instruction(buf, Op::Call(hook));
                emit_instruction(buf, Op::LocalSet(out_len_local));
                emit_instruction(buf, Op::LocalSet(out_ptr_local));
                Ok(())
            }
            Ty::Str => {
                let value_ty = self.emit_operand(buf, operand)?;
                Self::ensure_operand_type(value_ty, ValueType::I64, "str equality")?;
                emit_instruction(buf, Op::LocalSet(self.wide_temp_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::LocalSet(out_ptr_local));

                emit_instruction(buf, Op::LocalGet(self.wide_temp_local));
                emit_instruction(buf, Op::I64Const(32));
                emit_instruction(buf, Op::I64ShrU);
                emit_instruction(buf, Op::I32WrapI64);
                emit_instruction(buf, Op::LocalSet(out_len_local));
                Ok(())
            }
            other => Err(Error::Codegen(format!(
                "unsupported operand type {:?} for string equality in WASM backend",
                other
            ))),
        }
    }

    fn emit_int128_unary(
        &mut self,
        buf: &mut Vec<u8>,
        op: UnOp,
        operand: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        match op {
            UnOp::UnaryPlus => return self.emit_operand(buf, operand),
            UnOp::Neg if signed => {
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let call = self.runtime_hook_index(RuntimeHook::I128Neg)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            UnOp::BitNot => {
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = if signed {
                    RuntimeHook::I128Not
                } else {
                    RuntimeHook::U128Not
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            UnOp::Increment | UnOp::Decrement => {
                let one = Operand::Const(ConstOperand::new(ConstValue::Int(1)));
                self.materialize_int128_operand(buf, operand, signed, self.block_local)?;
                self.materialize_int128_operand(buf, &one, signed, self.stack_temp_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    UnOp::Increment => {
                        if signed {
                            RuntimeHook::I128Add
                        } else {
                            RuntimeHook::U128Add
                        }
                    }
                    UnOp::Decrement => {
                        if signed {
                            RuntimeHook::I128Sub
                        } else {
                            RuntimeHook::U128Sub
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            _ => {}
        }
        Err(Error::Codegen(format!(
            "unsupported int128 unary op {:?} in WASM backend (func={})",
            op, self.function.name
        )))
    }

    fn emit_int128_binary(
        &mut self,
        buf: &mut Vec<u8>,
        op: crate::mir::BinOp,
        lhs: &Operand,
        rhs: &Operand,
        signed: bool,
    ) -> Result<ValueType, Error> {
        match op {
            crate::mir::BinOp::Add
            | crate::mir::BinOp::Sub
            | crate::mir::BinOp::Mul
            | crate::mir::BinOp::Div
            | crate::mir::BinOp::Rem
            | crate::mir::BinOp::BitAnd
            | crate::mir::BinOp::BitOr
            | crate::mir::BinOp::BitXor => {
                self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                self.materialize_int128_operand(buf, rhs, signed, self.stack_temp_local)?;
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    crate::mir::BinOp::Add => {
                        if signed {
                            RuntimeHook::I128Add
                        } else {
                            RuntimeHook::U128Add
                        }
                    }
                    crate::mir::BinOp::Sub => {
                        if signed {
                            RuntimeHook::I128Sub
                        } else {
                            RuntimeHook::U128Sub
                        }
                    }
                    crate::mir::BinOp::Mul => {
                        if signed {
                            RuntimeHook::I128Mul
                        } else {
                            RuntimeHook::U128Mul
                        }
                    }
                    crate::mir::BinOp::Div => {
                        if signed {
                            RuntimeHook::I128Div
                        } else {
                            RuntimeHook::U128Div
                        }
                    }
                    crate::mir::BinOp::Rem => {
                        if signed {
                            RuntimeHook::I128Rem
                        } else {
                            RuntimeHook::U128Rem
                        }
                    }
                    crate::mir::BinOp::BitAnd => {
                        if signed {
                            RuntimeHook::I128And
                        } else {
                            RuntimeHook::U128And
                        }
                    }
                    crate::mir::BinOp::BitOr => {
                        if signed {
                            RuntimeHook::I128Or
                        } else {
                            RuntimeHook::U128Or
                        }
                    }
                    crate::mir::BinOp::BitXor => {
                        if signed {
                            RuntimeHook::I128Xor
                        } else {
                            RuntimeHook::U128Xor
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            crate::mir::BinOp::Shl | crate::mir::BinOp::Shr => {
                self.materialize_int128_operand(buf, lhs, signed, self.block_local)?;
                let amount_ty = self.emit_operand(buf, rhs)?;
                Self::ensure_operand_type(amount_ty, ValueType::I32, "int128 shift")?;
                emit_instruction(buf, Op::LocalSet(self.stack_temp_local));
                self.allocate_int128_temp(buf, 0, 0, self.temp_local)?;
                let hook = match op {
                    crate::mir::BinOp::Shl => {
                        if signed {
                            RuntimeHook::I128Shl
                        } else {
                            RuntimeHook::U128Shl
                        }
                    }
                    crate::mir::BinOp::Shr => {
                        if signed {
                            RuntimeHook::I128Shr
                        } else {
                            RuntimeHook::U128Shr
                        }
                    }
                    _ => unreachable!(),
                };
                let call = self.runtime_hook_index(hook)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, Op::LocalGet(self.block_local));
                emit_instruction(buf, Op::LocalGet(self.stack_temp_local));
                emit_instruction(buf, Op::Call(call));
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                return Ok(ValueType::I32);
            }
            _ => {}
        }
        Err(Error::Codegen(format!(
            "unsupported int128 binary op {:?} in WASM backend (func={})",
            op, self.function.name
        )))
    }

    pub(crate) fn pointer_width_bits(&self) -> u32 {
        32
    }

    pub(crate) fn emit_throw(
        &mut self,
        buf: &mut Vec<u8>,
        exception: &Option<Operand>,
        ty: &Option<Ty>,
    ) -> Result<(), Error> {
        let treat_as_reference = ty
            .as_ref()
            .map(|throw_ty| self.ty_is_reference(throw_ty) || matches!(throw_ty, Ty::Named(_)))
            .unwrap_or(false);
        if std::env::var_os("CHIC_DEBUG_WASM_THROW_LOWERING").is_some() {
            eprintln!(
                "[wasm-throw-lower] func={} ty={} operand={:?}",
                self.function.name,
                ty.as_ref()
                    .map(|ty| ty.canonical_name())
                    .unwrap_or_else(|| "<none>".to_string()),
                exception
            );
        }
        if let Some(value) = exception {
            let value_ty = if treat_as_reference {
                if let Operand::Copy(place) | Operand::Move(place) = value {
                    let access = self.resolve_memory_access(place)?;
                    let pointer_in_slot = access.vec_index.is_none()
                        && access.offset == 0
                        && (access.load_pointer_from_slot
                            || matches!(access.value_ty, Ty::Named(_))
                            || (self.ty_is_reference(&access.value_ty)
                                && matches!(
                                    self.representations[place.local.0],
                                    LocalRepresentation::PointerParam
                                )));
                    if std::env::var_os("CHIC_DEBUG_WASM_THROW_LOWERING").is_some() {
                        eprintln!(
                            "[wasm-throw-lower] place local={} load_slot={} offset={} vec={:?} ptr_in_slot={}",
                            place.local.0,
                            access.load_pointer_from_slot,
                            access.offset,
                            access.vec_index,
                            pointer_in_slot
                        );
                    }
                    self.emit_pointer_expression(buf, &access)?;
                    if !pointer_in_slot {
                        emit_instruction(buf, Op::I32Load(0));
                    }
                    ValueType::I32
                } else {
                    self.emit_operand(buf, value)?
                }
            } else {
                self.emit_operand(buf, value)?
            };
            match value_ty {
                ValueType::I32 => {}
                ValueType::I64 => emit_instruction(buf, Op::I32WrapI64),
                other => {
                    return Err(Error::Codegen(format!(
                        "throw operand type {:?} is not supported by the WASM backend",
                        other
                    )));
                }
            }
        } else {
            emit_instruction(buf, Op::I32Const(0));
        }

        let type_id = ty
            .as_ref()
            .map(|ty| exception_type_identity(&ty.canonical_name()))
            .unwrap_or(0);
        emit_instruction(buf, Op::I64Const(type_id as i64));
        let hook = self.runtime_hook_index(RuntimeHook::Throw)?;
        emit_instruction(buf, Op::Call(hook));
        // Treat throw as non-returning: tear down the current frame and branch to the
        // function epilogue so callers can observe the pending exception state.
        self.emit_frame_teardown(buf);
        emit_instruction(buf, Op::Br(2));
        Ok(())
    }

    pub(crate) fn const_to_op(value: &ConstValue) -> Result<Op, Error> {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => {
                i32::try_from(*v).map(Op::I32Const).map_err(|_| {
                    Error::Codegen("integer literal exceeds 32-bit range in WASM backend".into())
                })
            }
            ConstValue::UInt(v) => i32::try_from(*v).map(Op::I32Const).map_err(|_| {
                Error::Codegen(
                    "unsigned integer literal exceeds 32-bit range in WASM backend".into(),
                )
            }),
            ConstValue::Char(c) => Ok(Op::I32Const(*c as i32)),
            ConstValue::Bool(b) => Ok(Op::I32Const(i32::from(*b))),
            ConstValue::Null => Ok(Op::I32Const(0)),
            _ => Err(Error::Codegen(
                "only integral literals are supported by the WASM backend".into(),
            )),
        }
    }
}
