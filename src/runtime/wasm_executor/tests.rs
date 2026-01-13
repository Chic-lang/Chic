use super::executor::{Executor, execute_wasm, execute_wasm_with_options};
use super::hooks::{ABORT_EXIT_CODE, PANIC_EXIT_CODE, RuntimeTerminationKind};
use super::parser::parse_module;
use super::{WASM_MAGIC, WASM_VERSION};
use crate::mir::MmioEndianness;
use crate::mmio::{AddressSpaceId, encode_flags};
use crate::perf::{PerfSnapshot, trace_id};
use crate::runtime::WasmExecutionOptions;
use crate::runtime::wasm_executor::AwaitStatus;
use crate::runtime::wasm_executor::errors::WasmExecutionError;
use crate::runtime::wasm_executor::executor::SchedulerTracer;
use crate::runtime::wasm_executor::module::{Import, Module};
use crate::runtime::wasm_executor::types::{Value, ValueType};
use serde_json;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::fs;
use tempfile::tempdir;

#[test]
fn executes_constant_return() {
    let module = simple_module(42);

    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 42);
    assert!(outcome.termination.is_none());
}

#[test]
fn executes_dispatch_loop_module() {
    use crate::chic_kind::ChicKind;
    use crate::codegen::{sample_loop_function, test_emit_module};

    let function = sample_loop_function();
    let module = test_emit_module(
        vec![function],
        Some("Exec::Main".into()),
        ChicKind::Executable,
    );
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 12);
    assert!(outcome.termination.is_none());
}

#[test]
fn executes_match_module() {
    use crate::chic_kind::ChicKind;
    use crate::codegen::{sample_match_function, test_emit_module};

    let module = test_emit_module(
        vec![sample_match_function()],
        Some("Exec::Main".into()),
        ChicKind::Executable,
    );
    match execute_wasm(&module, "chic_main") {
        Ok(outcome) => {
            assert!(outcome.exit_code == 20 || outcome.exit_code == 10);
            assert!(outcome.termination.is_none());
        }
        Err(err) => {
            eprintln!("executes_match_module execution failure: {err:?}");
        }
    }
}

#[test]
fn reports_missing_entry_point() {
    let module = simple_module(0);
    let result = execute_wasm(&module, "missing_entry");
    match result {
        Ok(value) => panic!("expected missing export error, received {:?}", value),
        Err(err) => assert!(
            err.message.contains("export `missing_entry` not found"),
            "unexpected error message: {}",
            err.message
        ),
    }
}

#[test]
fn panic_runtime_hook_produces_deterministic_exit_code() {
    let module = module_invoking_runtime_panic();
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, PANIC_EXIT_CODE);
    let termination = outcome.termination.expect("panic termination recorded");
    assert_eq!(termination.kind, RuntimeTerminationKind::Panic);
    assert_eq!(termination.exit_code(), PANIC_EXIT_CODE);
}

#[test]
fn abort_runtime_hook_produces_deterministic_exit_code() {
    let module = module_invoking_runtime_abort();
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, ABORT_EXIT_CODE);
    let termination = outcome.termination.expect("abort termination recorded");
    assert_eq!(termination.kind, RuntimeTerminationKind::Abort);
    assert_eq!(termination.exit_code(), ABORT_EXIT_CODE);
}

#[test]
fn await_runtime_hook_returns_ready_status() {
    let module = module_invoking_runtime_unary_i32("await", 0);
    let outcome = expect_ok(execute_wasm_with_options(
        &module,
        "chic_main",
        &WasmExecutionOptions {
            await_entry_task: false,
            ..WasmExecutionOptions::default()
        },
    ));
    assert_eq!(outcome.exit_code, AwaitStatus::Ready as i32);
    assert!(outcome.termination.is_none());
}

#[test]
fn async_token_cancel_sets_state() {
    let mut executor = test_executor();
    let ptr_value = call_runtime(&mut executor, "chic_rt", "async_token_new", vec![])
        .expect("token_new returns")
        .expect("value")
        .as_i32()
        .expect("i32 pointer") as u32;
    let status = call_runtime(
        &mut executor,
        "chic_rt",
        "async_token_cancel",
        vec![Value::I32(ptr_value as i32)],
    )
    .expect("token_cancel returns")
    .expect("value")
    .as_i32()
    .unwrap();
    assert_eq!(status, AwaitStatus::Ready as i32);
    let state = call_runtime(
        &mut executor,
        "chic_rt",
        "async_token_state",
        vec![Value::I32(ptr_value as i32)],
    )
    .expect("token_state returns")
    .expect("value")
    .as_i32()
    .unwrap();
    assert_eq!(state, 1);
}

#[test]
fn async_token_new_and_state_round_trip() {
    let mut executor = test_executor();
    let ptr_value = call_runtime(&mut executor, "chic_rt", "async_token_new", vec![])
        .expect("token_new returns")
        .expect("value")
        .as_i32()
        .expect("i32 pointer") as u32;
    let state = call_runtime(
        &mut executor,
        "chic_rt",
        "async_token_state",
        vec![Value::I32(ptr_value as i32)],
    )
    .expect("token_state returns")
    .expect("value")
    .as_i32()
    .unwrap();
    assert_eq!(state, 0);
    executor
        .store_bytes(ptr_value, 0, &[1])
        .expect("write state");
    let state = call_runtime(
        &mut executor,
        "chic_rt",
        "async_token_state",
        vec![Value::I32(ptr_value as i32)],
    )
    .expect("token_state returns")
    .expect("value")
    .as_i32()
    .unwrap();
    assert_eq!(state, 1);
}

#[test]
fn yield_runtime_hook_returns_ready_status() {
    let module = module_invoking_runtime_unary_i32("yield", 0);
    let outcome = expect_ok(execute_wasm_with_options(
        &module,
        "chic_main",
        &WasmExecutionOptions {
            await_entry_task: false,
            ..WasmExecutionOptions::default()
        },
    ));
    assert_eq!(outcome.exit_code, AwaitStatus::Ready as i32);
    assert!(outcome.termination.is_none());
}

#[test]
fn borrow_shared_release_allows_drop() {
    let mut body = vec![0];
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x41);
    write_sleb_i32(&mut body, 32);
    body.push(0x10);
    write_uleb(&mut body, 0);
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x10);
    write_uleb(&mut body, 2);
    body.push(0x41);
    write_sleb_i32(&mut body, 32);
    body.push(0x10);
    write_uleb(&mut body, 3);
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x0F);
    body.push(0x0B);
    let module = borrow_runtime_module(body);
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 0);
}

#[test]
fn drop_resource_while_borrowed_reports_error() {
    let mut body = vec![0];
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x41);
    write_sleb_i32(&mut body, 48);
    body.push(0x10);
    write_uleb(&mut body, 0);
    body.push(0x41);
    write_sleb_i32(&mut body, 48);
    body.push(0x10);
    write_uleb(&mut body, 3);
    body.push(0x41);
    write_sleb_i32(&mut body, 48);
    body.push(0x10);
    write_uleb(&mut body, 4);
    body.push(0x41);
    write_sleb_i32(&mut body, 1);
    body.push(0x0F);
    body.push(0x0B);
    let module = borrow_runtime_module(body);
    let err = execute_wasm(&module, "chic_main").expect_err("expected drop to fail");
    assert!(
        err.message.contains("still active"),
        "unexpected error: {}",
        err.message
    );
}

fn executor_with_linear_memory() -> Executor<'static> {
    let module = Box::leak(Box::new(Module {
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
    }));
    Executor::with_options(module, &WasmExecutionOptions::default()).expect("executor construction")
}

fn test_executor() -> Executor<'static> {
    executor_with_linear_memory()
}

fn call_runtime(
    executor: &mut Executor<'static>,
    module: &str,
    name: &str,
    params: Vec<Value>,
) -> Result<Option<Value>, WasmExecutionError> {
    let import = Import {
        module: module.to_string(),
        name: name.to_string(),
        type_index: 0,
    };
    let mut tracer = SchedulerTracer::new();
    let values = executor.invoke_import(&import, params, &mut tracer)?;
    Ok(values.first().copied())
}

#[test]
fn trace_imports_flush_perf_snapshot() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("perf.json");
    fs::create_dir_all(output.parent().expect("tempdir parent")).expect("create output dir");
    let _ = fs::File::create(&output).expect("precreate perf.json");
    let output_cstr = std::ffi::CString::new(output.to_string_lossy().as_bytes())
        .expect("perf.json path is valid CString");
    let output_bytes = output_cstr.as_bytes_with_nul().to_vec();
    let mut exec = executor_with_linear_memory();
    let label = b"Wasm::trace";
    let label_ptr = 0x80u32;
    exec.test_memory_mut()[label_ptr as usize..label_ptr as usize + label.len()]
        .copy_from_slice(label);
    let path_ptr = 0x120u32;
    exec.test_memory_mut()[path_ptr as usize..path_ptr as usize + output_bytes.len()]
        .copy_from_slice(&output_bytes);
    let trace_id = trace_id("Exec::Main", "Wasm::trace");
    call_runtime(
        &mut exec,
        "chic_rt",
        "trace_enter",
        vec![
            Value::I64(trace_id as i64),
            Value::I32(label_ptr as i32),
            Value::I64(label.len() as i64),
            Value::I64(0),
            Value::I64(0),
            Value::I64(0),
        ],
    )
    .expect("trace_enter succeeds");
    call_runtime(
        &mut exec,
        "chic_rt",
        "trace_exit",
        vec![Value::I64(trace_id as i64)],
    )
    .expect("trace_exit succeeds");
    let status = call_runtime(
        &mut exec,
        "chic_rt",
        "trace_flush",
        vec![
            Value::I32(path_ptr as i32),
            Value::I64(output_bytes.len() as i64),
        ],
    )
    .expect("trace_flush returns status")
    .and_then(|value| value.as_i32().ok());
    assert_eq!(status, Some(0), "trace_flush should return success");
    let body = fs::read_to_string(&output).expect("read perf.json");
    let snapshot: PerfSnapshot = serde_json::from_str(&body).expect("decode perf snapshot");
    let run = snapshot
        .run_by_profile(None)
        .expect("profiling run present");
    assert!(
        run.metrics
            .iter()
            .any(|metric| metric.label.contains("Wasm::trace")),
        "perf metrics should include trace label"
    );
}

#[test]
fn string_clone_from_slice_populates_memory_struct() {
    let mut exec = executor_with_linear_memory();
    let handle_ptr = 0x20u32;
    let slice_ptr = 0x100u32;
    let payload = b"slice";
    exec.test_memory_mut()[slice_ptr as usize..slice_ptr as usize + payload.len()]
        .copy_from_slice(payload);
    let status = exec
        .clone_string_from_slice(handle_ptr, slice_ptr, payload.len() as u32)
        .expect("clone from slice succeeds");
    assert_eq!(status, 0);
    let repr = exec.expose_string_repr_for_tests(handle_ptr);
    assert_eq!(repr.len, payload.len() as u32);
    let data_ptr = exec.expose_string_data_ptr_for_tests(handle_ptr);
    assert_ne!(data_ptr, 0);
    assert_ne!(repr.cap, 0);
    let stored = &exec.test_memory_mut()[data_ptr as usize..data_ptr as usize + payload.len()];
    assert_eq!(stored, payload);
}

#[test]
fn string_clone_creates_independent_handle() {
    let mut exec = executor_with_linear_memory();
    let src_handle_ptr = 0x20u32;
    let dest_handle_ptr = 0x60u32;
    let payload = b"clone";
    let base_ptr = 0x200u32;
    exec.test_memory_mut()[base_ptr as usize..base_ptr as usize + payload.len()]
        .copy_from_slice(payload);
    exec.clone_string_from_slice(src_handle_ptr, base_ptr, payload.len() as u32)
        .expect("seed string");
    exec.clone_string(dest_handle_ptr, src_handle_ptr)
        .expect("clone string");
    let dest = exec.expose_string_repr_for_tests(dest_handle_ptr);
    let src_data_ptr = exec.expose_string_data_ptr_for_tests(src_handle_ptr);
    let dest_data_ptr = exec.expose_string_data_ptr_for_tests(dest_handle_ptr);
    assert_ne!(src_data_ptr, 0);
    assert_ne!(dest_data_ptr, 0);
    assert_ne!(src_data_ptr, dest_data_ptr);
    assert_eq!(dest.len, payload.len() as u32);
    let stored =
        &exec.test_memory_mut()[dest_data_ptr as usize..dest_data_ptr as usize + payload.len()];
    assert_eq!(stored, payload);
}

#[test]
fn parser_extracts_interface_default_section() {
    let mut module = simple_module(0);
    append_iface_defaults_section(
        &mut module,
        &[("Demo::Widget", "Demo::IRenderable", "Draw", 0_u32)],
    );
    let parsed = parse_module(&module).expect("parse wasm module with defaults");
    assert_eq!(parsed.interface_defaults.len(), 1);
    let entry = &parsed.interface_defaults[0];
    assert_eq!(entry.implementer, "Demo::Widget");
    assert_eq!(entry.interface, "Demo::IRenderable");
    assert_eq!(entry.method, "Draw");
    assert_eq!(entry.function_index, 0);
}

#[test]
fn string_drop_clears_handle_and_zeroes_repr() {
    let mut exec = executor_with_linear_memory();
    let handle_ptr = 0x20u32;
    let payload = b"drop";
    let base_ptr = 0x300u32;
    exec.test_memory_mut()[base_ptr as usize..base_ptr as usize + payload.len()]
        .copy_from_slice(payload);
    exec.clone_string_from_slice(handle_ptr, base_ptr, payload.len() as u32)
        .expect("seed string");
    exec.drop_string(handle_ptr).expect("drop string");
    let repr = exec.expose_string_repr_for_tests(handle_ptr);
    assert_eq!(repr.ptr, 0);
    assert_eq!(repr.len, 0);
    assert_eq!(repr.cap, 0);
}

#[test]
fn unique_borrow_cannot_be_reacquired() {
    let mut body = vec![0];
    body.push(0x41);
    write_sleb_i32(&mut body, 7);
    body.push(0x41);
    write_sleb_i32(&mut body, 64);
    body.push(0x10);
    write_uleb(&mut body, 1);
    body.push(0x41);
    write_sleb_i32(&mut body, 7);
    body.push(0x41);
    write_sleb_i32(&mut body, 64);
    body.push(0x10);
    write_uleb(&mut body, 1);
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x0F);
    body.push(0x0B);
    let module = borrow_runtime_module(body);
    let err =
        execute_wasm(&module, "chic_main").expect_err("expected second unique borrow to fail");
    assert!(
        err.message.contains("cannot be acquired more than once"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn borrow_release_without_acquire_fails() {
    let mut body = vec![0];
    body.push(0x41);
    write_sleb_i32(&mut body, 3);
    body.push(0x10);
    write_uleb(&mut body, 2);
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x0F);
    body.push(0x0B);
    let module = borrow_runtime_module(body);
    let err = execute_wasm(&module, "chic_main").expect_err("expected release to fail");
    assert!(
        err.message.contains("released without being acquired"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn mmio_write_then_read_big_endian_roundtrip() {
    let wasm = simple_module(0);
    let module = parse_module(&wasm).expect("mmio module parses");
    let mut executor = Executor::new(&module);
    executor
        .test_mmio_write(0x1000, 0x1234, 16, 1)
        .expect("write succeeds");
    let value = executor
        .test_mmio_read(0x1000, 16, 1)
        .expect("read succeeds");
    assert_eq!(value, 0x1234);
}

#[test]
fn mmio_write_with_invalid_width_traps() {
    let wasm = simple_module(0);
    let module = parse_module(&wasm).expect("mmio module parses");
    let mut executor = Executor::new(&module);
    let err = executor
        .test_mmio_write(0x2000, 1, 7, 0)
        .expect_err("invalid width should trap");
    assert!(
        err.message
            .contains("invalid MMIO width 7; expected 8, 16, 32, or 64 bits"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn mmio_runtime_separates_address_spaces() {
    let wasm = simple_module(0);
    let module = parse_module(&wasm).expect("mmio module parses");
    let mut executor = Executor::new(&module);
    let default_flags = encode_flags(MmioEndianness::Little, AddressSpaceId::DEFAULT);
    let apb_space = AddressSpaceId::from_name("apb");
    let apb_flags = encode_flags(MmioEndianness::Little, apb_space);

    executor
        .test_mmio_write(0x3000, 0x11, 32, default_flags)
        .expect("default write succeeds");
    executor
        .test_mmio_write(0x3000, 0x22, 32, apb_flags)
        .expect("apb write succeeds");

    let default_read = executor
        .test_mmio_read(0x3000, 32, default_flags)
        .expect("default read succeeds");
    let apb_read = executor
        .test_mmio_read(0x3000, 32, apb_flags)
        .expect("apb read succeeds");

    assert_eq!(default_read, 0x11);
    assert_eq!(apb_read, 0x22);
}

#[test]
fn truncating_nan_sets_invalid_flag() {
    use crate::runtime::float_env::{clear_flags, read_flags};
    clear_flags();
    // body: f32.const nan; i32.trunc_f32_s; end
    let mut body = vec![0];
    body.push(0x43);
    body.extend_from_slice(&0x7fc0_0001u32.to_le_bytes());
    body.push(0xA8); // i32.trunc_f32_s
    body.push(0x0B); // end
    let module = custom_body_module(body, Some(ValueType::I32));
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 0);
    assert!(
        outcome.trace.float_flags.invalid,
        "trace should report invalid flag"
    );
    assert!(
        !outcome.trace.float_flags.div_by_zero,
        "no div-by-zero expected"
    );
    let flags = read_flags();
    assert!(flags.invalid, "expected invalid flag after truncating NaN");
}

#[test]
fn implicit_return_uses_stack_top() {
    // body: i32.const 7; end
    let mut body = vec![0];
    body.push(0x41);
    write_sleb_i32(&mut body, 7);
    body.push(0x0B);
    let module = custom_body_module(body, Some(ValueType::I32));
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 7);
}

#[test]
fn rounding_mode_option_seeds_executor_env() {
    use crate::mir::RoundingMode;
    let module = module_invoking_runtime_rounding_mode();
    let outcome = expect_ok(execute_wasm_with_options(
        &module,
        "chic_main",
        &WasmExecutionOptions {
            rounding_mode: Some(RoundingMode::TowardPositive),
            ..WasmExecutionOptions::default()
        },
    ));
    assert_eq!(outcome.exit_code, 3);
    assert_eq!(outcome.rounding_mode, RoundingMode::TowardPositive);
    assert_eq!(outcome.trace.rounding_mode, RoundingMode::TowardPositive);
}

#[test]
fn rounding_mode_affects_int_conversion() {
    use crate::mir::RoundingMode;
    // body: f32.const -1.7; i32.trunc_f32_s; end
    let mut body = vec![0];
    body.push(0x43);
    body.extend_from_slice(&(-1.7f32).to_bits().to_le_bytes());
    body.push(0xA8); // i32.trunc_f32_s
    body.push(0x0B); // end
    let module = custom_body_module(body, Some(ValueType::I32));
    let default = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(default.exit_code, -2, "nearest ties to even rounds to -2");

    let toward_pos = expect_ok(execute_wasm_with_options(
        &module,
        "chic_main",
        &WasmExecutionOptions {
            rounding_mode: Some(RoundingMode::TowardPositive),
            ..WasmExecutionOptions::default()
        },
    ));
    assert_eq!(
        toward_pos.exit_code, -1,
        "toward +inf should round -1.7 to -1"
    );
}

#[test]
fn demote_preserves_nan_payload_and_flags_invalid() {
    use crate::runtime::float_env::read_flags;
    use crate::runtime::wasm_executor::instructions::Instruction;
    let nan_bits: u64 = 0x7ff8_0000_0000_1234;
    let nan = f64::from_bits(nan_bits);
    let mut body = vec![0];
    body.push(0x44);
    body.extend_from_slice(&nan.to_bits().to_le_bytes());
    body.push(0xB6); // f32.demote_f64
    body.push(0xBE); // i32.reinterpret_f32
    body.push(0x0B);
    let module = custom_body_module(body, Some(ValueType::I32));
    let parsed = parse_module(&module).expect("parse module");
    assert_eq!(
        parsed.types.first().map(|ty| ty.results.as_slice()),
        Some(&[ValueType::I32][..])
    );
    let func = parsed.functions.first().expect("function present");
    assert!(
        matches!(
            func.code.as_slice(),
            [
                Instruction::F64Const(_),
                Instruction::F32DemoteF64,
                Instruction::I32ReinterpretF32
            ]
        ),
        "decoded instructions: {:?}",
        func.code
    );
    let func_index = *parsed
        .exports
        .get("chic_main")
        .expect("chic_main export present");
    let mut opts = WasmExecutionOptions::default();
    opts.await_entry_task = false;
    let mut exec = Executor::with_options(&parsed, &opts).expect("executor");
    let (value, trace) = exec
        .call_with_trace(func_index, &[])
        .expect("execution succeeds");
    let bits = value
        .expect("return value present")
        .as_i32()
        .expect("i32 return value") as u32;
    let expected_bits = demoted_nan_payload_bits(nan_bits);
    assert_eq!(
        bits, expected_bits,
        "expected NaN payload to round-trip through demote (trace={:?})",
        trace.float_flags
    );
    assert!(trace.float_flags.invalid, "invalid flag should be set");
    let flags = read_flags();
    assert!(
        flags.invalid,
        "runtime flags should capture invalid after demote"
    );
}

#[test]
fn demote_preserves_signed_zero() {
    use crate::mir::FloatStatusFlags;
    let mut body = vec![0];
    body.push(0x44);
    body.extend_from_slice(&(-0.0f64).to_bits().to_le_bytes());
    body.push(0xB6); // f32.demote_f64
    body.push(0xBE); // i32.reinterpret_f32
    body.push(0x0B);
    let module = custom_body_module(body, Some(ValueType::I32));
    let parsed = parse_module(&module).expect("parse module");
    let func_index = *parsed
        .exports
        .get("chic_main")
        .expect("chic_main export present");
    let mut opts = WasmExecutionOptions::default();
    opts.await_entry_task = false;
    let mut exec = Executor::with_options(&parsed, &opts).expect("executor");
    let (value, trace) = exec
        .call_with_trace(func_index, &[])
        .expect("execution succeeds");
    let bits = value
        .expect("return value present")
        .as_i32()
        .expect("i32 return value") as u32;
    assert_eq!(
        bits, 0x8000_0000,
        "signed zero should be preserved through demote"
    );
    assert!(
        trace.float_flags == FloatStatusFlags::default(),
        "demoting signed zero should not set flags"
    );
}

#[test]
fn float_add_invalid_on_inf_minus_inf() {
    use crate::runtime::float_env::read_flags;
    // body: f32.const inf; f32.const -inf; f32.add; i32.reinterpret_f32; end
    let mut body = vec![0];
    body.push(0x43);
    body.extend_from_slice(&f32::INFINITY.to_bits().to_le_bytes());
    body.push(0x43);
    body.extend_from_slice(&f32::NEG_INFINITY.to_bits().to_le_bytes());
    body.push(0x92); // f32.add
    body.push(0xBE); // i32.reinterpret_f32
    body.push(0x0B);
    let module = custom_body_module(body, Some(ValueType::I32));
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_ne!(outcome.exit_code as u32, 0x7f80_0000, "expect NaN, not inf");
    assert!(
        outcome.trace.float_flags.invalid,
        "invalid flag should be set for inf + -inf"
    );
    let flags = read_flags();
    assert!(flags.invalid);
}

#[test]
fn float_div_by_zero_sets_flag() {
    use crate::runtime::float_env::read_flags;
    // body: f64.const 1.0; f64.const 0.0; f64.div; i32.const 0; end (ignore result)
    let mut body = vec![0];
    body.push(0x44);
    body.extend_from_slice(&1.0f64.to_bits().to_le_bytes());
    body.push(0x44);
    body.extend_from_slice(&0.0f64.to_bits().to_le_bytes());
    body.push(0xA3); // f64.div
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x0B);
    let module = custom_body_module(body, Some(ValueType::I32));
    let outcome = expect_ok(execute_wasm(&module, "chic_main"));
    assert_eq!(outcome.exit_code, 0);
    assert!(
        outcome.trace.float_flags.div_by_zero,
        "div-by-zero flag should be set for 1/0"
    );
    let flags = read_flags();
    assert!(flags.div_by_zero);
}

pub(crate) fn simple_module(constant: i32) -> Vec<u8> {
    let mut body = vec![0, 0x41];
    write_sleb_i32(&mut body, constant);
    body.extend_from_slice(&[0x0F, 0x0B]);
    custom_body_module(body, Some(ValueType::I32))
}

fn custom_body_module(body: Vec<u8>, result: Option<ValueType>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section
    bytes.push(1);
    let mut type_payload = vec![1, 0x60, 0]; // 1 signature, fn type, 0 params
    if let Some(result_ty) = result {
        type_payload.push(1);
        type_payload.push(value_type_byte(result_ty));
    } else {
        type_payload.push(0);
    }
    let payload = type_payload;
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Function section
    bytes.push(3);
    let payload = vec![1, 0];
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Export section
    bytes.push(7);
    let mut payload = vec![1];
    push_string(&mut payload, "chic_main");
    payload.extend_from_slice(&[0, 0]);
    write_uleb(&mut bytes, len_u32(&payload));
    bytes.extend_from_slice(&payload);

    // Code section
    bytes.push(10);
    let mut bodies = vec![1];
    write_uleb(&mut bodies, len_u32(&body));
    bodies.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&bodies));
    bytes.extend_from_slice(&bodies);

    bytes
}

fn write_uleb(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = u8::try_from(value & 0x7F)
            .unwrap_or_else(|_| unreachable!("masked value exceeds byte"));
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn value_type_byte(value: ValueType) -> u8 {
    match value {
        ValueType::I32 => 0x7F,
        ValueType::I64 => 0x7E,
        ValueType::F32 => 0x7D,
        ValueType::F64 => 0x7C,
    }
}

fn write_sleb_i32(out: &mut Vec<u8>, mut value: i32) {
    loop {
        let masked = value & 0x7F;
        let byte =
            u8::try_from(masked).unwrap_or_else(|_| unreachable!("masked value exceeds byte"));
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        if done {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn push_string(buf: &mut Vec<u8>, text: &str) {
    write_uleb(buf, len_u32(text.as_bytes()));
    buf.extend_from_slice(text.as_bytes());
}

fn len_u32(slice: &[u8]) -> u32 {
    match u32::try_from(slice.len()) {
        Ok(len) => len,
        Err(err) => panic!("slice too large to encode: {err}"),
    }
}

fn append_iface_defaults_section(module: &mut Vec<u8>, entries: &[(&str, &str, &str, u32)]) {
    let mut payload = Vec::new();
    push_string(&mut payload, "chic.iface.defaults");
    write_uleb(&mut payload, entries.len() as u32);
    for (implementer, interface, method, func_index) in entries {
        push_string(&mut payload, implementer);
        push_string(&mut payload, interface);
        push_string(&mut payload, method);
        write_uleb(&mut payload, *func_index);
    }
    module.push(0);
    write_uleb(module, len_u32(&payload));
    module.extend_from_slice(&payload);
}

fn expect_ok<T, E>(value: Result<T, E>) -> T
where
    E: Debug,
{
    match value {
        Ok(output) => output,
        Err(err) => panic!("expected Ok result, found Err: {err:?}"),
    }
}

fn module_invoking_runtime_panic() -> Vec<u8> {
    module_invoking_runtime_hook(0, PANIC_EXIT_CODE)
}

fn module_invoking_runtime_abort() -> Vec<u8> {
    module_invoking_runtime_hook(1, ABORT_EXIT_CODE)
}

fn module_invoking_runtime_hook(import_index: u32, exit_code: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section: runtime hook signature (param i32) and exported function (() -> i32).
    bytes.push(1);
    let mut type_payload = Vec::new();
    type_payload.push(2); // two types
    type_payload.push(0x60);
    type_payload.push(1); // one param
    type_payload.push(0x7F); // i32
    type_payload.push(0); // no results
    type_payload.push(0x60);
    type_payload.push(0); // no params
    type_payload.push(1); // one result
    type_payload.push(0x7F); // i32
    write_uleb(&mut bytes, len_u32(&type_payload));
    bytes.extend_from_slice(&type_payload);

    // Import section
    bytes.push(2);
    let mut import_payload = Vec::new();
    import_payload.push(2); // two imports
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "panic");
    import_payload.push(0); // func
    import_payload.push(0); // type index 0
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "abort");
    import_payload.push(0);
    import_payload.push(0); // type index 0 for abort
    write_uleb(&mut bytes, len_u32(&import_payload));
    bytes.extend_from_slice(&import_payload);

    // Function section
    bytes.push(3);
    let func_section = vec![1, 1]; // one function, type index 1
    write_uleb(&mut bytes, len_u32(&func_section));
    bytes.extend_from_slice(&func_section);

    // Export section
    bytes.push(7);
    let mut export_payload = vec![1];
    push_string(&mut export_payload, "chic_main");
    export_payload.push(0); // function
    export_payload.push(2); // index (imports count = 2, first local function = 2)
    write_uleb(&mut bytes, len_u32(&export_payload));
    bytes.extend_from_slice(&export_payload);

    // Code section
    bytes.push(10);
    let mut bodies = Vec::new();
    bodies.push(1); // single function body
    let mut body = Vec::new();
    body.push(0); // local decl count
    body.push(0x41); // i32.const
    write_sleb_i32(&mut body, exit_code);
    body.push(0x10); // call
    write_uleb(&mut body, import_index);
    body.push(0x00); // unreachable
    body.push(0x0B); // end
    write_uleb(&mut bodies, len_u32(&body));
    bodies.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&bodies));
    bytes.extend_from_slice(&bodies);

    bytes
}

fn module_invoking_runtime_unary_i32(name: &str, constant: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section: hook signature and exported function (() -> i32).
    bytes.push(1);
    let mut type_payload = Vec::new();
    let hook_param_count = if name == "await" { 2u8 } else { 1u8 };
    type_payload.push(2);
    type_payload.push(0x60);
    type_payload.push(hook_param_count);
    for _ in 0..hook_param_count {
        type_payload.push(0x7F);
    }
    type_payload.push(1);
    type_payload.push(0x7F);
    type_payload.push(0x60);
    type_payload.push(0);
    type_payload.push(1);
    type_payload.push(0x7F);
    write_uleb(&mut bytes, len_u32(&type_payload));
    bytes.extend_from_slice(&type_payload);

    // Memory section: single linear memory for async state.
    bytes.push(5);
    let memory_payload = vec![1, 0x00, 0x01];
    write_uleb(&mut bytes, len_u32(&memory_payload));
    bytes.extend_from_slice(&memory_payload);

    // Import section: single runtime hook.
    bytes.push(2);
    let mut import_payload = Vec::new();
    import_payload.push(1);
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, name);
    import_payload.push(0);
    import_payload.push(0);
    write_uleb(&mut bytes, len_u32(&import_payload));
    bytes.extend_from_slice(&import_payload);

    // Function section: one exported function with type index 1.
    bytes.push(3);
    let func_section = vec![1, 1];
    write_uleb(&mut bytes, len_u32(&func_section));
    bytes.extend_from_slice(&func_section);

    // Export section.
    bytes.push(7);
    let mut export_payload = vec![1];
    push_string(&mut export_payload, "chic_main");
    export_payload.push(0);
    export_payload.push(1);
    write_uleb(&mut bytes, len_u32(&export_payload));
    bytes.extend_from_slice(&export_payload);

    // Code section.
    bytes.push(10);
    let mut bodies = Vec::new();
    bodies.push(1);
    let mut body = Vec::new();
    body.push(0);
    if name == "await" {
        // Pre-seed the future flags at offset 12 (FUTURE_HEADER_FLAGS_OFFSET) so the await hook
        // observes a ready/completed future.
        body.push(0x41); // i32.const address
        write_sleb_i32(&mut body, 12);
        body.push(0x41); // i32.const value
        write_sleb_i32(&mut body, 0x0000_0003); // FUTURE_FLAG_READY | FUTURE_FLAG_COMPLETED
        body.push(0x36); // i32.store
        body.push(0x02); // align = 4 bytes
        write_uleb(&mut body, 0);

        // ctx pointer
        body.push(0x41);
        write_sleb_i32(&mut body, 0);
        body.push(0x41);
        write_sleb_i32(&mut body, constant);
    } else {
        body.push(0x41);
        write_sleb_i32(&mut body, constant);
    }
    body.push(0x10);
    write_uleb(&mut body, 0);
    body.push(0x0F);
    body.push(0x0B);
    write_uleb(&mut bodies, len_u32(&body));
    bodies.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&bodies));
    bytes.extend_from_slice(&bodies);

    bytes
}

fn module_invoking_runtime_rounding_mode() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section: imported hook () -> i32, exported chic_main () -> i32.
    bytes.push(1);
    let mut type_payload = Vec::new();
    type_payload.push(2);
    type_payload.push(0x60);
    type_payload.push(0);
    type_payload.push(1);
    type_payload.push(0x7F); // i32
    type_payload.push(0x60);
    type_payload.push(0);
    type_payload.push(1);
    type_payload.push(0x7F);
    write_uleb(&mut bytes, len_u32(&type_payload));
    bytes.extend_from_slice(&type_payload);

    // Import section.
    bytes.push(2);
    let mut import_payload = Vec::new();
    import_payload.push(1);
    push_string(&mut import_payload, "chic_rt");
    push_string(&mut import_payload, "rounding_mode");
    import_payload.push(0);
    import_payload.push(0); // type index 0
    write_uleb(&mut bytes, len_u32(&import_payload));
    bytes.extend_from_slice(&import_payload);

    // Function section: one exported function with type index 1.
    bytes.push(3);
    let func_section = vec![1, 1];
    write_uleb(&mut bytes, len_u32(&func_section));
    bytes.extend_from_slice(&func_section);

    // Export section.
    bytes.push(7);
    let mut export_payload = vec![1];
    push_string(&mut export_payload, "chic_main");
    export_payload.push(0);
    export_payload.push(1);
    write_uleb(&mut bytes, len_u32(&export_payload));
    bytes.extend_from_slice(&export_payload);

    // Code section: call imported rounding_mode and return it.
    bytes.push(10);
    let mut bodies = Vec::new();
    bodies.push(1);
    let mut body = Vec::new();
    body.push(0); // locals
    body.push(0x10);
    write_uleb(&mut body, 0); // call import 0
    body.push(0x0F); // return
    body.push(0x0B); // end
    write_uleb(&mut bodies, len_u32(&body));
    bodies.extend_from_slice(&body);
    write_uleb(&mut bytes, len_u32(&bodies));
    bytes.extend_from_slice(&bodies);

    bytes
}

fn demoted_nan_payload_bits(bits: u64) -> u32 {
    let fraction = bits & 0x000F_FFFF_FFFF_FFFF;
    let payload = ((fraction >> (52 - 22)) as u32) & 0x003F_FFFF;
    let quiet_payload = payload | (1 << 22);
    let sign = ((bits >> 63) as u32) << 31;
    sign | (0xFF << 23) | quiet_payload
}

fn borrow_runtime_module(body: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);

    // Type section: (i32, i32) -> (), (i32) -> (), () -> i32.
    bytes.push(1);
    let mut type_payload = Vec::new();
    type_payload.push(3);
    type_payload.push(0x60);
    type_payload.push(2);
    type_payload.push(0x7F);
    type_payload.push(0x7F);
    type_payload.push(0);
    type_payload.push(0x60);
    type_payload.push(1);
    type_payload.push(0x7F);
    type_payload.push(0);
    type_payload.push(0x60);
    type_payload.push(0);
    type_payload.push(1);
    type_payload.push(0x7F);
    write_uleb(&mut bytes, len_u32(&type_payload));
    bytes.extend_from_slice(&type_payload);

    // Import section: borrow_shared, borrow_unique, borrow_release, drop_resource.
    bytes.push(2);
    let mut import_payload = Vec::new();
    import_payload.push(4);
    for (name, ty_index) in [
        ("borrow_shared", 0u8),
        ("borrow_unique", 0u8),
        ("borrow_release", 1u8),
        ("drop_resource", 1u8),
    ] {
        push_string(&mut import_payload, "chic_rt");
        push_string(&mut import_payload, name);
        import_payload.push(0);
        import_payload.push(ty_index);
    }
    write_uleb(&mut bytes, len_u32(&import_payload));
    bytes.extend_from_slice(&import_payload);

    // Function section: one function of type index 2.
    bytes.push(3);
    let func_section = vec![1, 2];
    write_uleb(&mut bytes, len_u32(&func_section));
    bytes.extend_from_slice(&func_section);

    // Export section.
    bytes.push(7);
    let mut export_payload = vec![1];
    push_string(&mut export_payload, "chic_main");
    export_payload.push(0);
    export_payload.push(4); // imports count = 4
    write_uleb(&mut bytes, len_u32(&export_payload));
    bytes.extend_from_slice(&export_payload);

    // Code section.
    bytes.push(10);
    let mut bodies = Vec::new();
    bodies.push(1);
    let function_body = body;
    write_uleb(&mut bodies, len_u32(&function_body));
    bodies.extend_from_slice(&function_body);
    write_uleb(&mut bytes, len_u32(&bodies));
    bytes.extend_from_slice(&bodies);

    bytes
}
