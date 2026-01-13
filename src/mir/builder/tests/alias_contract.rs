use super::common::RequireExt;
use super::*;
use crate::mir::AliasContract;

fn alias_contracts(function: &MirFunction) -> Vec<AliasContract> {
    let mut contracts = vec![AliasContract::default(); function.signature.params.len()];
    for local in &function.body.locals {
        if let LocalKind::Arg(index) = local.kind {
            contracts[index] = local.aliasing;
        }
    }
    contracts
}

#[test]
fn assigns_alias_contracts_for_borrowed_parameters() {
    let source = r#"
namespace Sample;

public unsafe void Borrowed(in int readOnly, ref int writable, out int output) { }
"#;
    let parsed = parse_module(source).require("parse alias module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let borrow_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Borrowed"))
        .expect("missing Borrowed function");
    let contracts = alias_contracts(borrow_fn);
    assert_eq!(contracts.len(), 3, "expected three parameters");

    let read_only = &contracts[0];
    assert!(read_only.noalias, "`in` parameter should be noalias");
    assert!(read_only.nocapture, "`in` parameter should be nocapture");
    assert!(read_only.readonly, "`in` parameter should be readonly");
    assert!(!read_only.writeonly, "`in` parameter must not be writeonly");

    let writable = &contracts[1];
    assert!(writable.noalias, "`ref` parameter should be noalias");
    assert!(writable.nocapture, "`ref` parameter should be nocapture");
    assert!(!writable.readonly, "`ref` parameter should not be readonly");

    let output = &contracts[2];
    assert!(output.noalias, "`out` parameter should be noalias");
    assert!(output.nocapture, "`out` parameter should be nocapture");
    assert!(output.writeonly, "`out` parameter should be writeonly");
}

#[test]
fn propagates_pointer_qualifier_metadata() {
    let source = r#"
namespace Sample;

public unsafe void Raw(*mut @restrict @aligned(32) @expose_address int dest, *mut int src) { }
"#;
    let parsed = parse_module(source).require("parse raw pointer module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let raw_fn = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Raw"))
        .expect("missing Raw function");
    let contracts = alias_contracts(raw_fn);
    assert_eq!(contracts.len(), 2, "expected raw pointer parameters");

    let dest = &contracts[0];
    assert!(dest.restrict, "dest pointer should record @restrict");
    assert!(
        dest.noalias,
        "dest pointer should be treated as noalias due to @restrict"
    );
    assert_eq!(
        dest.alignment,
        Some(32),
        "dest pointer should record @aligned(32)"
    );
    assert!(
        dest.expose_address,
        "dest pointer should record @expose_address"
    );

    let src = &contracts[1];
    assert!(
        !src.noalias && !src.restrict,
        "src pointer should not inherit restrictions by default"
    );
}
