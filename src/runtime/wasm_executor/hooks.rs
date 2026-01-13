//! Shared runtime hook metadata for the in-tree WebAssembly executor.
//!
//! These helpers mirror the imports registered by `codegen::wasm::ModuleBuilder`
//! so both the code generator and executor agree on exit codes and diagnostic
//! strings when runtime hooks terminate execution.

/// Exit code used when `chic_rt.panic` terminates a WebAssembly program.
pub const PANIC_EXIT_CODE: i32 = 101;

/// Exit code used when `chic_rt.abort` terminates a WebAssembly program.
pub const ABORT_EXIT_CODE: i32 = 134;

const PANIC_PREFIX: &str = "chic_rt.panic terminated execution with exit code ";
const ABORT_PREFIX: &str = "chic_rt.abort terminated execution with exit code ";

/// Classifies runtime termination initiated by one of the in-tree hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTermination {
    pub kind: RuntimeTerminationKind,
    pub exit_code: i32,
}

/// Enumeration of supported runtime hook termination kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTerminationKind {
    Panic,
    Abort,
}

impl RuntimeTermination {
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        self.exit_code
    }
}

/// Format the canonical diagnostic message for a panic termination.
#[must_use]
pub fn panic_message(exit_code: i32) -> String {
    format!("{PANIC_PREFIX}{exit_code}")
}

/// Format the canonical diagnostic message for an abort termination.
#[must_use]
pub fn abort_message(exit_code: i32) -> String {
    format!("{ABORT_PREFIX}{exit_code}")
}

/// Attempt to recover runtime termination metadata from an error message.
#[must_use]
pub fn parse_runtime_termination(message: &str) -> Option<RuntimeTermination> {
    if let Some(suffix) = message.strip_prefix(PANIC_PREFIX) {
        return suffix.parse::<i32>().ok().map(|code| RuntimeTermination {
            kind: RuntimeTerminationKind::Panic,
            exit_code: code,
        });
    }
    if let Some(suffix) = message.strip_prefix(ABORT_PREFIX) {
        return suffix.parse::<i32>().ok().map(|code| RuntimeTermination {
            kind: RuntimeTerminationKind::Abort,
            exit_code: code,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatter_round_trips_panic() {
        let message = panic_message(PANIC_EXIT_CODE);
        let parsed = parse_runtime_termination(&message);
        assert_eq!(
            parsed,
            Some(RuntimeTermination {
                kind: RuntimeTerminationKind::Panic,
                exit_code: PANIC_EXIT_CODE
            })
        );
    }

    #[test]
    fn formatter_round_trips_abort() {
        let message = abort_message(ABORT_EXIT_CODE);
        let parsed = parse_runtime_termination(&message);
        assert_eq!(
            parsed,
            Some(RuntimeTermination {
                kind: RuntimeTerminationKind::Abort,
                exit_code: ABORT_EXIT_CODE
            })
        );
    }

    #[test]
    fn parse_returns_none_for_unknown_message() {
        assert_eq!(parse_runtime_termination("unrelated error"), None);
    }
}
