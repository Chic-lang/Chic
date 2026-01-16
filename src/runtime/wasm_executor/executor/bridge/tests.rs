use super::*;
use crate::runtime::wasm_executor::executor::options::WasmExecutionOptions;
use crate::runtime::wasm_executor::instructions::Instruction;
use crate::runtime::wasm_executor::module::{Function, Module};
use crate::runtime::wasm_executor::types::{FuncType, ValueType};
use std::collections::HashMap;

fn empty_module_with_memory() -> Module {
    Module {
        types: Vec::new(),
        imports: Vec::new(),
        functions: Vec::new(),
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: Some(1),
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    }
}

#[test]
fn await_future_reads_ready_result_from_future_layout() {
    let module = empty_module_with_memory();
    let mut exec = Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
    let layout = exec.async_layout;
    exec.write_u32(
        layout.future_header_flags_offset,
        FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED,
    )
    .unwrap();
    let result_offset = layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
    exec.store_i32(0, result_offset, 42).unwrap();
    let value = exec.await_future_blocking(0, None).expect("await result");
    assert_eq!(value, 42);
}

#[test]
fn await_future_reports_pending_then_completes() {
    let module = Module {
        types: vec![FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::I32Const(0), Instruction::Return, Instruction::End],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: Some(1),
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    let mut exec = Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
    let base = 0x100;
    let layout = exec.async_layout;
    exec.write_u32(base + layout.future_header_vtable_offset, 0x80)
        .unwrap();
    exec.write_u32(0x80, 0).unwrap();

    let status = exec.await_future_once(base).expect("await once");
    assert_eq!(status, AwaitStatus::Pending);
    assert!(exec.ready_queue.contains(&base));

    exec.write_u32(
        base + layout.future_header_flags_offset,
        FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED,
    )
    .unwrap();
    let result_offset = layout.result_offset(layout.uint_size.max(4), Some(layout.uint_align));
    exec.store_i32(base, result_offset, 7).unwrap();
    let value = exec
        .await_future_blocking(base, None)
        .expect("await completion");
    assert_eq!(value, 7);
}

#[test]
fn await_future_blocking_reports_faulted_state() {
    let module = empty_module_with_memory();
    let mut exec = Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
    let layout = exec.async_layout;
    exec.write_u32(
        layout.future_header_flags_offset,
        FUTURE_FLAG_FAULTED | FUTURE_FLAG_COMPLETED,
    )
    .unwrap();
    let err = exec
        .await_future_blocking(0, None)
        .expect_err("faulted future");
    assert!(
        err.message.contains("faulted"),
        "unexpected error message: {}",
        err.message
    );
}

#[test]
fn cancel_future_marks_flags_and_wakes_waiters() {
    let module = empty_module_with_memory();
    let mut exec = Executor::with_options(&module, &WasmExecutionOptions::default()).expect("executor");
    let layout = exec.async_layout;
    exec.write_u32(layout.future_header_flags_offset, FUTURE_FLAG_READY)
        .unwrap();
    exec.register_future_node(0).expect("register");
    exec.cancel_future(0).expect("cancel");
    let flags = exec.future_flags(0).expect("flags");
    assert!(flags & FUTURE_FLAG_CANCELLED != 0);
    assert!(flags & FUTURE_FLAG_COMPLETED != 0);
    assert!(flags & FUTURE_FLAG_READY != 0);
}
