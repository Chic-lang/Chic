#![allow(unsafe_code)]

use crate::runtime::wasm_executor::executor::{
    Executor, host_io::IoHooks, options::WasmExecutionOptions,
};
use crate::runtime::wasm_executor::module::{Module, WasmProgram};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn wasm_env_hooks_module() -> Vec<u8> {
    // Minimal module calling env.monotonic_nanos and env.sleep_millis then returning 0.
    // Imports:
    // 0: env.write(i32 fd, i32 ptr, i32 len)
    // 1: env.monotonic_nanos()
    // 2: env.sleep_millis(i32 ms)
    let mut bytes = vec![
        0x00, 0x61, 0x73, 0x6D, // magic
        0x01, 0x00, 0x00, 0x00, // version
    ];
    // type section
    bytes.push(1);
    let type_payload = vec![
        3, // 3 entries
        0x60, 0, 0, // type 0: () -> ()
        0x60, 1, 0x7F, 0x01, 0x7F, // type 1: (i32) -> i32
        0x60, 0, 0x01, 0x7E, // type 2: () -> i64
    ];
    push_section(&mut bytes, &type_payload);
    // import section
    bytes.push(2);
    let mut import_payload = vec![3];
    push_import(&mut import_payload, "env", "write", 0); // type 0
    push_import(&mut import_payload, "env", "monotonic_nanos", 2); // type 2
    push_import(&mut import_payload, "env", "sleep_millis", 1); // type 1
    push_section(&mut bytes, &import_payload);
    // function section (one function, type 0)
    bytes.push(3);
    let func_payload = vec![1, 0];
    push_section(&mut bytes, &func_payload);
    // export section
    bytes.push(7);
    let mut export_payload = vec![1];
    push_name(&mut export_payload, "chic_main");
    export_payload.push(0); // function
    export_payload.push(3); // index after imports
    push_section(&mut bytes, &export_payload);
    // code section
    bytes.push(10);
    let mut code_payload = Vec::new();
    code_payload.push(1); // count
    let mut body = Vec::new();
    body.push(0); // locals
    // call env.monotonic_nanos
    body.push(0x10); // call
    write_uleb(&mut body, 1); // import func index 1
    // call env.sleep_millis(0)
    body.push(0x41);
    write_sleb_i32(&mut body, 0);
    body.push(0x10);
    write_uleb(&mut body, 2); // import func index 2
    body.push(0x0B); // end
    push_func(&mut code_payload, &body);
    push_section(&mut bytes, &code_payload);
    bytes
}

fn push_section(bytes: &mut Vec<u8>, payload: &[u8]) {
    write_uleb(bytes, payload.len() as u32);
    bytes.extend_from_slice(payload);
}

fn push_name(buf: &mut Vec<u8>, name: &str) {
    write_uleb(buf, name.len() as u32);
    buf.extend_from_slice(name.as_bytes());
}

fn push_import(buf: &mut Vec<u8>, module: &str, name: &str, type_index: u32) {
    push_name(buf, module);
    push_name(buf, name);
    buf.push(0); // import kind: function
    write_uleb(buf, type_index);
}

fn push_func(buf: &mut Vec<u8>, body: &[u8]) {
    write_uleb(buf, body.len() as u32);
    buf.extend_from_slice(body);
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
        }
        out.push(byte | 0x80);
    }
}

#[test]
fn wasm_io_hooks_drive_env_imports() {
    let module = wasm_env_hooks_module();
    let program = WasmProgram::from_bytes(&module).expect("valid module");
    let clock_calls = Arc::new(AtomicUsize::new(0));
    let sleep_calls = Arc::new(AtomicUsize::new(0));
    let hooks = IoHooks {
        write: None,
        flush: None,
        read: None,
        monotonic_nanos: Some(Arc::new({
            let clock_calls = clock_calls.clone();
            move || {
                clock_calls.fetch_add(1, Ordering::SeqCst);
                123
            }
        })),
        sleep_millis: Some(Arc::new({
            let sleep_calls = sleep_calls.clone();
            move |_ms| {
                sleep_calls.fetch_add(1, Ordering::SeqCst);
                0
            }
        })),
        ..IoHooks::empty()
    };
    let mut options = WasmExecutionOptions::default();
    options.io_hooks = Some(hooks);
    let outcome = program
        .execute_export_with_options("chic_main", &[], &options)
        .expect("executes");
    assert!(
        outcome.value.is_none(),
        "expected void export to return None"
    );
    assert_eq!(clock_calls.load(Ordering::SeqCst), 1);
    assert_eq!(sleep_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn wasm_io_hooks_route_fopen() {
    let fopen_calls = Arc::new(AtomicUsize::new(0));
    let mut hooks = IoHooks::empty();
    hooks.fopen = Some(Arc::new({
        let fopen_calls = fopen_calls.clone();
        move |path, mode| {
            assert_eq!(path, "");
            assert_eq!(mode, "");
            fopen_calls.fetch_add(1, Ordering::SeqCst);
            1337
        }
    }));
    let mut options = WasmExecutionOptions::default();
    options.io_hooks = Some(hooks);
    let module = Module {
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
    };
    let mut exec = Executor::with_options(&module, &options).expect("executor");
    let handle = exec.host_fopen(0, 0).expect("fopen");
    assert_eq!(fopen_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handle, 1337);
}
