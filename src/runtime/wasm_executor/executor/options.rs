use std::collections::HashMap;
use std::env;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::mir::{FloatStatusFlags, RoundingMode};
use crate::runtime::error::RuntimeThrownException;

/// Execution-time configuration for the WASM interpreter.
#[derive(Clone)]
pub struct WasmExecutionOptions {
    pub memory_limit_pages: Option<u32>,
    pub env: HashMap<String, String>,
    pub feature_flags: Vec<String>,
    pub error_hook: Option<Arc<dyn Fn(RuntimeThrownException) + Send + Sync + 'static>>,
    pub coverage_hook: Option<Arc<dyn Fn(u64) + Send + Sync + 'static>>,
    /// Optional host IO hooks for files/sockets/clock/sleep.
    pub io_hooks: Option<super::host_io::IoHooks>,
    /// Optional layout overrides derived from the compiler's async type layouts.
    pub async_layout: Option<AsyncLayoutOverrides>,
    /// When awaiting an exported async entry/test wrapper, interpret the task result using this
    /// byte length. Falls back to the runtime layout when unset.
    pub async_result_len: Option<u32>,
    /// Alignment to apply when loading async task results. When unset, falls back to the runtime
    /// layout heuristics.
    pub async_result_align: Option<u32>,
    /// When true, treat i32 return values from the exported entry/test wrapper as task pointers and
    /// synchronously await them to produce the process exit code.
    pub await_entry_task: bool,
    /// Bytes returned when guest code reads from stdin.
    pub stdin: Vec<u8>,
    /// Mark stdin/stdout/stderr as terminals (affects guest buffering).
    pub stdin_is_terminal: bool,
    pub stdout_is_terminal: bool,
    pub stderr_is_terminal: bool,
    /// Whether to capture stdout/stderr written by guest code.
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    /// Optional IEEE 754 rounding mode to seed the executor's float environment.
    pub rounding_mode: Option<RoundingMode>,
    /// Optional per-invocation watchdog limit (WASM instruction steps).
    pub watchdog_step_limit: Option<u64>,
    /// Optional per-invocation watchdog timeout (wall-clock).
    pub watchdog_timeout: Option<Duration>,
}

impl Default for WasmExecutionOptions {
    fn default() -> Self {
        Self {
            memory_limit_pages: None,
            env: HashMap::new(),
            feature_flags: Vec::new(),
            error_hook: None,
            coverage_hook: None,
            io_hooks: None,
            async_layout: None,
            async_result_len: None,
            async_result_align: None,
            await_entry_task: true,
            stdin: Vec::new(),
            stdin_is_terminal: false,
            stdout_is_terminal: false,
            stderr_is_terminal: false,
            capture_stdout: true,
            capture_stderr: true,
            rounding_mode: rounding_mode_from_env(),
            watchdog_step_limit: None,
            watchdog_timeout: None,
        }
    }
}

impl fmt::Debug for WasmExecutionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmExecutionOptions")
            .field("memory_limit_pages", &self.memory_limit_pages)
            .field("env", &self.env)
            .field("feature_flags", &self.feature_flags)
            .field("error_hook", &self.error_hook.is_some())
            .field("coverage_hook", &self.coverage_hook.is_some())
            .field("io_hooks", &self.io_hooks.is_some())
            .field(
                "async_layout",
                &self.async_layout.as_ref().map(|_| "<provided>"),
            )
            .field("async_result_len", &self.async_result_len)
            .field("async_result_align", &self.async_result_align)
            .field("await_entry_task", &self.await_entry_task)
            .field("stdin_len", &self.stdin.len())
            .field("stdin_is_terminal", &self.stdin_is_terminal)
            .field("stdout_is_terminal", &self.stdout_is_terminal)
            .field("stderr_is_terminal", &self.stderr_is_terminal)
            .field("capture_stdout", &self.capture_stdout)
            .field("capture_stderr", &self.capture_stderr)
            .field("rounding_mode", &self.rounding_mode)
            .field("watchdog_step_limit", &self.watchdog_step_limit)
            .field("watchdog_timeout", &self.watchdog_timeout)
            .finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct AsyncLayoutOverrides {
    pub future_header_state_offset: Option<u32>,
    pub future_header_vtable_offset: Option<u32>,
    pub future_header_executor_context_offset: Option<u32>,
    pub future_header_flags_offset: Option<u32>,
    pub future_completed_offset: Option<u32>,
    pub future_result_offset: Option<u32>,
    pub task_flags_offset: Option<u32>,
    pub task_inner_future_offset: Option<u32>,
}

/// Trace describing the environment applied during WASM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExecutionTrace {
    pub memory_limit_pages: Option<u32>,
    pub env: Vec<(String, String)>,
    pub feature_flags: Vec<String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub float_flags: FloatStatusFlags,
    pub rounding_mode: RoundingMode,
}

impl WasmExecutionTrace {
    #[must_use]
    pub fn default_with_rounding(rounding_mode: RoundingMode) -> Self {
        Self {
            memory_limit_pages: None,
            env: Vec::new(),
            feature_flags: Vec::new(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            float_flags: FloatStatusFlags::default(),
            rounding_mode,
        }
    }

    pub(crate) fn from_options(options: &WasmExecutionOptions) -> Self {
        let mut env: Vec<(String, String)> = options
            .env
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        env.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            memory_limit_pages: options.memory_limit_pages,
            env,
            feature_flags: options.feature_flags.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            float_flags: FloatStatusFlags::default(),
            rounding_mode: options
                .rounding_mode
                .unwrap_or(RoundingMode::NearestTiesToEven),
        }
    }
}

fn parse_rounding_mode(text: &str) -> Option<RoundingMode> {
    match text.trim().to_ascii_lowercase().as_str() {
        "rne" | "nearest" | "nearest_even" | "nearest_even_ties" | "ties_to_even" => {
            Some(RoundingMode::NearestTiesToEven)
        }
        "rna" | "nearest_away" | "ties_to_away" => Some(RoundingMode::NearestTiesToAway),
        "rtz" | "toward_zero" | "to_zero" => Some(RoundingMode::TowardZero),
        "rtp" | "toward_positive" | "to_posinf" | "plus_inf" => Some(RoundingMode::TowardPositive),
        "rtn" | "toward_negative" | "to_neginf" | "minus_inf" => Some(RoundingMode::TowardNegative),
        _ => None,
    }
}

fn rounding_mode_from_env() -> Option<RoundingMode> {
    env::var("CHIC_ROUNDING_MODE")
        .ok()
        .and_then(|value| parse_rounding_mode(&value))
}

impl Default for WasmExecutionTrace {
    fn default() -> Self {
        Self::default_with_rounding(RoundingMode::NearestTiesToEven)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rounding_mode_variants() {
        assert_eq!(
            parse_rounding_mode("rne"),
            Some(RoundingMode::NearestTiesToEven)
        );
        assert_eq!(
            parse_rounding_mode("rna"),
            Some(RoundingMode::NearestTiesToAway)
        );
        assert_eq!(parse_rounding_mode("rtz"), Some(RoundingMode::TowardZero));
        assert_eq!(
            parse_rounding_mode("rtp"),
            Some(RoundingMode::TowardPositive)
        );
        assert_eq!(
            parse_rounding_mode("rtn"),
            Some(RoundingMode::TowardNegative)
        );
        assert!(parse_rounding_mode("unknown").is_none());
    }
}
