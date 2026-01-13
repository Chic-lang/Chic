//! Chic formatter configuration and formatting engine scaffolding.
//!
//! The formatter is configurable via `manifest.yaml` and exposed through the
//! `chic format` CLI entrypoint.

pub mod config;
pub mod formatter;

pub use config::*;
pub use formatter::*;
