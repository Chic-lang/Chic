#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

use crate::mir::gpu::KernelMetadata;

/// Stub mapping of MIR kernel metadata into backend IR identifiers.
#[must_use]
pub fn encode_kernel_metadata(meta: &KernelMetadata) -> String {
    format!(
        "target={} block=({},{},{}) grid=({},{},{}) shared={}B",
        meta.target,
        meta.block_dim.0,
        meta.block_dim.1,
        meta.block_dim.2,
        meta.grid_dim.0,
        meta.grid_dim.1,
        meta.grid_dim.2,
        meta.shared_mem_bytes
    )
}
