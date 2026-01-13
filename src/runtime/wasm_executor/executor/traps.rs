use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::hooks::{abort_message, panic_message};

pub(super) fn panic_trap(code: i32) -> WasmExecutionError {
    WasmExecutionError {
        message: panic_message(code),
    }
}

pub(super) fn abort_trap(code: i32) -> WasmExecutionError {
    WasmExecutionError {
        message: abort_message(code),
    }
}
