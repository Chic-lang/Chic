#![cfg(test)]

use super::common::*;
use super::helpers::*;
use crate::codegen::wasm::{STACK_POINTER_GLOBAL_INDEX, push_i32_const};
use crate::mir::{FunctionKind, Ty};

#[test]
fn emit_frame_teardown_restores_stack_pointer() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let adjust_local = emitter.stack_adjust_local;
            let mut buf = Vec::new();
            emitter.emit_frame_teardown(&mut buf);
            let mut global_get = vec![0x23];
            global_get.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
            assert!(
                buf.starts_with(&global_get),
                "frame teardown should begin by loading the stack pointer"
            );
            let mut adjust_get = vec![0x20];
            adjust_get.extend(leb_u32(adjust_local));
            assert!(
                contains_bytes(&buf, &adjust_get),
                "frame teardown should load the dynamic stack adjustment local"
            );
            if emitter.frame_size > 0 {
                let mut size_const = Vec::new();
                push_i32_const(&mut size_const, emitter.frame_size as i32);
                assert!(
                    contains_bytes(&buf, &size_const),
                    "teardown should add the fixed frame size"
                );
            }
            let mut global_set = vec![0x24];
            global_set.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
            assert!(
                contains_bytes(&buf, &global_set),
                "frame teardown must restore the stack pointer global"
            );
        },
    );
}

#[test]
fn initialise_frame_allocations_sets_pointer_locals() {
    let body = {
        let (layouts, function) = struct_projection_fixture();
        emit_body_using_layouts(layouts, function, |_| None)
    };
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let frame_local = emitter.frame_local.expect("frame local expected");
            let mut buf = Vec::new();
            emitter
                .initialise_frame_allocations(&mut buf, frame_local)
                .expect("frame allocation init should succeed");
            let mut frame_get = vec![0x20];
            frame_get.extend(leb_u32(frame_local));
            assert!(
                contains_bytes(&buf, &frame_get),
                "initialisation should load the frame pointer"
            );
        },
    );
    assert_stack_frame_initialised(&body);
}

#[test]
fn emit_frame_teardown_handles_scalar_functions() {
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    with_emitter_default(function, |emitter| {
        let mut buf = Vec::new();
        emitter.emit_frame_teardown(&mut buf);
        let mut global_get = vec![0x23];
        global_get.extend(leb_u32(STACK_POINTER_GLOBAL_INDEX));
        assert!(
            buf.starts_with(&global_get),
            "teardown should start by loading the stack pointer even for scalar-only functions"
        );
        let mut adjust_get = vec![0x20];
        adjust_get.extend(leb_u32(emitter.stack_adjust_local));
        assert!(
            contains_bytes(&buf, &adjust_get),
            "scalar teardown should still reference the stack adjustment local"
        );
    });
}

#[test]
fn wasm_emitter_frames_struct_locals_and_handles_projection() {
    let (layouts, function) = struct_projection_fixture();
    let body = emit_body_using_layouts(layouts, function, |_| None);
    assert_stack_frame_initialised(&body);
    let frame_local = body
        .chunks(2)
        .find_map(|chunk| {
            if chunk[0] == 0x22 {
                Some(chunk[1])
            } else {
                None
            }
        })
        .expect("frame allocation missing for struct local");
    let frame_local = u32::from(frame_local);
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            assert_eq!(emitter.frame_size, 8);
            let frame = emitter.frame_local.expect("frame local expected");
            assert_eq!(frame, frame_local);
        },
    );
}
