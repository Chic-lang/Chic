#![cfg(test)]
#![allow(unused_imports)]

use super::common::*;
use super::helpers::*;
use super::*;
use crate::codegen::wasm::RuntimeHook;
use crate::drop_glue::drop_glue_symbol_for;
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, MmioEndianness, StructLayout, Ty, TypeLayout, TypeLayoutTable,
    TypeRepr,
};
use crate::mmio::{AddressSpaceId, encode_flags};

#[test]
fn emit_body_emits_mmio_runtime_calls() {
    let function = mmio_function();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    let hooks = harness.runtime_hooks().clone();
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit mmio body: {err}"));

    let write = hooks
        .get(&RuntimeHook::MmioWrite.qualified_name())
        .expect("mmio_write hook present");
    let read = hooks
        .get(&RuntimeHook::MmioRead.qualified_name())
        .expect("mmio_read hook present");
    assert!(
        body_contains_call(&body, *write),
        "expected MMIO store to call chic_rt.mmio_write"
    );
    assert!(
        body_contains_call(&body, *read),
        "expected MMIO load to call chic_rt.mmio_read"
    );
    harness.assert_golden("function_emitter/mmio_body", &body);
}

#[test]
fn emit_body_encodes_mmio_flags_with_address_space() {
    let function = mmio_function_in_apb_space();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit mmio flags body: {err}"));
    let expected_flags = encode_flags(MmioEndianness::Little, AddressSpaceId::from_name("apb"));
    assert!(
        body_contains_i32_const(&body, expected_flags),
        "expected MMIO lowering to embed encoded flags constant {expected_flags}"
    );
    harness.assert_golden("function_emitter/mmio_flags", &body);
}

#[test]
fn emit_body_errors_when_borrow_release_hook_missing() {
    let function = borrow_operand_function();
    let (harness, func_name) = harness_with_layouts(function, wasm_layouts());
    harness
        .with_emitter(
            &func_name,
            move |indices| {
                indices.remove(&RuntimeHook::BorrowRelease.qualified_name());
                None
            },
            |emitter| {
                let err = emitter
                    .emit_body()
                    .expect_err("missing borrow_release hook should error during lowering");
                assert!(
                    format!("{err}").contains("chic_rt::borrow_release"),
                    "unexpected error message: {err}"
                );
            },
        )
        .unwrap_or_else(|err| panic!("construct emitter without borrow_release: {err}"));
}

#[test]
fn emit_body_calls_drop_glue_for_nontrivial_structs() {
    let function = needs_drop_function();
    let mut layouts = wasm_layouts();
    layouts.types.insert(
        "Demo::NeedsDrop".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo::NeedsDrop".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: None,
            align: None,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: Some("Demo::NeedsDrop::dispose".into()),
            class: None,
        }),
    );

    let symbol = drop_glue_symbol_for("Demo::NeedsDrop");
    let body = emit_body_using_layouts(layouts, function, |indices| {
        indices.insert(symbol.clone(), 42);
        None
    });

    assert!(
        body_contains_call(&body, 42),
        "expected wasm emitter to call synthesised drop glue"
    );
}
