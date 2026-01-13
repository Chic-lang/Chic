//! Text formatter orchestrator.
//!
//! Pipeline: AST → layout/spacing helpers → streaming writer (structural emission). The heavy
//! lifting lives in `format/stream.rs`, with formatting primitives in `format/pretty.rs`.

mod pretty;
mod stream;

pub(crate) use stream::write_module;
