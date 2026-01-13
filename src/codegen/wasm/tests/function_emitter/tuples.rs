#![cfg(test)]

use super::common::*;
use super::helpers::*;
use crate::codegen::wasm::emitter::function::LocalRepresentation;

#[test]
fn tuple_aggregate_assignments_emit_field_stores() {
    let (layouts, function) = tuple_aggregate_fixture();
    let body = emit_body_using_layouts(layouts, function, |_| None);
    let store_count = body.iter().filter(|&&byte| byte == 0x36).count();
    assert!(
        store_count >= 2,
        "tuple aggregate should emit at least two stores, found {store_count}"
    );
    let load_count = body.iter().filter(|&&byte| byte == 0x28).count();
    assert_eq!(
        load_count, 0,
        "tuple aggregate initialisation should not load from memory"
    );
}

#[test]
fn tuple_copy_assignments_emit_loads_and_stores() {
    let (layouts, function) = tuple_copy_fixture();
    let body = emit_body_using_layouts(layouts, function, |_| None);
    let store_count = body.iter().filter(|&&byte| byte == 0x36).count();
    let load_count = body.iter().filter(|&&byte| byte == 0x28).count();
    assert!(
        store_count >= 4,
        "tuple copy should emit at least four stores (two initial, two copy), found {store_count}"
    );
    assert!(
        load_count >= 2,
        "tuple copy should emit at least two loads, found {load_count}"
    );
}

#[test]
fn tuple_parameters_lower_as_pointer_params() {
    let (layouts, function) = tuple_param_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            assert!(
                matches!(
                    emitter.representations[1],
                    LocalRepresentation::PointerParam
                ),
                "tuple parameters should lower as pointer arguments"
            );
            assert!(
                emitter.locals[1].is_some(),
                "tuple pointer parameter should be assigned a wasm local"
            );
        },
    );
}
