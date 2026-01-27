use std::path::PathBuf;
use std::time::Duration;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/testdate/",
            $name
        ))
    };
}

pub(crate) use fixture;

pub(crate) const WASM_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn async_stdlib_stub() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate/stdlib_async_stub.ch")
}

pub(crate) fn function_pointer_program() -> &'static str {
    fixture!("function_pointer.ch")
}

pub(crate) fn const_program() -> &'static str {
    fixture!("const.ch")
}

pub(crate) fn ref_parameter_program() -> &'static str {
    fixture!("ref_parameter.ch")
}

pub(crate) fn span_program() -> &'static str {
    fixture!("span.ch")
}

pub(crate) fn utf8_span_program() -> &'static str {
    fixture!("utf8/utf8_span.ch")
}

pub(crate) fn numeric_pointer_format_program() -> &'static str {
    fixture!("numeric/pointer_format.ch")
}

pub(crate) fn null_conditional_assignment_program() -> &'static str {
    fixture!("null_conditional_assignment.ch")
}

pub(crate) fn io_stackalloc_program() -> &'static str {
    fixture!("io_stackalloc.ch")
}

pub(crate) fn virtual_dispatch_program() -> &'static str {
    fixture!("virtual_dispatch.ch")
}

pub(crate) fn core_option_result_program() -> &'static str {
    fixture!("core_option_result.ch")
}

pub(crate) fn local_function_program() -> &'static str {
    fixture!("local_functions.ch")
}

pub(crate) fn optional_parameters_program() -> &'static str {
    include_str!("../spec/optional_parameters.ch")
}

pub(crate) fn advanced_pattern_program() -> &'static str {
    fixture!("advanced_patterns.ch")
}

pub(crate) fn bool_main_true() -> &'static str {
    fixture!("bool_main_true.ch")
}

pub(crate) fn bool_main_false() -> &'static str {
    fixture!("bool_main_false.ch")
}

pub(crate) fn llvm_factorial_program() -> &'static str {
    fixture!("llvm_factorial.ch")
}

pub(crate) fn complex_control_flow_program() -> &'static str {
    fixture!("complex_control_flow.ch")
}

pub(crate) fn guarded_match_program() -> &'static str {
    fixture!("guarded_match.ch")
}

pub(crate) fn wasm_test_runner_program() -> &'static str {
    fixture!("wasm_test_runner.ch")
}

pub(crate) fn llvm_test_runner_program() -> &'static str {
    fixture!("llvm_test_harness.ch")
}

pub(crate) fn string_interpolation_program() -> &'static str {
    fixture!("string_interpolation.ch")
}

pub(crate) fn unicode_identifiers_program() -> &'static str {
    fixture!("unicode_identifiers.ch")
}

pub(crate) fn unicode_identifiers_defs_program() -> &'static str {
    fixture!("unicode_identifiers_defs.ch")
}

pub(crate) fn async_entry_program() -> &'static str {
    fixture!("async_entry.ch")
}

pub(crate) fn async_testcases_program() -> &'static str {
    fixture!("async_testcases.ch")
}

pub(crate) fn async_cancellation_program() -> &'static str {
    fixture!("async_cancellation.ch")
}

pub(crate) fn wasm_properties_program() -> &'static str {
    fixture!("wasm_properties.ch")
}

pub(crate) fn simple_return_module(value: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
    bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    // type section with one function (no params, i32 result)
    bytes.push(1);
    let type_payload = vec![1, 0x60, 0, 1, 0x7F];
    write_uleb(&mut bytes, type_payload.len() as u32);
    bytes.extend_from_slice(&type_payload);

    // function section referencing type 0
    bytes.push(3);
    let func_payload = vec![1, 0];
    write_uleb(&mut bytes, func_payload.len() as u32);
    bytes.extend_from_slice(&func_payload);

    // export section exporting chic_main
    bytes.push(7);
    let mut export_payload = vec![1];
    write_uleb(&mut export_payload, "chic_main".len() as u32);
    export_payload.extend_from_slice("chic_main".as_bytes());
    export_payload.push(0); // function
    export_payload.push(0); // index
    write_uleb(&mut bytes, export_payload.len() as u32);
    bytes.extend_from_slice(&export_payload);

    // code section with single body
    bytes.push(10);
    let mut code_payload = Vec::new();
    code_payload.push(1); // function count
    let mut body = Vec::new();
    body.push(0); // locals
    body.push(0x41);
    write_sleb_i32(&mut body, value);
    body.push(0x0F);
    body.push(0x0B);
    write_uleb(&mut code_payload, body.len() as u32);
    code_payload.extend_from_slice(&body);
    write_uleb(&mut bytes, code_payload.len() as u32);
    bytes.extend_from_slice(&code_payload);

    bytes
}

fn write_uleb(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
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

fn write_sleb_i32(out: &mut Vec<u8>, mut value: i32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        if done {
            out.push(byte);
            break;
        } else {
            out.push(byte | 0x80);
        }
    }
}
