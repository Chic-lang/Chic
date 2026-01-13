use chic::codegen::llvm::intrinsics::QuantRoundingMode;
use chic::codegen::llvm::numerics::quantized::{
    QuantPolicy, choose_kernel, dequantize_scalar, quantize_scalar,
};

fn default_policy() -> QuantPolicy {
    QuantPolicy {
        scales: vec![0.5],
        zero_points: vec![0],
        per_channel_axis: None,
        rounding: QuantRoundingMode::NearestEven,
        saturate: true,
    }
}

#[test]
fn quantize_scalar_rounds_and_clamps() {
    let policy = default_policy();
    let quantized = quantize_scalar(1.0, &policy, 0, 8, true);
    assert_eq!(quantized, 2, "1.0 / 0.5 should round to 2");

    let clamped = quantize_scalar(200.0, &policy, 0, 8, true);
    assert_eq!(clamped, 127, "8-bit signed saturates at 127");
}

#[test]
fn per_channel_policy_changes_zero_points() {
    let policy = QuantPolicy {
        scales: vec![1.0, 0.3],
        zero_points: vec![0, 0],
        per_channel_axis: Some(1),
        rounding: QuantRoundingMode::NearestEven,
        saturate: true,
    };
    let ch0 = quantize_scalar(1.1, &policy, 0, 8, true);
    let ch1 = quantize_scalar(1.1, &policy, 1, 8, true);
    assert_ne!(ch0, ch1, "channel metadata should influence quantization");
    let dq0 = dequantize_scalar(ch0, &policy, 0);
    let dq1 = dequantize_scalar(ch1, &policy, 1);
    assert!(
        (dq0 - dq1).abs() > f32::EPSILON,
        "channel 1 scale should produce a different dequantized value"
    );
}

#[test]
fn quantized_kernel_selection_prefers_intrinsic() {
    let policy = default_policy();
    let kernel = choose_kernel("qgemm", &policy).expect("intrinsic available");
    assert_eq!(kernel.symbol, "llvm.chic.quant.qgemm");

    let mut unsupported = policy;
    unsupported.rounding = QuantRoundingMode::TowardZero;
    assert!(
        choose_kernel("qgemm", &unsupported).is_none(),
        "unsupported rounding should fall back to loop nest"
    );
}
