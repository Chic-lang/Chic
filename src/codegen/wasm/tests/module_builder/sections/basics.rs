#![cfg(test)]

use crate::codegen::wasm::ValueType;
use crate::codegen::wasm::module_builder::{FunctionSignature, Section};
use crate::codegen::wasm::tests::common::*;
use crate::mir::*;

#[test]
fn function_signature_from_mir_respects_return_type() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("double"),
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(simple_return_block(0));
    let function = MirFunction {
        name: "Demo::ReturnDouble".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("int")],
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

    let layouts = wasm_layouts();
    let sig = FunctionSignature::from_mir(&function, &layouts);
    assert_eq!(sig.params, vec![ValueType::I32]);
    assert_eq!(sig.results, vec![ValueType::F64]);
}

#[test]
fn section_encode_includes_identifier_and_length() {
    let section = Section::new(11, vec![0x01, 0x02, 0x03]);
    let mut encoded = Vec::new();
    section.encode_into(&mut encoded).expect("encode section");
    assert_eq!(encoded[0], 11);
    assert!(
        encoded.ends_with(&[0x01, 0x02, 0x03]),
        "expected payload to appear at end of encoded buffer"
    );
}
