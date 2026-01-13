use crate::mir::{FloatStatusFlags, RoundingMode};
use crate::runtime::wasm_executor::hooks::RuntimeTermination;

use super::options::WasmExecutionTrace;

/// Result of executing a WebAssembly export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmRunOutcome {
    pub exit_code: i32,
    pub termination: Option<RuntimeTermination>,
    pub trace: WasmExecutionTrace,
    pub float_flags: FloatStatusFlags,
    pub rounding_mode: RoundingMode,
}

impl WasmRunOutcome {
    #[must_use]
    pub fn completed(exit_code: i32) -> Self {
        Self::completed_with_trace(exit_code, WasmExecutionTrace::default())
    }

    #[must_use]
    pub fn completed_with_trace(exit_code: i32, trace: WasmExecutionTrace) -> Self {
        Self {
            exit_code,
            termination: None,
            float_flags: trace.float_flags,
            rounding_mode: trace.rounding_mode,
            trace,
        }
    }

    #[must_use]
    pub fn with_trace(
        exit_code: i32,
        termination: Option<RuntimeTermination>,
        trace: WasmExecutionTrace,
    ) -> Self {
        Self {
            exit_code,
            termination,
            float_flags: trace.float_flags,
            rounding_mode: trace.rounding_mode,
            trace,
        }
    }
}
