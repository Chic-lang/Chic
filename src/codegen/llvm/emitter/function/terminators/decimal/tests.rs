use super::intrinsic::decimal_op_from_kind;
use super::runtime::{decimal_runtime_symbol, runtime_spec};
use super::wrappers::{DecimalWrapperSignature, decimal_op_from_method, wrapper_spec};

#[test]
fn runtime_spec_resolves_scalar_variants() {
    let scalar = runtime_spec(
        "std::decimal::intrinsics::runtimeintrinsics::chic_rt_decimal_add",
        "",
    )
    .expect("scalar spec");
    assert_eq!("chic_rt_decimal_add", scalar.symbol);
    assert_eq!(2, scalar.decimal_args);

    assert!(
        runtime_spec(
            "std::decimal::intrinsics::runtimeintrinsics::chic_rt_decimal_add_simd",
            ""
        )
        .is_none(),
        "SIMD runtime entrypoints have been removed"
    );
}

#[test]
fn decimal_op_helpers_cover_wrapper_and_kind_mappings() {
    assert_eq!(
        Some("add"),
        decimal_op_from_method("AddVectorizedWithRounding")
    );
    assert_eq!(Some("fma"), decimal_op_from_method("Fma"));
    assert!(decimal_op_from_method("Unsupported").is_none());
    assert_eq!(
        "mul",
        decimal_op_from_kind(crate::mir::DecimalIntrinsicKind::Mul)
    );
}

#[test]
fn wrapper_spec_classifies_binary_and_fma_signatures() {
    let binary = wrapper_spec(
        "std::decimal::intrinsics::addwithoptions",
        "Std::Decimal::Intrinsics::Add",
    );
    let binary = binary.expect("binary spec");
    matches!(binary.signature, DecimalWrapperSignature::BinaryWithOptions);

    let fma = wrapper_spec(
        "std::decimal::intrinsics::fmawithrounding",
        "Std::Decimal::Intrinsics::FmaWithRounding",
    )
    .expect("fma spec");
    matches!(fma.signature, DecimalWrapperSignature::FmaWithRounding);
}

#[test]
fn decimal_runtime_symbol_handles_known_ops() {
    assert_eq!(
        Some("chic_rt_decimal_mul"),
        decimal_runtime_symbol("mul", false)
    );
    assert_eq!(
        Some("chic_rt_decimal_mul"),
        decimal_runtime_symbol("mul", true),
        "simd flag should map to scalar runtime symbol"
    );
    assert!(decimal_runtime_symbol("noop", false).is_none());
}
