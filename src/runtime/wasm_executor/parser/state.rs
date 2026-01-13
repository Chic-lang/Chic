//! Parser state tracking glue between the lexer and higher-level builders.
//!
//! This module captures the cursor management that the parser keeps inline.

#![allow(dead_code)]

use super::diagnostics::ParserDiagnostic;
use super::lexer::{WasmLexer, read_f32, read_f64, read_sleb_i32, read_sleb_i64, read_uleb};
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::instructions::{ControlKind, Instruction};

pub(crate) struct ParserState<'a> {
    lexer: WasmLexer<'a>,
}

impl<'a> ParserState<'a> {
    pub(crate) fn new(bytes: &'a [u8], cursor: usize) -> Self {
        Self {
            lexer: WasmLexer::new(bytes, cursor),
        }
    }

    pub(crate) fn lexer(&self) -> &WasmLexer<'a> {
        &self.lexer
    }

    pub(crate) fn lexer_mut(&mut self) -> &mut WasmLexer<'a> {
        &mut self.lexer
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "Initial WASM instruction decoding remains monolithic until more helpers emerge."
)]
pub(crate) fn parse_instructions(bytes: &[u8]) -> Result<Vec<Instruction>, WasmExecutionError> {
    let mut cursor = 0usize;
    let mut instructions = Vec::new();
    let mut control_stack: Vec<(ControlKind, usize)> = Vec::new();

    while cursor < bytes.len() {
        let opcode = *bytes.get(cursor).ok_or_else(|| WasmExecutionError {
            message: "unexpected end of instruction stream".into(),
        })?;
        cursor += 1;
        let instr_index = instructions.len();
        match opcode {
            0x00 => instructions.push(Instruction::Unreachable),
            0x02 => {
                expect_block_type(bytes, &mut cursor)?;
                control_stack.push((ControlKind::Block, instr_index));
                instructions.push(Instruction::Block { end: 0 });
            }
            0x03 => {
                expect_block_type(bytes, &mut cursor)?;
                control_stack.push((ControlKind::Loop, instr_index));
                instructions.push(Instruction::Loop { end: 0 });
            }
            0x04 => {
                expect_block_type(bytes, &mut cursor)?;
                control_stack.push((ControlKind::If, instr_index));
                instructions.push(Instruction::If { end: 0 });
            }
            0x0B => {
                if let Some((kind, index)) = control_stack.pop() {
                    let end_target = instructions.len() + 1;
                    match (kind, &mut instructions[index]) {
                        (ControlKind::Block, Instruction::Block { end })
                        | (ControlKind::Loop, Instruction::Loop { end })
                        | (ControlKind::If, Instruction::If { end }) => *end = end_target,
                        _ => {
                            return Err(WasmExecutionError {
                                message: "malformed control stack".into(),
                            });
                        }
                    }
                } else {
                    break;
                }
                instructions.push(Instruction::End);
            }
            0x0C => {
                let depth = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::Br { depth });
            }
            0x0F => instructions.push(Instruction::Return),
            0x10 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::Call { func: index });
            }
            0x11 => {
                let type_index = read_uleb(bytes, &mut cursor)?;
                let table_index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::CallIndirect {
                    type_index,
                    table_index,
                });
            }
            0x1A => instructions.push(Instruction::Drop),
            0x20 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::LocalGet(index));
            }
            0x21 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::LocalSet(index));
            }
            0x22 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::LocalTee(index));
            }
            0x23 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::GlobalGet(index));
            }
            0x24 => {
                let index = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::GlobalSet(index));
            }
            0x41 => {
                let value = read_sleb_i32(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Const(value));
            }
            0x42 => {
                let value = read_sleb_i64(bytes, &mut cursor)?;
                instructions.push(Instruction::I64Const(value));
            }
            0x43 => {
                let value = read_f32(bytes, &mut cursor)?;
                instructions.push(Instruction::F32Const(value));
            }
            0x44 => {
                let value = read_f64(bytes, &mut cursor)?;
                instructions.push(Instruction::F64Const(value));
            }
            0x45 => instructions.push(Instruction::I32Eqz),
            0x46 => instructions.push(Instruction::I32Eq),
            0x47 => instructions.push(Instruction::I32Ne),
            0x48 => instructions.push(Instruction::I32LtS),
            0x49 => instructions.push(Instruction::I32LtU),
            0x4C => instructions.push(Instruction::I32LeS),
            0x4D => instructions.push(Instruction::I32LeU),
            0x4A => instructions.push(Instruction::I32GtS),
            0x4B => instructions.push(Instruction::I32GtU),
            0x4E => instructions.push(Instruction::I32GeS),
            0x4F => instructions.push(Instruction::I32GeU),
            0x50 => instructions.push(Instruction::I64Eqz),
            0x51 => instructions.push(Instruction::I64Eq),
            0x52 => instructions.push(Instruction::I64Ne),
            0x53 => instructions.push(Instruction::I64LtS),
            0x54 => instructions.push(Instruction::I64LtU),
            0x55 => instructions.push(Instruction::I64GtS),
            0x56 => instructions.push(Instruction::I64GtU),
            0x57 => instructions.push(Instruction::I64LeS),
            0x58 => instructions.push(Instruction::I64LeU),
            0x59 => instructions.push(Instruction::I64GeS),
            0x5A => instructions.push(Instruction::I64GeU),
            0x5B => instructions.push(Instruction::F32Eq),
            0x5C => instructions.push(Instruction::F32Ne),
            0x5D => instructions.push(Instruction::F32Lt),
            0x5E => instructions.push(Instruction::F32Gt),
            0x5F => instructions.push(Instruction::F32Le),
            0x60 => instructions.push(Instruction::F32Ge),
            0x61 => instructions.push(Instruction::F64Eq),
            0x62 => instructions.push(Instruction::F64Ne),
            0x63 => instructions.push(Instruction::F64Lt),
            0x64 => instructions.push(Instruction::F64Gt),
            0x65 => instructions.push(Instruction::F64Le),
            0x66 => instructions.push(Instruction::F64Ge),
            0x6A => instructions.push(Instruction::I32Add),
            0x6B => instructions.push(Instruction::I32Sub),
            0x6C => instructions.push(Instruction::I32Mul),
            0x6D => instructions.push(Instruction::I32DivS),
            0x6E => instructions.push(Instruction::I32DivU),
            0x6F => instructions.push(Instruction::I32RemS),
            0x70 => instructions.push(Instruction::I32RemU),
            0x71 => instructions.push(Instruction::I32And),
            0x72 => instructions.push(Instruction::I32Or),
            0x73 => instructions.push(Instruction::I32Xor),
            0x74 => instructions.push(Instruction::I32Shl),
            0x75 => instructions.push(Instruction::I32ShrS),
            0x76 => instructions.push(Instruction::I32ShrU),
            0x83 => instructions.push(Instruction::I64And),
            0x84 => instructions.push(Instruction::I64Or),
            0x85 => instructions.push(Instruction::I64Xor),
            0x7C => instructions.push(Instruction::I64Add),
            0x7D => instructions.push(Instruction::I64Sub),
            0x7E => instructions.push(Instruction::I64Mul),
            0x7F => instructions.push(Instruction::I64DivS),
            0x80 => instructions.push(Instruction::I64DivU),
            0x81 => instructions.push(Instruction::I64RemS),
            0x82 => instructions.push(Instruction::I64RemU),
            0x86 => instructions.push(Instruction::I64Shl),
            0x87 => instructions.push(Instruction::I64ShrS),
            0x88 => instructions.push(Instruction::I64ShrU),
            0x8F => instructions.push(Instruction::F32Trunc),
            0x92 => instructions.push(Instruction::F32Add),
            0x93 => instructions.push(Instruction::F32Sub),
            0x94 => instructions.push(Instruction::F32Mul),
            0x95 => instructions.push(Instruction::F32Div),
            0xA7 => instructions.push(Instruction::I32WrapI64),
            0xA8 => instructions.push(Instruction::I32TruncF32S),
            0xA9 => instructions.push(Instruction::I32TruncF32U),
            0xAA => instructions.push(Instruction::I32TruncF64S),
            0xAB => instructions.push(Instruction::I32TruncF64U),
            0xAC => instructions.push(Instruction::I64ExtendI32S),
            0xAD => instructions.push(Instruction::I64ExtendI32U),
            0xAE => instructions.push(Instruction::I64TruncF32S),
            0xAF => instructions.push(Instruction::I64TruncF32U),
            0xB0 => instructions.push(Instruction::I64TruncF64S),
            0xB1 => instructions.push(Instruction::I64TruncF64U),
            0x9D => instructions.push(Instruction::F64Trunc),
            0xA0 => instructions.push(Instruction::F64Add),
            0xA1 => instructions.push(Instruction::F64Sub),
            0xA2 => instructions.push(Instruction::F64Mul),
            0xA3 => instructions.push(Instruction::F64Div),
            0xB2 => instructions.push(Instruction::F32ConvertI32S),
            0xB3 => instructions.push(Instruction::F32ConvertI32U),
            0xB4 => instructions.push(Instruction::F32ConvertI64S),
            0xB5 => instructions.push(Instruction::F32ConvertI64U),
            0xB6 => instructions.push(Instruction::F32DemoteF64),
            0xB7 => instructions.push(Instruction::F64ConvertI32S),
            0xB8 => instructions.push(Instruction::F64ConvertI32U),
            0xB9 => instructions.push(Instruction::F64ConvertI64S),
            0xBA => instructions.push(Instruction::F64ConvertI64U),
            0xBB => instructions.push(Instruction::F64PromoteF32),
            0xBC => instructions.push(Instruction::F32ReinterpretI32),
            0xBD => instructions.push(Instruction::F64ReinterpretI64),
            0xBE => instructions.push(Instruction::I32ReinterpretF32),
            0xBF => instructions.push(Instruction::I64ReinterpretF64),
            0x28 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Load { offset });
            }
            0x29 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I64Load { offset });
            }
            0x2A => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::F32Load { offset });
            }
            0x2B => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::F64Load { offset });
            }
            0x2C => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Load8S { offset });
            }
            0x2D => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Load8U { offset });
            }
            0x2E => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Load16S { offset });
            }
            0x2F => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Load16U { offset });
            }
            0x36 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Store { offset });
            }
            0x3A => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Store8 { offset });
            }
            0x3B => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I32Store16 { offset });
            }
            0x37 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::I64Store { offset });
            }
            0x38 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::F32Store { offset });
            }
            0x39 => {
                let _align = read_uleb(bytes, &mut cursor)?;
                let offset = read_uleb(bytes, &mut cursor)?;
                instructions.push(Instruction::F64Store { offset });
            }
            0xFC => {
                let ext = bytes
                    .get(cursor)
                    .copied()
                    .ok_or_else(ParserDiagnostic::unexpected_eof)?;
                cursor += 1;
                match ext {
                    0x0B => {
                        let mem = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::MemoryFill { mem });
                    }
                    other => return Err(ParserDiagnostic::invalid_opcode(other)),
                }
            }
            0xFE => {
                let ext = bytes
                    .get(cursor)
                    .copied()
                    .ok_or_else(ParserDiagnostic::unexpected_eof)?;
                cursor += 1;
                match ext {
                    0x03 => {
                        let _flags = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::AtomicFence);
                    }
                    0x10 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicLoad { offset });
                    }
                    0x11 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicLoad { offset });
                    }
                    0x17 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicStore { offset });
                    }
                    0x18 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicStore { offset });
                    }
                    0x1E => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwAdd { offset });
                    }
                    0x1F => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwAdd { offset });
                    }
                    0x25 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwSub { offset });
                    }
                    0x26 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwSub { offset });
                    }
                    0x2C => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwAnd { offset });
                    }
                    0x2D => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwAnd { offset });
                    }
                    0x33 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwOr { offset });
                    }
                    0x34 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwOr { offset });
                    }
                    0x3A => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwXor { offset });
                    }
                    0x3B => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwXor { offset });
                    }
                    0x41 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwXchg { offset });
                    }
                    0x42 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwXchg { offset });
                    }
                    0x48 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwCmpxchg { offset });
                    }
                    0x49 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwCmpxchg { offset });
                    }
                    0x4F => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwMinS { offset });
                    }
                    0x53 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwMinS { offset });
                    }
                    0x51 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I32AtomicRmwMaxS { offset });
                    }
                    0x55 => {
                        let _align = read_uleb(bytes, &mut cursor)?;
                        let offset = read_uleb(bytes, &mut cursor)?;
                        instructions.push(Instruction::I64AtomicRmwMaxS { offset });
                    }
                    other => return Err(ParserDiagnostic::invalid_opcode(other)),
                }
            }
            _ => return Err(ParserDiagnostic::invalid_opcode(opcode)),
        }
    }

    Ok(instructions)
}

pub(crate) fn expect_block_type(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<(), WasmExecutionError> {
    match bytes.get(*cursor).copied() {
        Some(0x40) => {
            *cursor += 1;
            Ok(())
        }
        _ => Err(WasmExecutionError {
            message: "only empty block types supported".into(),
        }),
    }
}
