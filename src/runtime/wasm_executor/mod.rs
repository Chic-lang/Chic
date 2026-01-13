//! Minimal WebAssembly executor for Impact-generated modules.
//!
//! This interpreter understands the subset of WebAssembly opcodes currently
//! emitted by the Chic bootstrap compiler. It exists so `chic run/test
//! --backend wasm` can execute real `.wasm` artifacts without relying on an
//! external engine.

mod errors;
mod executor;
pub mod instructions;
pub(crate) mod module;
pub mod parser;
pub(crate) mod types;

pub use errors::WasmExecutionError;
pub use executor::host_io::IoHooks;
pub use executor::{
    AsyncLayoutOverrides, AwaitStatus, WasmExecutionOptions, WasmExecutionTrace, WasmRunOutcome,
    execute_wasm, execute_wasm_with_options,
};
pub mod hooks;
pub use module::{WasmProgram, WasmProgramExportOutcome};
pub use types::WasmValue;

pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
pub(super) const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

#[cfg(test)]
mod tests;
