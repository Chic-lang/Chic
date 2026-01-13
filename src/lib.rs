#![deny(warnings)]
#![deny(clippy::all, clippy::pedantic, clippy::perf, clippy::suspicious)] // Catch correctness + perf + suspicious patterns early.
#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Core library for the temporary Rust implementation of the Chic compiler.

pub mod abi;
pub mod accessibility;
pub mod async_flags;
pub mod cc1;
pub mod chic_coverage;
pub mod chic_kind;
pub mod cli;
pub mod clone_glue;
pub mod code_style;
pub mod codegen;
pub mod const_eval_config;
pub mod decimal;
pub mod defines;
pub mod di;
pub mod diagnostics;
pub mod doc;
pub mod driver;
pub mod drop_glue;
pub mod eq_glue;
pub mod error;
pub mod export;
pub mod extern_bind;
pub mod format;
pub mod frontend;
pub mod hash_glue;
pub mod header;
pub mod import;
pub mod language;
pub mod lint;
pub mod logging;
pub mod lsp;
pub mod manifest;
pub mod mir;
pub mod mmio;
pub mod monomorphize;
pub mod package;
pub mod perf;
pub mod primitives;
pub mod run_log;
pub mod runtime;
pub mod runtime_package;
pub mod spec;
pub mod string_support;
pub mod support;
pub mod syntax;
pub mod target;
pub mod threading;
pub mod type_identity;
pub mod type_metadata;
pub mod typeck;
pub mod unicode;
pub mod version;

pub use chic_kind::ChicKind;
pub use driver::{CompilerDriver, TestCaseResult, TestRun, TestStatus};
pub use error::{Error, Result};
pub use target::Target;
