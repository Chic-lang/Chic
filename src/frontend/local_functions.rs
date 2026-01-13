//! Helpers for assigning deterministic symbol names to local functions and their environments.

/// Compute the fully-qualified symbol name for a local function.
///
/// The name is derived from the parent function's qualified name plus a stable ordinal and the
/// surface identifier so tooling can map diagnostics back to source.
#[must_use]
pub fn local_function_symbol(parent_function: &str, ordinal: usize, name: &str) -> String {
    if parent_function.is_empty() {
        format!("local${ordinal}::{name}")
    } else {
        format!("{parent_function}::local${ordinal}::{name}")
    }
}

/// Compute the synthetic struct/closure name used for the capture environment of a local function.
#[must_use]
pub fn local_function_env_name(parent_function: &str, ordinal: usize) -> String {
    if parent_function.is_empty() {
        format!("local_env#{ordinal}")
    } else {
        format!("{parent_function}::local_env#{ordinal}")
    }
}
