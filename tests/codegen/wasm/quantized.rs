use chic::codegen::llvm::intrinsics::QuantRoundingMode;
use chic::codegen::llvm::numerics::quantized::{QuantPolicy, quantize_scalar as llvm_quantize};
use chic::codegen::wasm::numerics::quantized::{
    WasmQuantPolicy, WasmRoundingMode, emit_quant_sequence, quantize_scalar as wasm_quantize,
};

fn policies() -> (QuantPolicy, WasmQuantPolicy) {
    let llvm = QuantPolicy {
        scales: vec![0.25, 0.5],
        zero_points: vec![1, -3],
        per_channel_axis: Some(1),
        rounding: QuantRoundingMode::NearestEven,
        saturate: true,
    };
    let wasm = WasmQuantPolicy {
        scales: llvm.scales.clone(),
        zero_points: llvm.zero_points.clone(),
        per_channel_axis: llvm.per_channel_axis,
        rounding: WasmRoundingMode::NearestEven,
        saturate: llvm.saturate,
    };
    (llvm, wasm)
}

#[test]
fn wasm_quantization_matches_llvm_scalars() {
    let (llvm, wasm) = policies();
    let value = 1.75f32;
    let llvm_q0 = llvm_quantize(value, &llvm, 0, 8, true);
    let wasm_q0 = wasm_quantize(value, &wasm, 0, 8, true);
    assert_eq!(llvm_q0, wasm_q0, "channel 0 quantization should match");

    let llvm_q1 = llvm_quantize(value, &llvm, 1, 8, true);
    let wasm_q1 = wasm_quantize(value, &wasm, 1, 8, true);
    assert_eq!(llvm_q1, wasm_q1, "channel 1 quantization should match");
}

#[test]
fn emit_quant_sequence_captures_policy() {
    let (_, wasm_policy) = policies();
    let wat = emit_quant_sequence("acc", 0.5, &wasm_policy, 1, 8, true);
    assert!(
        wat.contains("rounding") && wat.contains("saturate"),
        "quantized sequence should record policy metadata: {wat}"
    );
}
