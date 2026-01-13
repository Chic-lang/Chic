//! Global thread runtime configuration and capability checks.

use std::sync::{OnceLock, RwLock};

/// Availability of the native threading runtime for the current compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadRuntimeMode {
    Supported,
    Unsupported { backend: &'static str },
}

impl ThreadRuntimeMode {
    #[must_use]
    pub fn backend(self) -> Option<&'static str> {
        match self {
            ThreadRuntimeMode::Supported => None,
            ThreadRuntimeMode::Unsupported { backend } => Some(backend),
        }
    }
}

fn thread_mode_cell() -> &'static RwLock<ThreadRuntimeMode> {
    static MODE: OnceLock<RwLock<ThreadRuntimeMode>> = OnceLock::new();
    MODE.get_or_init(|| RwLock::new(ThreadRuntimeMode::Supported))
}

/// Configure the current thread runtime mode.
pub fn configure_thread_runtime(mode: ThreadRuntimeMode) {
    if let Ok(mut guard) = thread_mode_cell().write() {
        *guard = mode;
    }
}

/// Query the configured runtime mode.
#[must_use]
pub fn thread_runtime_mode() -> ThreadRuntimeMode {
    thread_mode_cell()
        .read()
        .map(|guard| *guard)
        .unwrap_or(ThreadRuntimeMode::Supported)
}

/// Returns `true` when native threads are supported for the current compilation.
#[must_use]
pub fn threads_supported() -> bool {
    matches!(thread_runtime_mode(), ThreadRuntimeMode::Supported)
}
