use chic::codegen::llvm::{
    function::emit_tensor::TensorEmitter, linalg::TensorLayout, memory::plan_tensor_alloc,
};
use chic::codegen::wasm::expr::tensors::{
    WasmTensorEmitter, WasmTensorLayout, plan_and_emit_alloc,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn llvm_alloc_row_major(c: &mut Criterion) {
    let shape = vec![64usize, 64];
    c.bench_function("tensors::alloc::llvm_row_major", |b| {
        b.iter(|| {
            let plan =
                plan_tensor_alloc(&shape, 4, Some(64), "host", true, 1 << 20).expect("alloc plan");
            let mut emitter = TensorEmitter::new();
            emitter.emit_alloc("bench", &plan);
            black_box(emitter.into_ir());
        });
    });
}

fn llvm_copy_strided(c: &mut Criterion) {
    let dst = TensorLayout {
        shape: vec![16, 8],
        strides: vec![256, 16],
        offset_bytes: 0,
        align: 32,
        mem_space: "host".into(),
        layout_id: "row-major".into(),
    };
    let src = TensorLayout {
        shape: vec![16, 8],
        strides: vec![512, 32],
        offset_bytes: 32,
        align: 32,
        mem_space: "host".into(),
        layout_id: "blocked".into(),
    };
    c.bench_function("tensors::copy::llvm_strided", |b| {
        b.iter(|| {
            let mut emitter = TensorEmitter::new();
            emitter.emit_copy("dst", &dst, "src", &src, 4);
            black_box(emitter.into_ir());
        });
    });
}

fn wasm_alloc_and_copy(c: &mut Criterion) {
    let shape = vec![32u32, 32];
    c.bench_function("tensors::alloc::wasm_linear", |b| {
        b.iter(|| {
            let mut emitter = WasmTensorEmitter::new();
            plan_and_emit_alloc(
                &mut emitter,
                "tmp",
                &shape,
                4,
                16,
                0x1000,
                0x40000,
                true,
                "host",
            );
            black_box(emitter.into_wat());
        });
    });

    let layout = WasmTensorLayout {
        shape: shape.clone(),
        strides: vec![128, 4],
        offset_bytes: 0,
        align: 16,
        mem_space: "host".into(),
    };
    c.bench_function("tensors::copy::wasm_contiguous", |b| {
        b.iter(|| {
            let mut emitter = WasmTensorEmitter::new();
            emitter.emit_copy("dst", &layout, "src", &layout, 4);
            black_box(emitter.into_wat());
        });
    });
}

fn tensor_benches(c: &mut Criterion) {
    llvm_alloc_row_major(c);
    llvm_copy_strided(c);
    wasm_alloc_and_copy(c);
}

criterion_group!(tensor_codegen, tensor_benches);
criterion_main!(tensor_codegen);
