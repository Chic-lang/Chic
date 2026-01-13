//! Shared diagnostic construction helpers for the WASM executor parser.
//!
//! This will evolve toward full RichDiagnostic coverage as we migrate the
//! parser. For now it centralises common error messages to avoid duplication.

#![allow(dead_code)]

use crate::runtime::wasm_executor::errors::WasmExecutionError;

pub(crate) struct ParserDiagnostic;

impl ParserDiagnostic {
    pub(crate) fn unexpected_eof() -> WasmExecutionError {
        WasmExecutionError {
            message: "unexpected end of wasm payload".into(),
        }
    }

    pub(crate) fn unsupported_section(section: u8) -> WasmExecutionError {
        WasmExecutionError {
            message: format!("unsupported wasm section {section}"),
        }
    }

    pub(crate) fn invalid_opcode(opcode: u8) -> WasmExecutionError {
        WasmExecutionError {
            message: format!("unsupported wasm opcode 0x{opcode:02x}"),
        }
    }
}
