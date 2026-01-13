#![cfg(test)]

use crate::codegen::wasm::RuntimeHook;
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::codegen::wasm::runtime_hooks::ALL_RUNTIME_HOOKS;
use crate::mir::MirModule;
use std::collections::HashMap;

pub(super) fn runtime_hooks(coverage_enabled: bool) -> Vec<RuntimeHook> {
    ALL_RUNTIME_HOOKS
        .iter()
        .copied()
        .filter(|hook| coverage_enabled || !matches!(hook, RuntimeHook::CoverageHit))
        .collect()
}

pub(super) struct SignatureState {
    pub(crate) signatures: Vec<FunctionSignature>,
    pub(crate) function_type_indices: Vec<u32>,
    pub(crate) runtime_hook_indices: HashMap<RuntimeHook, u32>,
}

pub(super) fn compute_signature_state(
    module: &MirModule,
    coverage_enabled: bool,
) -> SignatureState {
    let mut signatures = Vec::new();
    let mut signature_indices = HashMap::new();
    let mut function_type_indices = Vec::new();
    let mut runtime_hook_indices = HashMap::new();

    for function in &module.functions {
        let sig = FunctionSignature::from_mir(function, &module.type_layouts);
        let index = intern_signature(sig, &mut signature_indices, &mut signatures);
        function_type_indices.push(index);
    }

    for hook in runtime_hooks(coverage_enabled) {
        let sig = hook.signature();
        let index = intern_signature(sig, &mut signature_indices, &mut signatures);
        runtime_hook_indices.insert(hook, index);
    }

    SignatureState {
        signatures,
        function_type_indices,
        runtime_hook_indices,
    }
}

fn intern_signature(
    sig: FunctionSignature,
    indices: &mut HashMap<FunctionSignature, u32>,
    order: &mut Vec<FunctionSignature>,
) -> u32 {
    if let Some(&index) = indices.get(&sig) {
        index
    } else {
        let index = order.len() as u32;
        indices.insert(sig.clone(), index);
        order.push(sig);
        index
    }
}

pub(super) fn read_uleb(bytes: &[u8], cursor: &mut usize) -> u32 {
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

pub(super) fn read_string(bytes: &[u8], cursor: &mut usize) -> String {
    let len = read_uleb(bytes, cursor) as usize;
    let start = *cursor;
    let end = start + len;
    *cursor = end;
    std::str::from_utf8(&bytes[start..end])
        .expect("payload should contain utf8 strings")
        .to_string()
}
