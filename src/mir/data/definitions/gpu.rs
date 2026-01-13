//! MIR GPU kernel metadata stubs.

#[derive(Debug, Clone)]
pub struct KernelMetadata {
    pub target: String,
    pub shared_mem_bytes: u32,
    pub block_dim: (u32, u32, u32),
    pub grid_dim: (u32, u32, u32),
}
