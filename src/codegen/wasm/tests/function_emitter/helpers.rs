#![cfg(test)]

use super::common::*;
use crate::codegen::wasm::emitter::function::FunctionEmitter;
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::codegen::wasm::push_i32_const;
use crate::mir::{MirFunction, TypeLayoutTable};
use std::collections::HashMap;

pub(super) fn body_contains_call(body: &[u8], index: u32) -> bool {
    let mut pattern = vec![0x10];
    pattern.extend(leb_u32(index));
    contains_bytes(body, &pattern)
}

pub(super) fn body_contains_call_indirect(body: &[u8], type_index: u32, table_index: u32) -> bool {
    let mut pattern = vec![0x11];
    pattern.extend(leb_u32(type_index));
    pattern.extend(leb_u32(table_index));
    contains_bytes(body, &pattern)
}

pub(super) fn body_contains_i32_const(body: &[u8], value: i32) -> bool {
    let mut pattern = Vec::new();
    push_i32_const(&mut pattern, value);
    contains_bytes(body, &pattern)
}

pub(super) fn with_emitter_using_layouts<R>(
    layouts: TypeLayoutTable,
    function: MirFunction,
    configure: impl FnOnce(&mut HashMap<String, u32>) -> Option<HashMap<FunctionSignature, u32>>,
    f: impl FnOnce(&mut FunctionEmitter<'_>) -> R,
) -> R {
    let (harness, func_name) = harness_with_layouts(function, layouts);
    harness
        .with_emitter(&func_name, configure, f)
        .unwrap_or_else(|err| panic!("construct emitter: {err}"))
}

pub(super) fn with_emitter_default<R>(
    function: MirFunction,
    f: impl FnOnce(&mut FunctionEmitter<'_>) -> R,
) -> R {
    with_emitter_using_layouts(wasm_layouts(), function, |_| None, f)
}

pub(super) fn emit_body_using_layouts(
    layouts: TypeLayoutTable,
    function: MirFunction,
    configure: impl FnOnce(&mut HashMap<String, u32>) -> Option<HashMap<FunctionSignature, u32>>,
) -> Vec<u8> {
    let (harness, func_name) = harness_with_layouts(function, layouts);
    harness
        .emit_body_with(&func_name, configure)
        .unwrap_or_else(|err| panic!("emit body failed: {err}"))
}

pub(super) fn emit_body_default(function: MirFunction) -> Vec<u8> {
    emit_body_using_layouts(wasm_layouts(), function, |_| None)
}
