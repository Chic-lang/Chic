#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::ValueType;
use crate::codegen::wasm::tests::common::*;
use crate::codegen::wasm::tests::module_builder::common::*;
use crate::mir::*;

#[test]
fn module_builder_type_section_encodes_function_signatures() {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("double"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("lhs".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("rhs".into()),
        Ty::named("float"),
        true,
        None,
        LocalKind::Arg(1),
    ));
    body.blocks.push(simple_return_block(0));

    let function = MirFunction {
        name: "Demo::Binary".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("int"), Ty::named("float")],
            ret: Ty::named("double"),
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![function]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("construct module builder");
    let state = compute_signature_state(harness.module(), false);
    let section = builder.emit_type_section().expect("type section result");
    assert_eq!(section.id(), 1);
    let payload = section.payload_bytes();
    let mut cursor = 0usize;
    let ty_count = read_uleb(payload, &mut cursor);
    assert_eq!(ty_count, state.signatures.len() as u32);
    assert_eq!(payload[cursor], 0x60);
    cursor += 1;
    let param_count = read_uleb(payload, &mut cursor);
    assert_eq!(param_count, 0x02);
    assert_eq!(payload[cursor], ValueType::I32.to_byte());
    cursor += 1;
    assert_eq!(payload[cursor], ValueType::F32.to_byte());
    cursor += 1;
    let result_count = read_uleb(payload, &mut cursor);
    assert_eq!(result_count, 0x01);
    assert_eq!(payload[cursor], ValueType::F64.to_byte());
}

#[test]
fn module_builder_function_section_lists_all_indices() {
    let function_a = simple_function("Demo::A", FunctionKind::Function, Ty::Unit);
    let function_b = simple_function("Demo::B", FunctionKind::Function, Ty::Unit);
    let harness =
        WasmFunctionHarness::from_module(module_with_functions(vec![function_a, function_b]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("construct module builder");
    let module_ref = harness.module();
    let state = compute_signature_state(module_ref, false);
    let section = builder
        .emit_function_section()
        .expect("function section result");
    assert_eq!(section.id(), 3);
    let payload = section.payload_bytes();
    let mut cursor = 0usize;
    let function_count = read_uleb(payload, &mut cursor);
    assert_eq!(function_count, module_ref.functions.len() as u32);
    for (idx, &expected_index) in state.function_type_indices.iter().enumerate() {
        let actual = read_uleb(payload, &mut cursor);
        assert_eq!(
            actual, expected_index,
            "function {idx} should reference canonical signature index"
        );
    }
    assert_eq!(cursor, payload.len());
}
