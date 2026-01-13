#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;
use crate::mir::{
    Abi, AliasContract, BasicBlock, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, Operand, ParamMode, Place, ProjectionElem, Rvalue, Statement, StatementKind, Ty,
};

#[test]
fn alias_contract_section_serialises_restrict_pointers() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![alias_function()]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("module builder");
    let section = builder
        .emit_alias_contracts_section()
        .expect("emit alias metadata")
        .expect("alias section present");
    assert_eq!(section.id(), 0);
    let payload = section.payload_bytes();
    let mut cursor = 0usize;
    let name = read_string(payload, &mut cursor);
    assert_eq!(name, "chx.alias.contracts");
    let entry_count = read_uleb(payload, &mut cursor);
    assert_eq!(entry_count, 1, "expected single entry");
    let function = read_string(payload, &mut cursor);
    assert_eq!(function, "Demo::Alias::Copy");
    let param_count = read_uleb(payload, &mut cursor);
    assert_eq!(param_count, 2);
    let dest_flags = payload[cursor];
    cursor += 1;
    let dest_align = read_uleb(payload, &mut cursor);
    assert_eq!(dest_flags & 0x03, 0x03, "dest should be restrict+noalias");
    assert_eq!(dest_align, 16);
    let src_flags = payload[cursor];
    cursor += 1;
    let src_align = read_uleb(payload, &mut cursor);
    assert_eq!(src_flags & 0x10, 0x10, "src should retain nocapture flag");
    assert_eq!(src_align, 0);
}

fn alias_function() -> MirFunction {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let ptr_ty = Ty::named("byte*");
    body.locals.push(
        LocalDecl::new(
            Some("dest".into()),
            ptr_ty.clone(),
            true,
            None,
            LocalKind::Arg(0),
        )
        .with_param_mode(ParamMode::Value)
        .with_alias_contract(AliasContract {
            restrict: true,
            noalias: true,
            alignment: Some(16),
            ..AliasContract::default()
        }),
    );
    body.locals.push(
        LocalDecl::new(
            Some("src".into()),
            ptr_ty.clone(),
            false,
            None,
            LocalKind::Arg(1),
        )
        .with_param_mode(ParamMode::Value)
        .with_alias_contract(AliasContract {
            nocapture: true,
            ..AliasContract::default()
        }),
    );
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named("byte"),
        false,
        None,
        LocalKind::Temp,
    ));

    let mut block = BasicBlock::new(body.entry(), None);
    let mut load_place = Place::new(LocalId(2));
    load_place.projection.push(ProjectionElem::Deref);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(load_place)),
        },
    });
    let mut store_place = Place::new(LocalId(1));
    store_place.projection.push(ProjectionElem::Deref);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: store_place,
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });
    block.terminator = Some(crate::mir::Terminator::Return);
    body.blocks.push(block);

    MirFunction {
        name: "Demo::Alias::Copy".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![ptr_ty.clone(), ptr_ty],
            ret: Ty::Unit,
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
    }
}

fn read_uleb(bytes: &[u8], cursor: &mut usize) -> u32 {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        let byte = bytes[*cursor];
        *cursor += 1;
        result |= u32::from(byte & 0x7F) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
    }
    result
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> String {
    let len = read_uleb(bytes, cursor) as usize;
    let start = *cursor;
    let end = start + len;
    *cursor = end;
    std::str::from_utf8(&bytes[start..end])
        .expect("alias metadata uses utf8 names")
        .to_string()
}
