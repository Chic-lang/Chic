mod cpu_helpers;
mod variants;

pub(crate) use cpu_helpers::{emit_cpu_dispatch_helpers, emit_external_declarations};
pub(crate) use variants::{emit_multiversion_variants, should_multiversion};
