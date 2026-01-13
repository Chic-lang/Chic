use chic::codegen::llvm::gpu::kernel::encode_kernel_metadata;
use chic::mir::gpu::KernelMetadata;

#[test]
fn encode_kernel_metadata_formats_fields() {
    let meta = KernelMetadata {
        target: "ptx".into(),
        shared_mem_bytes: 1024,
        block_dim: (16, 8, 1),
        grid_dim: (32, 1, 1),
    };
    let encoded = encode_kernel_metadata(&meta);
    assert!(encoded.contains("target=ptx"));
    assert!(encoded.contains("block=(16,8,1)"));
    assert!(encoded.contains("shared=1024B"));
}
