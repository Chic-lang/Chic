mod bridge;
mod engine;
pub mod host_io;
mod options;
mod outcome;
mod runtime;
mod scheduler;
mod traps;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwaitStatus {
    Pending = 0,
    Ready = 1,
}

pub use engine::{DefaultExecutorFactory, WasmExecutor, WasmExecutorFactory};
pub use options::{AsyncLayoutOverrides, WasmExecutionOptions, WasmExecutionTrace};
pub use outcome::WasmRunOutcome;
pub use scheduler::Executor;
#[cfg(test)]
pub(crate) use scheduler::SchedulerTracer;

use super::errors::WasmExecutionError;
use super::hooks::parse_runtime_termination;
use super::module::WasmProgram;
use crate::runtime::float_env::{read_flags, rounding_mode};
pub fn execute_wasm(bytes: &[u8], entry: &str) -> Result<WasmRunOutcome, WasmExecutionError> {
    execute_wasm_with_options(bytes, entry, &WasmExecutionOptions::default())
}

pub fn execute_wasm_with_options(
    bytes: &[u8],
    entry: &str,
    options: &WasmExecutionOptions,
) -> Result<WasmRunOutcome, WasmExecutionError> {
    let program = WasmProgram::from_bytes(bytes)?;
    let result = match program.execute_export_with_options(entry, &[], options) {
        Ok(outcome) => match outcome.value {
            None => Ok(WasmRunOutcome::completed_with_trace(0, outcome.trace)),
            Some(value) => value
                .as_i32()
                .map(|code| WasmRunOutcome::completed_with_trace(code, outcome.trace))
                .ok_or_else(|| WasmExecutionError {
                    message: format!(
                        "export `{entry}` returned a value that cannot be coerced to i32"
                    ),
                }),
        },
        Err(err) => {
            if let Some(termination) = parse_runtime_termination(&err.message) {
                let mut trace = WasmExecutionTrace::from_options(options);
                trace.rounding_mode = rounding_mode();
                trace.float_flags = read_flags();
                return Ok(WasmRunOutcome::with_trace(
                    termination.exit_code(),
                    Some(termination),
                    trace,
                ));
            }
            Err(err)
        }
    };
    let _ = unsafe { crate::runtime::tracing::chic_rt_trace_flush(std::ptr::null(), 0) };
    result
}

#[cfg(test)]
mod tests;
