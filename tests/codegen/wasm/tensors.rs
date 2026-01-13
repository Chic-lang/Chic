use chic::codegen::wasm::expr::tensors::{
    WasmTensorEmitter, WasmTensorLayout, plan_and_emit_alloc,
};
use chic::codegen::wasm::memory::WasmTensorAllocPlan;

#[test]
fn contiguous_copy_uses_memory_copy() {
    let layout = WasmTensorLayout {
        shape: vec![2, 3],
        strides: vec![12, 4],
        offset_bytes: 0,
        align: 4,
        mem_space: "host".into(),
    };

    let mut emitter = WasmTensorEmitter::new();
    emitter.emit_copy("dst", &layout, "src", &layout, 4);
    let wat = emitter.into_wat();

    assert!(
        wat.contains("memory.copy"),
        "contiguous layouts should emit memory.copy: {wat}"
    );
}

#[test]
fn strided_copy_emits_loops() {
    let dst = WasmTensorLayout {
        shape: vec![4, 2],
        strides: vec![32, 8],
        offset_bytes: 0,
        align: 8,
        mem_space: "host".into(),
    };
    let src = WasmTensorLayout {
        shape: vec![4, 2],
        strides: vec![64, 16],
        offset_bytes: 16,
        align: 8,
        mem_space: "host".into(),
    };

    let mut emitter = WasmTensorEmitter::new();
    emitter.emit_copy("dst", &dst, "src", &src, 4);
    let wat = emitter.into_wat();

    assert!(
        wat.contains("(loop"),
        "strided copies should lower to explicit loops: {wat}"
    );
    assert!(
        wat.contains("strides"),
        "loop body should record stride metadata: {wat}"
    );
}

#[test]
fn view_emits_pointer_offset() {
    let base = WasmTensorLayout {
        shape: vec![8, 8],
        strides: vec![32, 4],
        offset_bytes: 0,
        align: 8,
        mem_space: "host".into(),
    };
    let view = WasmTensorLayout {
        shape: vec![4, 4],
        strides: vec![32, 8],
        offset_bytes: 32,
        align: 8,
        mem_space: "host".into(),
    };

    let mut emitter = WasmTensorEmitter::new();
    emitter.emit_view("view", "base", &view, &base);
    let wat = emitter.into_wat();

    assert!(
        wat.contains("offset=32"),
        "view should apply byte offset: {wat}"
    );
    assert!(
        wat.contains("strides"),
        "view should surface stride metadata: {wat}"
    );
}

#[test]
fn allocation_bounds_guard_emits_unreachable() {
    let plan = WasmTensorAllocPlan {
        base: 8_192,
        size: 65_536,
        align: 16,
        use_heap: false,
        mem_space: "host".into(),
    };
    let mut emitter = WasmTensorEmitter::new();
    emitter.emit_alloc("t", &plan, 16_384);
    let wat = emitter.into_wat();
    assert!(
        wat.contains("unreachable"),
        "failing bounds check should trap: {wat}"
    );
}

#[test]
fn plan_and_emit_alloc_prefers_stack_when_sized() {
    let shape = vec![2, 2];
    let mut emitter = WasmTensorEmitter::new();
    plan_and_emit_alloc(
        &mut emitter,
        "small",
        &shape,
        4,
        8,
        0x1000,
        0x4000,
        true,
        "host",
    );
    let wat = emitter.into_wat();
    assert!(
        wat.contains("TensorAlloc small memspace"),
        "plan_and_emit should materialise an alloc: {wat}"
    );
}
