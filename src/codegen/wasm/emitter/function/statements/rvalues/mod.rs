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
mod casts;
mod consts;
mod int128;
mod operands;
mod strings;
mod throw;
mod ty_info;

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
}
