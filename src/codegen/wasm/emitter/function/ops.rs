use crate::codegen::wasm::{
    ValueType, push_atomic_fence, push_atomic_memory, push_block_like, push_br, push_byte,
    push_call, push_call_indirect, push_f32_const, push_f64_const, push_global, push_i32_const,
    push_i64_const, push_local, push_memory,
};
use crate::mir::BinOp;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Op {
    Block,
    Loop,
    End,
    If,
    Else,
    Br(u32),
    Return,
    Unreachable,
    Drop,
    Call(u32),
    CallIndirect {
        type_index: u32,
        table_index: u32,
    },
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Eq,
    I32Ne,
    I32Eqz,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    I32LtS,
    I32LeS,
    I32GtS,
    I32GeS,
    I32LtU,
    I32LeU,
    I32GtU,
    I32GeU,
    I32WrapI64,
    I64ExtendI32S,
    I64ExtendI32U,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    I64LtS,
    I64LeS,
    I64GtS,
    I64LtU,
    I64LeU,
    I64GtU,
    I64GeS,
    I64GeU,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    #[allow(dead_code)]
    F32Neg,
    #[allow(dead_code)]
    F32Trunc,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    #[allow(dead_code)]
    F64Neg,
    #[allow(dead_code)]
    F64Trunc,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F64PromoteF32,
    F32DemoteF64,
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    I32Load(u32),
    I32Load8S(u32),
    I32Load8U(u32),
    I32Load16S(u32),
    I32Load16U(u32),
    I32Store(u32),
    #[allow(dead_code)]
    I32Store8(u32),
    #[allow(dead_code)]
    I32Store16(u32),
    I64Load(u32),
    I64Store(u32),
    F32Load(u32),
    F32Store(u32),
    F64Load(u32),
    F64Store(u32),
    I32AtomicLoad(u32),
    I64AtomicLoad(u32),
    I32AtomicStore(u32),
    I64AtomicStore(u32),
    I32AtomicRmwAdd(u32),
    I64AtomicRmwAdd(u32),
    I32AtomicRmwSub(u32),
    I64AtomicRmwSub(u32),
    I32AtomicRmwAnd(u32),
    I64AtomicRmwAnd(u32),
    I32AtomicRmwOr(u32),
    I64AtomicRmwOr(u32),
    I32AtomicRmwXor(u32),
    I64AtomicRmwXor(u32),
    I32AtomicRmwXchg(u32),
    I64AtomicRmwXchg(u32),
    I32AtomicRmwCmpxchg(u32),
    I64AtomicRmwCmpxchg(u32),
    I32AtomicRmwMinS(u32),
    I64AtomicRmwMinS(u32),
    I32AtomicRmwMaxS(u32),
    I64AtomicRmwMaxS(u32),
    AtomicFence,
    MemoryFill,
}

impl Op {
    pub(crate) fn from_int_bin_op(op: BinOp, value_ty: ValueType, signed: bool) -> Option<Self> {
        match value_ty {
            ValueType::I32 => Some(match op {
                BinOp::Add => Op::I32Add,
                BinOp::Sub => Op::I32Sub,
                BinOp::Mul => Op::I32Mul,
                BinOp::Div => {
                    if signed {
                        Op::I32DivS
                    } else {
                        Op::I32DivU
                    }
                }
                BinOp::Rem => {
                    if signed {
                        Op::I32RemS
                    } else {
                        Op::I32RemU
                    }
                }
                BinOp::BitAnd | BinOp::And => Op::I32And,
                BinOp::BitOr | BinOp::Or => Op::I32Or,
                BinOp::BitXor => Op::I32Xor,
                BinOp::Shl => Op::I32Shl,
                BinOp::Shr => {
                    if signed {
                        Op::I32ShrS
                    } else {
                        Op::I32ShrU
                    }
                }
                BinOp::Eq => Op::I32Eq,
                BinOp::Ne => Op::I32Ne,
                BinOp::Lt => {
                    if signed {
                        Op::I32LtS
                    } else {
                        Op::I32LtU
                    }
                }
                BinOp::Le => {
                    if signed {
                        Op::I32LeS
                    } else {
                        Op::I32LeU
                    }
                }
                BinOp::Gt => {
                    if signed {
                        Op::I32GtS
                    } else {
                        Op::I32GtU
                    }
                }
                BinOp::Ge => {
                    if signed {
                        Op::I32GeS
                    } else {
                        Op::I32GeU
                    }
                }
                BinOp::NullCoalesce => return None,
            }),
            ValueType::I64 => Some(match op {
                BinOp::Add => Op::I64Add,
                BinOp::Sub => Op::I64Sub,
                BinOp::Mul => Op::I64Mul,
                BinOp::Div => {
                    if signed {
                        Op::I64DivS
                    } else {
                        Op::I64DivU
                    }
                }
                BinOp::Rem => {
                    if signed {
                        Op::I64RemS
                    } else {
                        Op::I64RemU
                    }
                }
                BinOp::BitAnd | BinOp::And => Op::I64And,
                BinOp::BitOr | BinOp::Or => Op::I64Or,
                BinOp::BitXor => Op::I64Xor,
                BinOp::Shl => Op::I64Shl,
                BinOp::Shr => {
                    if signed {
                        Op::I64ShrS
                    } else {
                        Op::I64ShrU
                    }
                }
                BinOp::Eq => Op::I64Eq,
                BinOp::Ne => Op::I64Ne,
                BinOp::Lt => {
                    if signed {
                        Op::I64LtS
                    } else {
                        Op::I64LtU
                    }
                }
                BinOp::Le => {
                    if signed {
                        Op::I64LeS
                    } else {
                        Op::I64LeU
                    }
                }
                BinOp::Gt => {
                    if signed {
                        Op::I64GtS
                    } else {
                        Op::I64GtU
                    }
                }
                BinOp::Ge => {
                    if signed {
                        Op::I64GeS
                    } else {
                        Op::I64GeU
                    }
                }
                BinOp::NullCoalesce => return None,
            }),
            _ => None,
        }
    }

    pub(crate) fn from_float_bin_op(op: BinOp, value_ty: ValueType) -> Option<Self> {
        match (value_ty, op) {
            (ValueType::F32, BinOp::Add) => Some(Op::F32Add),
            (ValueType::F32, BinOp::Sub) => Some(Op::F32Sub),
            (ValueType::F32, BinOp::Mul) => Some(Op::F32Mul),
            (ValueType::F32, BinOp::Div) => Some(Op::F32Div),
            (ValueType::F32, BinOp::Eq) => Some(Op::F32Eq),
            (ValueType::F32, BinOp::Ne) => Some(Op::F32Ne),
            (ValueType::F32, BinOp::Lt) => Some(Op::F32Lt),
            (ValueType::F32, BinOp::Le) => Some(Op::F32Le),
            (ValueType::F32, BinOp::Gt) => Some(Op::F32Gt),
            (ValueType::F32, BinOp::Ge) => Some(Op::F32Ge),
            (ValueType::F64, BinOp::Add) => Some(Op::F64Add),
            (ValueType::F64, BinOp::Sub) => Some(Op::F64Sub),
            (ValueType::F64, BinOp::Mul) => Some(Op::F64Mul),
            (ValueType::F64, BinOp::Div) => Some(Op::F64Div),
            (ValueType::F64, BinOp::Eq) => Some(Op::F64Eq),
            (ValueType::F64, BinOp::Ne) => Some(Op::F64Ne),
            (ValueType::F64, BinOp::Lt) => Some(Op::F64Lt),
            (ValueType::F64, BinOp::Le) => Some(Op::F64Le),
            (ValueType::F64, BinOp::Gt) => Some(Op::F64Gt),
            (ValueType::F64, BinOp::Ge) => Some(Op::F64Ge),
            _ => None,
        }
    }
}

pub(crate) fn emit_instruction(buf: &mut Vec<u8>, op: Op) {
    match op {
        Op::Block => push_block_like(buf, 0x02),
        Op::Loop => push_block_like(buf, 0x03),
        Op::End => push_byte(buf, 0x0B),
        Op::If => push_block_like(buf, 0x04),
        Op::Else => push_byte(buf, 0x05),
        Op::Br(depth) => push_br(buf, depth),
        Op::Return => push_byte(buf, 0x0F),
        Op::Unreachable => push_byte(buf, 0x00),
        Op::Drop => push_byte(buf, 0x1A),
        Op::Call(index) => push_call(buf, index),
        Op::CallIndirect {
            type_index,
            table_index,
        } => push_call_indirect(buf, type_index, table_index),
        Op::I32Const(value) => push_i32_const(buf, value),
        Op::I64Const(value) => push_i64_const(buf, value),
        Op::F32Const(value) => push_f32_const(buf, value),
        Op::F64Const(value) => push_f64_const(buf, value),
        Op::I32Eq => push_byte(buf, 0x46),
        Op::I32Ne => push_byte(buf, 0x47),
        Op::I32Eqz => push_byte(buf, 0x45),
        Op::I32Clz => push_byte(buf, 0x67),
        Op::I32Ctz => push_byte(buf, 0x68),
        Op::I32Popcnt => push_byte(buf, 0x69),
        Op::I32Add => push_byte(buf, 0x6A),
        Op::I32Sub => push_byte(buf, 0x6B),
        Op::I32Mul => push_byte(buf, 0x6C),
        Op::I32DivS => push_byte(buf, 0x6D),
        Op::I32DivU => push_byte(buf, 0x6E),
        Op::I32RemS => push_byte(buf, 0x6F),
        Op::I32RemU => push_byte(buf, 0x70),
        Op::I32And => push_byte(buf, 0x71),
        Op::I32Or => push_byte(buf, 0x72),
        Op::I32Xor => push_byte(buf, 0x73),
        Op::I32Shl => push_byte(buf, 0x74),
        Op::I32ShrS => push_byte(buf, 0x75),
        Op::I32ShrU => push_byte(buf, 0x76),
        Op::I32Rotl => push_byte(buf, 0x77),
        Op::I32Rotr => push_byte(buf, 0x78),
        Op::I32LtS => push_byte(buf, 0x48),
        Op::I32LeS => push_byte(buf, 0x4C),
        Op::I32GtS => push_byte(buf, 0x4A),
        Op::I32GeS => push_byte(buf, 0x4E),
        Op::I32LtU => push_byte(buf, 0x49),
        Op::I32LeU => push_byte(buf, 0x4D),
        Op::I32GtU => push_byte(buf, 0x4B),
        Op::I32GeU => push_byte(buf, 0x4F),
        Op::I32WrapI64 => push_byte(buf, 0xA7),
        Op::I64ExtendI32S => push_byte(buf, 0xAC),
        Op::I64ExtendI32U => push_byte(buf, 0xAD),
        Op::I64Eqz => push_byte(buf, 0x50),
        Op::I64Eq => push_byte(buf, 0x51),
        Op::I64Ne => push_byte(buf, 0x52),
        Op::I64Clz => push_byte(buf, 0x79),
        Op::I64Ctz => push_byte(buf, 0x7A),
        Op::I64Popcnt => push_byte(buf, 0x7B),
        Op::I64Add => push_byte(buf, 0x7C),
        Op::I64Sub => push_byte(buf, 0x7D),
        Op::I64Mul => push_byte(buf, 0x7E),
        Op::I64DivS => push_byte(buf, 0x7F),
        Op::I64DivU => push_byte(buf, 0x80),
        Op::I64RemS => push_byte(buf, 0x81),
        Op::I64RemU => push_byte(buf, 0x82),
        Op::I64And => push_byte(buf, 0x83),
        Op::I64Or => push_byte(buf, 0x84),
        Op::I64Xor => push_byte(buf, 0x85),
        Op::I64Shl => push_byte(buf, 0x86),
        Op::I64ShrS => push_byte(buf, 0x87),
        Op::I64ShrU => push_byte(buf, 0x88),
        Op::I64Rotl => push_byte(buf, 0x89),
        Op::I64Rotr => push_byte(buf, 0x8A),
        Op::I64LtS => push_byte(buf, 0x53),
        Op::I64LeS => push_byte(buf, 0x57),
        Op::I64GtS => push_byte(buf, 0x55),
        Op::I64LtU => push_byte(buf, 0x54),
        Op::I64LeU => push_byte(buf, 0x58),
        Op::I64GtU => push_byte(buf, 0x56),
        Op::I64GeS => push_byte(buf, 0x59),
        Op::I64GeU => push_byte(buf, 0x5A),
        Op::F32Add => push_byte(buf, 0x92),
        Op::F32Sub => push_byte(buf, 0x93),
        Op::F32Mul => push_byte(buf, 0x94),
        Op::F32Div => push_byte(buf, 0x95),
        Op::F32Neg => push_byte(buf, 0x8C),
        Op::F32Trunc => push_byte(buf, 0x8F),
        Op::F32Eq => push_byte(buf, 0x5B),
        Op::F32Ne => push_byte(buf, 0x5C),
        Op::F32Lt => push_byte(buf, 0x5D),
        Op::F32Gt => push_byte(buf, 0x5E),
        Op::F32Le => push_byte(buf, 0x5F),
        Op::F32Ge => push_byte(buf, 0x60),
        Op::F32ConvertI32S => push_byte(buf, 0xB2),
        Op::F32ConvertI32U => push_byte(buf, 0xB3),
        Op::F32ConvertI64S => push_byte(buf, 0xB4),
        Op::F32ConvertI64U => push_byte(buf, 0xB5),
        Op::F64Add => push_byte(buf, 0xA0),
        Op::F64Sub => push_byte(buf, 0xA1),
        Op::F64Mul => push_byte(buf, 0xA2),
        Op::F64Div => push_byte(buf, 0xA3),
        Op::F64Neg => push_byte(buf, 0x9A),
        Op::F64Trunc => push_byte(buf, 0x9D),
        Op::F64Eq => push_byte(buf, 0x61),
        Op::F64Ne => push_byte(buf, 0x62),
        Op::F64Lt => push_byte(buf, 0x63),
        Op::F64Gt => push_byte(buf, 0x64),
        Op::F64Le => push_byte(buf, 0x65),
        Op::F64Ge => push_byte(buf, 0x66),
        Op::F64ConvertI32S => push_byte(buf, 0xB7),
        Op::F64ConvertI32U => push_byte(buf, 0xB8),
        Op::F64ConvertI64S => push_byte(buf, 0xB9),
        Op::F64ConvertI64U => push_byte(buf, 0xBA),
        Op::F64PromoteF32 => push_byte(buf, 0xBB),
        Op::F32DemoteF64 => push_byte(buf, 0xB6),
        Op::I32TruncF32S => push_byte(buf, 0xA8),
        Op::I32TruncF32U => push_byte(buf, 0xA9),
        Op::I32TruncF64S => push_byte(buf, 0xAA),
        Op::I32TruncF64U => push_byte(buf, 0xAB),
        Op::I64TruncF32S => push_byte(buf, 0xAE),
        Op::I64TruncF32U => push_byte(buf, 0xAF),
        Op::I64TruncF64S => push_byte(buf, 0xB0),
        Op::I64TruncF64U => push_byte(buf, 0xB1),
        Op::LocalGet(index) => push_local(buf, 0x20, index),
        Op::LocalSet(index) => push_local(buf, 0x21, index),
        Op::LocalTee(index) => push_local(buf, 0x22, index),
        Op::GlobalGet(index) => push_global(buf, 0x23, index),
        Op::GlobalSet(index) => push_global(buf, 0x24, index),
        Op::I32Load(offset) => push_memory(buf, 0x28, offset),
        Op::I32Load8S(offset) => push_memory(buf, 0x2C, offset),
        Op::I32Load8U(offset) => push_memory(buf, 0x2D, offset),
        Op::I32Load16S(offset) => push_memory(buf, 0x2E, offset),
        Op::I32Load16U(offset) => push_memory(buf, 0x2F, offset),
        Op::I32Store(offset) => push_memory(buf, 0x36, offset),
        Op::I32Store8(offset) => push_memory(buf, 0x3A, offset),
        Op::I32Store16(offset) => push_memory(buf, 0x3B, offset),
        Op::I64Load(offset) => push_memory(buf, 0x29, offset),
        Op::I64Store(offset) => push_memory(buf, 0x37, offset),
        Op::F32Load(offset) => push_memory(buf, 0x2A, offset),
        Op::F32Store(offset) => push_memory(buf, 0x38, offset),
        Op::F64Load(offset) => push_memory(buf, 0x2B, offset),
        Op::F64Store(offset) => push_memory(buf, 0x39, offset),
        Op::I32AtomicLoad(offset) => push_atomic_memory(buf, 0x10, offset),
        Op::I64AtomicLoad(offset) => push_atomic_memory(buf, 0x11, offset),
        Op::I32AtomicStore(offset) => push_atomic_memory(buf, 0x17, offset),
        Op::I64AtomicStore(offset) => push_atomic_memory(buf, 0x18, offset),
        Op::I32AtomicRmwAdd(offset) => push_atomic_memory(buf, 0x1E, offset),
        Op::I64AtomicRmwAdd(offset) => push_atomic_memory(buf, 0x1F, offset),
        Op::I32AtomicRmwSub(offset) => push_atomic_memory(buf, 0x25, offset),
        Op::I64AtomicRmwSub(offset) => push_atomic_memory(buf, 0x26, offset),
        Op::I32AtomicRmwAnd(offset) => push_atomic_memory(buf, 0x2C, offset),
        Op::I64AtomicRmwAnd(offset) => push_atomic_memory(buf, 0x2D, offset),
        Op::I32AtomicRmwOr(offset) => push_atomic_memory(buf, 0x33, offset),
        Op::I64AtomicRmwOr(offset) => push_atomic_memory(buf, 0x34, offset),
        Op::I32AtomicRmwXor(offset) => push_atomic_memory(buf, 0x3A, offset),
        Op::I64AtomicRmwXor(offset) => push_atomic_memory(buf, 0x3B, offset),
        Op::I32AtomicRmwXchg(offset) => push_atomic_memory(buf, 0x41, offset),
        Op::I64AtomicRmwXchg(offset) => push_atomic_memory(buf, 0x42, offset),
        Op::I32AtomicRmwCmpxchg(offset) => push_atomic_memory(buf, 0x48, offset),
        Op::I64AtomicRmwCmpxchg(offset) => push_atomic_memory(buf, 0x49, offset),
        Op::I32AtomicRmwMinS(offset) => push_atomic_memory(buf, 0x4F, offset),
        Op::I64AtomicRmwMinS(offset) => push_atomic_memory(buf, 0x53, offset),
        Op::I32AtomicRmwMaxS(offset) => push_atomic_memory(buf, 0x51, offset),
        Op::I64AtomicRmwMaxS(offset) => push_atomic_memory(buf, 0x55, offset),
        Op::AtomicFence => push_atomic_fence(buf),
        Op::MemoryFill => {
            push_byte(buf, 0xFC);
            push_byte(buf, 0x0B);
            push_byte(buf, 0x00);
        }
    }
}
