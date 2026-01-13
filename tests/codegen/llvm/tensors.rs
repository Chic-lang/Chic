use chic::codegen::llvm::function::emit_tensor::TensorEmitter;
use chic::codegen::llvm::linalg::TensorLayout;
use chic::codegen::llvm::memory::{TensorPlacement, plan_tensor_alloc};

#[test]
fn contiguous_tensor_alloc_and_copy_use_memcpy() {
    let shape = vec![2usize, 3];
    let plan = plan_tensor_alloc(&shape, 4, Some(64), "host", true, 4096).unwrap();
    let mut layout = TensorLayout::contiguous(4, &shape);
    layout.align = plan.align;

    let mut emitter = TensorEmitter::new();
    emitter.emit_alloc("tensor", &plan);
    emitter.emit_copy("dst", &layout, "src", &layout, 4);
    let ir = emitter.into_ir();

    assert!(
        ir.contains("TensorAlloc tensor (stack)"),
        "expected stack alloc annotation"
    );
    assert!(
        ir.contains("call void @llvm.memcpy.p0.p0.i64"),
        "contiguous copies should pick memcpy intrinsic: {ir}"
    );
    assert!(
        ir.contains("i64 64"),
        "explicit alignment should surface in alloc path: {ir}"
    );
}

#[test]
fn strided_copy_falls_back_to_loop_nest() {
    let shape = vec![4usize, 2];
    let dst = TensorLayout {
        shape: shape.clone(),
        strides: vec![32, 4],
        offset_bytes: 0,
        align: 16,
        mem_space: "host".into(),
        layout_id: "blocked".into(),
    };
    let src = TensorLayout {
        shape,
        strides: vec![64, 8],
        offset_bytes: 8,
        align: 16,
        mem_space: "host".into(),
        layout_id: "strided".into(),
    };

    let mut emitter = TensorEmitter::new();
    emitter.emit_copy("dst", &dst, "src", &src, 4);
    let ir = emitter.into_ir();

    assert!(
        ir.contains("loop depth 0"),
        "fallback copy should emit loop scaffolding: {ir}"
    );
    assert!(
        ir.contains("offset"),
        "strided copy should record offset/stride metadata: {ir}"
    );
}

#[test]
fn sliced_view_computes_offset() {
    let shape = vec![8usize, 8];
    let base = TensorLayout::contiguous(4, &shape);
    let view = TensorLayout {
        shape: vec![4, 4],
        strides: vec![32, 4],
        offset_bytes: 16,
        align: 8,
        mem_space: "host".into(),
        layout_id: "slice".into(),
    };

    let mut emitter = TensorEmitter::new();
    emitter.emit_view("view", "base", &base, &view);
    let ir = emitter.into_ir();

    assert!(
        ir.contains("getelementptr"),
        "views should lower to pointer arithmetic: {ir}"
    );
    assert!(
        ir.contains("offset 16"),
        "view offset should be applied deterministically: {ir}"
    );
    assert!(
        ir.contains("strides"),
        "view should record stride metadata: {ir}"
    );
}

#[test]
fn aligned_alloc_respects_heap_policy() {
    let shape = vec![128usize, 128];
    let plan = plan_tensor_alloc(&shape, 2, Some(32), "host", false, 1024).unwrap();
    let mut emitter = TensorEmitter::new();
    emitter.emit_alloc("big", &plan);
    let ir = emitter.into_ir();

    assert!(matches!(plan.placement, TensorPlacement::Heap));
    assert!(
        ir.contains("TensorAlloc big (heap)"),
        "heap allocations should be tagged: {ir}"
    );
    assert!(ir.contains("32"), "alignment should remain explicit: {ir}");
}

#[test]
fn layout_mismatch_surfaces_diagnostic_comment() {
    let dst = TensorLayout {
        shape: vec![2, 2],
        strides: vec![8, 4],
        offset_bytes: 0,
        align: 8,
        mem_space: "host".into(),
        layout_id: "row-major".into(),
    };
    let src = TensorLayout {
        shape: vec![2, 2, 2],
        strides: vec![16, 8, 4],
        offset_bytes: 0,
        align: 8,
        mem_space: "host".into(),
        layout_id: "3d".into(),
    };

    let mut emitter = TensorEmitter::new();
    emitter.emit_copy("dst", &dst, "src", &src, 4);
    let ir = emitter.into_ir();

    assert!(
        ir.contains("rank/layout mismatch"),
        "incompatible layouts should be called out: {ir}"
    );
}
