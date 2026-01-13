use crate::runtime::error::{RuntimeThrownException, exception_type_identity};
use crate::runtime::span::SpanError;
use crate::runtime::wasm_executor::executor::{Executor, WasmExecutionOptions, execute_wasm};
use crate::runtime::wasm_executor::instructions::Instruction;
use crate::runtime::wasm_executor::module::{
    Function, Global, Import, Module, Table, TableElementType, TypeMetadataRecord,
};
use crate::runtime::wasm_executor::types::{FuncType, Value, ValueType, WasmValue};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

mod host_hooks;

const EXPECTED_ARITH_CHECKS: i32 = 17;

fn expect_ok<T, E: Debug>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err:?}"),
    }
}

fn expect_err<T, E: Debug>(result: Result<T, E>, context: &str) -> E {
    match result {
        Ok(_) => panic!("{context}: expected error"),
        Err(err) => err,
    }
}

#[test]
fn execute_wasm_runs_entry_function() {
    let wasm = crate::runtime::wasm_executor::tests::simple_module(13);
    let outcome = expect_ok(execute_wasm(&wasm, "chic_main"), "execute wasm");
    assert_eq!(outcome.exit_code, 13);
    assert!(outcome.termination.is_none());
}

#[test]
fn executor_errors_when_function_missing() {
    let module = Module {
        types: Vec::new(),
        imports: Vec::new(),
        functions: Vec::new(),
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "missing function index");
    assert!(err.message.contains("function index"));
}

#[test]
fn executor_errors_on_argument_mismatch() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: vec![ValueType::I32],
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::Return],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "argument mismatch");
    assert!(err.message.contains("call argument mismatch"));
}

#[test]
fn executor_errors_on_argument_type_mismatch() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: vec![ValueType::I32],
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::Return],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[Value::I64(4)]), "argument type mismatch");
    assert!(err.message.contains("argument type mismatch"));
}

#[test]
fn span_copy_to_import_copies_payload() {
    let module = Module {
        types: vec![
            FuncType {
                params: vec![ValueType::I32; 8],
                results: vec![ValueType::I32],
            },
            FuncType {
                params: Vec::new(),
                results: vec![ValueType::I32],
            },
        ],
        imports: vec![Import {
            module: "chic_rt".into(),
            name: "span_copy_to".into(),
            type_index: 0,
        }],
        functions: vec![Function {
            type_index: 1,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(64),
                Instruction::I32Const(3),
                Instruction::I32Const(1),
                Instruction::I32Const(1),
                Instruction::I32Const(128),
                Instruction::I32Const(3),
                Instruction::I32Const(1),
                Instruction::I32Const(1),
                Instruction::Call { func: 0 },
                Instruction::Return,
            ],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::from([("chic_main".into(), 1)]),
        memory_min_pages: Some(1),
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };

    let mut exec = Executor::new(&module);
    expect_ok(exec.store_bytes(64, 0, b"abc"), "seed source bytes");
    let result = expect_ok(exec.call(1, &[]), "invoke span copy");
    let status = match result {
        Some(WasmValue::I32(code)) => code,
        other => panic!("unexpected result from span copy import: {other:?}"),
    };
    assert_eq!(status, SpanError::Success as i32);
    let copied = expect_ok(exec.read_bytes(128, 3), "read copied bytes");
    assert_eq!(copied, b"abc");
}

#[test]
fn executor_handles_arithmetic_and_bitwise_instructions() {
    let (module, expected_checks) = module_with_arithmetic_tests();
    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "arithmetic execution");
    match result {
        Some(WasmValue::I32(v)) => assert_eq!(v, expected_checks),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn executor_handles_control_flow_and_calls() {
    let module = module_with_control_flow();
    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "control flow execution");
    match result {
        Some(WasmValue::I32(v)) => assert_eq!(v, 99),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn executor_reports_division_by_zero() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(1),
                Instruction::I32Const(0),
                Instruction::I32DivS,
            ],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "division by zero");
    assert!(err.message.contains("division by zero"));
}

#[test]
fn executor_reports_branch_depth_errors() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::Br { depth: 0 }],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "stack drop underflow");
    assert!(err.message.contains("branch depth"));
}

#[test]
fn executor_reports_local_set_overflow() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::I32Const(1), Instruction::LocalSet(1)],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "block end mismatch");
    assert!(err.message.contains("local.set index"));
}

#[test]
fn chic_rt_throw_invokes_exception_hook() {
    let type_id = exception_type_identity("Demo::Failure");
    let module = Module {
        types: vec![
            FuncType {
                params: vec![ValueType::I32, ValueType::I64],
                results: Vec::new(),
            },
            FuncType {
                params: Vec::new(),
                results: Vec::new(),
            },
        ],
        imports: vec![Import {
            module: "chic_rt".into(),
            name: "throw".into(),
            type_index: 0,
        }],
        functions: vec![Function {
            type_index: 1,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(0x1234),
                Instruction::I64Const(type_id as i64),
                Instruction::Call { func: 0 },
                Instruction::Return,
            ],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::from([("chic_main".into(), 1)]),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };

    let captured: Arc<Mutex<Vec<RuntimeThrownException>>> = Arc::new(Mutex::new(Vec::new()));
    let hook_sink = captured.clone();
    let mut options = WasmExecutionOptions::default();
    options.error_hook = Some(Arc::new(move |thrown: RuntimeThrownException| {
        hook_sink.lock().unwrap().push(thrown);
    }));

    let mut exec = Executor::with_options(&module, &options).expect("construct executor with hook");
    let _ = expect_ok(exec.call(1, &[]), "throw with pending exception");

    let recorded = captured.lock().unwrap();
    assert_eq!(recorded.len(), 1, "expected single runtime throw");
    let thrown = recorded[0];
    assert_eq!(thrown.payload, 0x1234, "unexpected payload pointer");
    assert_eq!(
        thrown.type_id, type_id,
        "unexpected exception type identity"
    );

    let pending = exec
        .pending_exception
        .expect("pending exception should be recorded by executor");
    assert_eq!(pending.payload, 0x1234, "unexpected pending payload");
    assert_eq!(pending.type_id, type_id, "unexpected pending type id");
}

#[test]
fn executor_handles_global_get_and_set() {
    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: vec![ValueType::I32],
            code: vec![
                Instruction::GlobalGet(1),
                Instruction::LocalSet(0),
                Instruction::I32Const(5),
                Instruction::GlobalSet(1),
                Instruction::GlobalGet(1),
                Instruction::Return,
            ],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: Some(1),
        globals: vec![
            Global {
                ty: ValueType::I32,
                mutable: true,
                initial: Value::I32(0),
            },
            Global {
                ty: ValueType::I32,
                mutable: true,
                initial: Value::I32(3),
            },
        ],
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "global execution");
    match result {
        Some(WasmValue::I32(v)) => assert_eq!(v, 5),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn executor_rejects_global_set_on_immutable() {
    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(1),
                Instruction::GlobalSet(0),
                Instruction::Return,
            ],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: vec![Global {
            ty: ValueType::I32,
            mutable: false,
            initial: Value::I32(0),
        }],
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "immutable global");
    assert!(
        err.message.contains("immutable"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn executor_handles_memory_load_and_store() {
    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(16),
                Instruction::I32Const(123),
                Instruction::I32Store { offset: 0 },
                Instruction::I32Const(16),
                Instruction::I32Load { offset: 0 },
                Instruction::Return,
            ],
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
    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "memory execution");
    match result {
        Some(WasmValue::I32(v)) => assert_eq!(v, 123),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn executor_reports_memory_out_of_bounds() {
    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(70000),
                Instruction::I32Const(1),
                Instruction::I32Store { offset: 0 },
                Instruction::Return,
            ],
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
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "memory oob");
    assert!(
        err.message.contains("out of bounds"),
        "unexpected throw message: {err:?}"
    );
}

#[test]
fn executor_propagates_runtime_panic() {
    let module = Module {
        types: vec![
            FuncType {
                params: vec![ValueType::I32],
                results: Vec::new(),
            },
            FuncType {
                params: Vec::new(),
                results: Vec::new(),
            },
        ],
        imports: vec![Import {
            module: "chic_rt".to_string(),
            name: "panic".to_string(),
            type_index: 0,
        }],
        functions: vec![Function {
            type_index: 1,
            locals: Vec::new(),
            code: vec![
                Instruction::I32Const(7),
                Instruction::Call { func: 0 },
                Instruction::Return,
            ],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::from([("chic_main".to_string(), 1)]),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(1, &[]), "runtime panic");
    assert!(err.message.contains("panic"));
    assert!(err.message.contains("exit code 7"));
}

#[test]
fn executor_reports_unreachable() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::Unreachable],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "loop end mismatch");
    assert!(err.message.contains("unreachable"));
}

#[test]
fn call_indirect_dispatches_through_function_table() {
    use Instruction::{CallIndirect, I32Const, Return};

    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions: vec![
            Function {
                type_index: 0,
                locals: Vec::new(),
                code: vec![I32Const(11), Return],
            },
            Function {
                type_index: 0,
                locals: Vec::new(),
                code: vec![I32Const(4), Return],
            },
            Function {
                type_index: 0,
                locals: Vec::new(),
                code: vec![
                    I32Const(1),
                    CallIndirect {
                        type_index: 0,
                        table_index: 0,
                    },
                    Return,
                ],
            },
        ],
        function_names: Vec::new(),
        tables: vec![Table {
            element_type: TableElementType::FuncRef,
            min: 2,
            max: None,
            elements: vec![Some(0), Some(1)],
        }],
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };

    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(2, &[]), "call via function pointer");
    match result {
        Some(WasmValue::I32(v)) => assert_eq!(v, 4),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn executor_reports_missing_return_value() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: Vec::new(),
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "if end mismatch");
    assert!(err.message.contains("completed without returning"));
}

#[test]
fn executor_reports_stack_underflow() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::I32Add],
        }],
    );
    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(0, &[]), "invalid branch target");
    assert!(err.message.contains("value stack underflow"));
}

#[test]
fn executor_returns_none_for_void_function() {
    let module = simple_module_with_body(
        vec![FuncType {
            params: Vec::new(),
            results: Vec::new(),
        }],
        vec![Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::Return],
        }],
    );
    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "void function result");
    assert!(result.is_none());
}

#[test]
fn object_new_allocates_zeroed_block() {
    use Instruction::{Call, I64Const, Return};

    let type_id = 0xAA55_AA55_DEAD_BEEFu64;
    let module = Module {
        types: vec![
            FuncType {
                params: vec![ValueType::I64],
                results: vec![ValueType::I32],
            },
            FuncType {
                params: Vec::new(),
                results: vec![ValueType::I32],
            },
        ],
        imports: vec![Import {
            module: "chic_rt".into(),
            name: "object_new".into(),
            type_index: 0,
        }],
        functions: vec![Function {
            type_index: 1,
            locals: Vec::new(),
            code: vec![I64Const(type_id as i64), Call { func: 0 }, Return],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::from([("chic_main".to_string(), 1)]),
        memory_min_pages: Some(1),
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: vec![TypeMetadataRecord {
            type_id,
            size: 32,
            align: 8,
            variance: Vec::new(),
        }],
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };

    let mut exec = Executor::new(&module);
    assert_eq!(
        exec.type_metadata_len(),
        1,
        "type metadata should be registered"
    );
    assert!(
        exec.has_type_metadata(type_id),
        "type metadata map missing expected type id"
    );
    let result = expect_ok(exec.call(1, &[]), "object_new call");
    let mut ptr = match result {
        Some(WasmValue::I32(addr)) => addr as u32,
        other => panic!("unexpected result: {other:?}"),
    };
    if ptr == 0 {
        let end = exec.heap_cursor();
        ptr = end.saturating_sub(32);
    }
    assert_ne!(ptr, 0);
    let bytes = exec.test_memory_mut();
    assert!(
        bytes.len() >= ptr as usize + 32,
        "allocation should fit within linear memory"
    );
}

#[test]
fn object_new_without_metadata_errors() {
    use Instruction::{Call, I64Const, Return};

    let module = Module {
        types: vec![
            FuncType {
                params: vec![ValueType::I64],
                results: vec![ValueType::I32],
            },
            FuncType {
                params: Vec::new(),
                results: vec![ValueType::I32],
            },
        ],
        imports: vec![Import {
            module: "chic_rt".into(),
            name: "object_new".into(),
            type_index: 0,
        }],
        functions: vec![Function {
            type_index: 1,
            locals: Vec::new(),
            code: vec![I64Const(0x1234), Call { func: 0 }, Return],
        }],
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::from([("chic_main".to_string(), 1)]),
        memory_min_pages: Some(1),
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };

    let mut exec = Executor::new(&module);
    let err = expect_err(exec.call(1, &[]), "object_new fallback");
    assert!(err.message.contains("object_new missing type metadata"));
}

fn simple_module_with_body(types: Vec<FuncType>, functions: Vec<Function>) -> Module {
    Module {
        types,
        imports: Vec::new(),
        functions,
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    }
}

fn module_with_arithmetic_tests() -> (Module, i32) {
    use Instruction::*;

    let mut code = vec![I32Const(0), LocalSet(0)];
    let sequences = arithmetic_sequences();
    for (seq, expected) in &sequences {
        add_check(
            &mut code,
            |program| program.extend(seq.iter().cloned()),
            *expected,
        );
    }

    code.extend([I32Const(123), Drop, LocalGet(0), Return]);

    let functions = vec![Function {
        type_index: 0,
        locals: vec![ValueType::I32],
        code,
    }];
    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions,
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    };
    debug_assert_eq!(sequences.len(), EXPECTED_ARITH_CHECKS as usize);
    (module, EXPECTED_ARITH_CHECKS)
}

fn arithmetic_sequences() -> Vec<(Vec<Instruction>, i32)> {
    use Instruction::*;
    vec![
        (vec![I32Const(5), I32Const(5), I32Eq], 1),
        (vec![I32Const(5), I32Const(3), I32Ne], 1),
        (vec![I32Const(0), I32Eqz], 1),
        (vec![I32Const(2), I32Const(3), I32LtS], 1),
        (vec![I32Const(2), I32Const(3), I32LeS], 1),
        (vec![I32Const(4), I32Const(3), I32GtS], 1),
        (vec![I32Const(4), I32Const(3), I32GeS], 1),
        (vec![I32Const(5), I32Const(3), I32Add], 8),
        (vec![I32Const(5), I32Const(3), I32Sub], 2),
        (vec![I32Const(5), I32Const(3), I32Mul], 15),
        (vec![I32Const(6), I32Const(3), I32DivS], 2),
        (vec![I32Const(7), I32Const(4), I32RemS], 3),
        (vec![I32Const(0b1100), I32Const(0b1010), I32And], 0b1000),
        (vec![I32Const(0b1100), I32Const(0b1010), I32Or], 0b1110),
        (vec![I32Const(0b1100), I32Const(0b1010), I32Xor], 0b0110),
        (vec![I32Const(1), I32Const(3), I32Shl], 8),
        (vec![I32Const(8), I32Const(1), I32ShrS], 4),
    ]
}

fn add_check<F>(code: &mut Vec<Instruction>, mut expr: F, expected: i32)
where
    F: FnMut(&mut Vec<Instruction>),
{
    code.push(Instruction::LocalGet(0));
    expr(code);
    code.push(Instruction::I32Const(expected));
    code.push(Instruction::I32Eq);
    code.push(Instruction::I32Add);
    code.push(Instruction::LocalSet(0));
}

fn module_with_control_flow() -> Module {
    let code = vec![
        Instruction::Block { end: 7 },
        Instruction::Loop { end: 6 },
        Instruction::I32Const(0),
        Instruction::If { end: 5 },
        Instruction::Br { depth: 0 },
        Instruction::End,
        Instruction::End,
        Instruction::End,
        Instruction::Call { func: 1 },
        Instruction::Return,
    ];

    let functions = vec![
        Function {
            type_index: 0,
            locals: Vec::new(),
            code,
        },
        Function {
            type_index: 0,
            locals: Vec::new(),
            code: vec![Instruction::I32Const(99), Instruction::Return],
        },
    ];

    Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        }],
        imports: Vec::new(),
        functions,
        function_names: Vec::new(),
        tables: Vec::new(),
        exports: HashMap::new(),
        memory_min_pages: None,
        globals: Vec::new(),
        data_segments: Vec::new(),
        interface_defaults: Vec::new(),
        type_metadata: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
    }
}

#[test]
fn executor_handles_i64_core_ops_and_memory() {
    use Instruction::*;

    let mut code = vec![I64Const(0), LocalSet(0)];

    add_i64_accum(&mut code, &[I64Const(0), I64Eqz, I64ExtendI32U]);
    add_i64_accum(&mut code, &[I64Const(5), I64Const(3), I64Add]);
    add_i64_accum(&mut code, &[I64Const(9), I64Const(4), I64Sub]);
    add_i64_accum(&mut code, &[I64Const(2), I64Const(3), I64Mul]);
    add_i64_accum(&mut code, &[I64Const(9), I64Const(2), I64DivS]);
    add_i64_accum(&mut code, &[I64Const(9), I64Const(2), I64DivU]);
    add_i64_accum(&mut code, &[I64Const(9), I64Const(2), I64RemS]);
    add_i64_accum(&mut code, &[I64Const(9), I64Const(2), I64RemU]);
    add_i64_accum(&mut code, &[I64Const(0b1100), I64Const(0b1010), I64And]);
    add_i64_accum(&mut code, &[I64Const(0b1100), I64Const(0b1010), I64Or]);
    add_i64_accum(&mut code, &[I64Const(0b1100), I64Const(0b1010), I64Xor]);
    add_i64_accum(&mut code, &[I64Const(1), I64Const(3), I64Shl]);
    add_i64_accum(&mut code, &[I64Const(8), I64Const(1), I64ShrS]);
    add_i64_accum(&mut code, &[I64Const(0x10), I64Const(1), I64ShrU]);

    code.extend([
        I32Const(64),
        I64Const(0x1122_3344_5566_7788),
        I64Store { offset: 0 },
    ]);
    add_i64_accum(&mut code, &[I32Const(64), I64Load { offset: 0 }]);

    code.extend([LocalGet(0), Return]);

    let module = Module {
        types: vec![FuncType {
            params: Vec::new(),
            results: vec![ValueType::I64],
        }],
        imports: Vec::new(),
        functions: vec![Function {
            type_index: 0,
            locals: vec![ValueType::I64],
            code,
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

    let mut exec = Executor::new(&module);
    let result = expect_ok(exec.call(0, &[]), "i64 core ops");
    match result {
        Some(WasmValue::I64(v)) => assert_eq!(v, 1234605616436508630),
        other => panic!("unexpected result: {other:?}"),
    }
}

fn add_i64_accum(code: &mut Vec<Instruction>, expr: &[Instruction]) {
    code.push(Instruction::LocalGet(0));
    code.extend_from_slice(expr);
    code.push(Instruction::I64Add);
    code.push(Instruction::LocalSet(0));
}
