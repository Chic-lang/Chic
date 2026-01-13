//! Byte-oriented helpers for decoding WASM sections prior to stateful parsing.
//!
//! This is a scaffold that will be populated as the parser/state split
//! progresses. For now it offers cursor management utilities that keep the
//! existing parser code compiling while we migrate logic in follow-up tasks.

#![allow(dead_code)]

use super::diagnostics::ParserDiagnostic;
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::types::{Value, ValueType};

pub(crate) struct WasmLexer<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> WasmLexer<'a> {
    pub(crate) fn new(bytes: &'a [u8], cursor: usize) -> Self {
        Self { bytes, cursor }
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8, WasmExecutionError> {
        let byte = *self
            .bytes
            .get(self.cursor)
            .ok_or_else(ParserDiagnostic::unexpected_eof)?;
        self.cursor += 1;
        Ok(byte)
    }

    pub(crate) fn preview_byte(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }
}

pub(crate) fn read_uleb(bytes: &[u8], cursor: &mut usize) -> Result<u32, WasmExecutionError> {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(*cursor).ok_or_else(|| WasmExecutionError {
            message: "unexpected end of LEB128".into(),
        })?;
        *cursor += 1;
        result |= u32::from(byte & 0x7F) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}

pub(crate) fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, WasmExecutionError> {
    let end = cursor.saturating_add(4);
    if end > bytes.len() {
        return Err(WasmExecutionError {
            message: "unexpected end of u32".into(),
        });
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[*cursor..end]);
    *cursor = end;
    Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_sleb_i32(bytes: &[u8], cursor: &mut usize) -> Result<i32, WasmExecutionError> {
    let mut result: i32 = 0;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(*cursor).ok_or_else(|| WasmExecutionError {
            message: "unexpected end of signed LEB128".into(),
        })?;
        *cursor += 1;
        result |= i32::from(byte & 0x7F) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            if shift < 32 && (byte & 0x40) != 0 {
                result |= !0 << shift;
            }
            break;
        }
    }
    Ok(result)
}

pub(crate) fn read_sleb_i64(bytes: &[u8], cursor: &mut usize) -> Result<i64, WasmExecutionError> {
    let mut result: i64 = 0;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(*cursor).ok_or_else(|| WasmExecutionError {
            message: "unexpected end of signed LEB128".into(),
        })?;
        *cursor += 1;
        result |= i64::from(byte & 0x7F) << shift;
        shift += 7;
        if (byte & 0x80) == 0 {
            if shift < 64 && (byte & 0x40) != 0 {
                result |= !0 << shift;
            }
            break;
        }
    }
    Ok(result)
}

pub(crate) fn read_f32(bytes: &[u8], cursor: &mut usize) -> Result<f32, WasmExecutionError> {
    let end = *cursor + 4;
    if end > bytes.len() {
        return Err(WasmExecutionError {
            message: "unexpected end of f32 literal".into(),
        });
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[*cursor..end]);
    *cursor = end;
    Ok(f32::from_le_bytes(buf))
}

pub(crate) fn read_f64(bytes: &[u8], cursor: &mut usize) -> Result<f64, WasmExecutionError> {
    let end = *cursor + 8;
    if end > bytes.len() {
        return Err(WasmExecutionError {
            message: "unexpected end of f64 literal".into(),
        });
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[*cursor..end]);
    *cursor = end;
    Ok(f64::from_le_bytes(buf))
}

pub(crate) fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, WasmExecutionError> {
    let end = *cursor + 8;
    if end > bytes.len() {
        return Err(WasmExecutionError {
            message: "unexpected end of u64 literal".into(),
        });
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[*cursor..end]);
    *cursor = end;
    Ok(u64::from_le_bytes(buf))
}

pub(crate) fn read_value_type(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<ValueType, WasmExecutionError> {
    match bytes.get(*cursor).copied() {
        Some(0x7F) => {
            *cursor += 1;
            Ok(ValueType::I32)
        }
        Some(0x7E) => {
            *cursor += 1;
            Ok(ValueType::I64)
        }
        Some(0x7D) => {
            *cursor += 1;
            Ok(ValueType::F32)
        }
        Some(0x7C) => {
            *cursor += 1;
            Ok(ValueType::F64)
        }
        _ => Err(WasmExecutionError {
            message: "unsupported value type".into(),
        }),
    }
}

pub(crate) fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, WasmExecutionError> {
    let len = read_uleb(bytes, cursor)? as usize;
    let end = *cursor + len;
    if end > bytes.len() {
        return Err(WasmExecutionError {
            message: "string length exceeds section".into(),
        });
    }
    let text = std::str::from_utf8(&bytes[*cursor..end])
        .map_err(|_| WasmExecutionError {
            message: "utf-8 error in string".into(),
        })?
        .to_string();
    *cursor = end;
    Ok(text)
}

pub(crate) fn read_init_expr(
    bytes: &[u8],
    cursor: &mut usize,
    ty: ValueType,
) -> Result<Value, WasmExecutionError> {
    let opcode = bytes
        .get(*cursor)
        .copied()
        .ok_or_else(|| WasmExecutionError {
            message: "unexpected end of global init expression".into(),
        })?;
    *cursor += 1;
    let value = match (opcode, ty) {
        (0x41, ValueType::I32) => {
            let literal = read_sleb_i32(bytes, cursor)?;
            Value::I32(literal)
        }
        (0x42, ValueType::I64) => {
            let literal = read_sleb_i64(bytes, cursor)?;
            Value::I64(literal)
        }
        (0x43, ValueType::F32) => Value::F32(read_f32(bytes, cursor)?),
        (0x44, ValueType::F64) => Value::F64(read_f64(bytes, cursor)?),
        _ => {
            return Err(WasmExecutionError {
                message: "unsupported global initializer".into(),
            });
        }
    };
    if bytes
        .get(*cursor)
        .copied()
        .ok_or_else(|| WasmExecutionError {
            message: "unexpected end of global init expression".into(),
        })?
        != 0x0B
    {
        return Err(WasmExecutionError {
            message: "unsupported global init expression".into(),
        });
    }
    *cursor += 1;
    Ok(value)
}
