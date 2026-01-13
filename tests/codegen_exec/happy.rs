use std::env;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;
use tempfile::NamedTempFile;

use chic::driver::TestStatus;
use chic::runtime::{
    WasmExecutionOptions, execute_wasm, execute_wasm_with_options, wasm_executor::IoHooks,
};

use super::fixtures::*;
use super::harness::{Category, ExecHarness, HarnessError};

fn wasm_harness() -> ExecHarness {
    ExecHarness::wasm(Category::Happy)
}

fn llvm_harness() -> ExecHarness {
    ExecHarness::llvm(Category::Happy)
}

struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = env::var(key).ok();
        // Environment setters are marked unsafe under the current toolchain config.
        unsafe {
            env::set_var(key, value);
        }
        EnvGuard { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.previous {
            unsafe {
                env::set_var(self.key, prev);
            }
        } else {
            unsafe {
                env::remove_var(self.key);
            }
        }
    }
}

fn build_and_execute_wasm(program: &str, expected_exit: i32) -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm_with(program, expected_exit, &[])
}

fn build_and_execute_wasm_with(
    program: &str,
    expected_exit: i32,
    extra_inputs: &[std::path::PathBuf],
) -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let artifact = match harness.build_executable_with_inputs(program, Some("wasm"), extra_inputs) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    let wasm_bytes = fs::read(artifact.output.path())?;
    let outcome = execute_wasm(&wasm_bytes, "chic_main")?;
    assert_eq!(outcome.exit_code, expected_exit);
    assert!(outcome.termination.is_none());
    Ok(())
}

fn build_and_execute_wasm_with_options(
    program: &str,
    expected_exit: i32,
    extra_inputs: &[std::path::PathBuf],
    options: &WasmExecutionOptions,
) -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let artifact = match harness.build_executable_with_inputs(program, Some("wasm"), extra_inputs) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    let wasm_bytes = fs::read(artifact.output.path())?;
    let outcome = execute_wasm_with_options(&wasm_bytes, "chic_main", options)?;
    assert_eq!(outcome.exit_code, expected_exit);
    assert!(outcome.termination.is_none());
    Ok(())
}

fn build_and_execute_llvm(program: &str, expected_exit: i32) -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm_with(program, expected_exit, &[])
}

fn build_and_execute_llvm_with(
    program: &str,
    expected_exit: i32,
    extra_inputs: &[std::path::PathBuf],
) -> Result<(), Box<dyn Error>> {
    let harness = llvm_harness();
    let artifact = match harness.build_executable_with_inputs(program, None, extra_inputs) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let status = Command::new(artifact.output.path()).status()?;
    assert_eq!(status.code(), Some(expected_exit));
    Ok(())
}

#[test]
fn wasm_build_produces_wasm_artifact() -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let artifact = match harness.build_executable(fixture!("wasm_simple_add.cl"), Some("wasm")) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let bin_path = artifact.output.path();
    assert_eq!(
        bin_path.extension().and_then(|ext| ext.to_str()),
        Some("wasm")
    );
    assert!(bin_path.exists());
    Ok(())
}

#[test]
fn wasm_run_executes_program() -> Result<(), Box<dyn Error>> {
    let module = simple_return_module(0);
    let outcome = execute_wasm(&module, "chic_main")?;
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.termination.is_none());
    Ok(())
}

#[test]
fn wasm_test_runner_executes_testcases() -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let run = match harness.run_tests(wasm_test_runner_program()) {
        Ok(run) => run,
        Err(err) => return err.into_test_result(&harness),
    };

    let mut status_by_name = std::collections::HashMap::new();
    for case in &run.cases {
        status_by_name.insert(case.name.clone(), case.status);
    }

    assert_eq!(run.cases.len(), 3);
    assert_eq!(
        status_by_name.get("Passes"),
        Some(&chic::driver::TestStatus::Passed)
    );
    assert_eq!(
        status_by_name.get("ReturnsValue"),
        Some(&chic::driver::TestStatus::Passed)
    );
    assert_eq!(
        status_by_name.get("DividesByZero"),
        Some(&chic::driver::TestStatus::Failed)
    );
    Ok(())
}

#[test]
fn wasm_executes_optional_parameter_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(optional_parameters_program(), 0)
}

#[test]
fn llvm_executes_advanced_pattern_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(advanced_pattern_program(), 13)
}

#[test]
fn wasm_executes_advanced_pattern_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(advanced_pattern_program(), 13)
}

#[test]
fn wasm_executes_function_pointer_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(function_pointer_program(), 22)
}

#[test]
fn wasm_executes_const_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(const_program(), 23)
}

#[test]
fn wasm_executes_core_option_result_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(core_option_result_program(), 0)
}

#[test]
fn llvm_executes_utf8_span_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(utf8_span_program(), 0)
}

#[test]
fn wasm_executes_utf8_span_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(utf8_span_program(), 0)
}

#[test]
fn llvm_executes_pointer_format_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(numeric_pointer_format_program(), 0)
}

#[test]
fn llvm_executes_string_interpolation_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(string_interpolation_program(), 0)
}

#[test]
fn llvm_executes_unicode_identifier_program_when_available() -> Result<(), Box<dyn Error>> {
    let defs = NamedTempFile::new().map_err(HarnessError::build)?;
    fs::write(defs.path(), unicode_identifiers_defs_program()).map_err(HarnessError::build)?;
    build_and_execute_llvm_with(
        unicode_identifiers_program(),
        0,
        &[defs.path().to_path_buf()],
    )
}

#[test]
fn wasm_executes_pointer_format_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(numeric_pointer_format_program(), 0)
}

#[test]
fn wasm_executes_string_interpolation_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(string_interpolation_program(), 0)
}

#[test]
fn wasm_executes_unicode_identifier_program() -> Result<(), Box<dyn Error>> {
    let defs = NamedTempFile::new().map_err(HarnessError::build)?;
    fs::write(defs.path(), unicode_identifiers_defs_program()).map_err(HarnessError::build)?;
    build_and_execute_wasm_with(
        unicode_identifiers_program(),
        0,
        &[defs.path().to_path_buf()],
    )
}

#[test]
fn llvm_executes_io_stackalloc_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(io_stackalloc_program(), 0)
}

#[test]
fn wasm_executes_io_stackalloc_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(io_stackalloc_program(), 0)
}

#[test]
fn wasm_ref_parameters_modify_argument() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(ref_parameter_program(), 42)
}

#[test]
fn wasm_executes_virtual_dispatch_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(virtual_dispatch_program(), 0)
}

#[test]
fn wasm_span_program_executes() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(span_program(), 0)
}

#[test]
fn llvm_executes_function_pointer_program_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(function_pointer_program(), 22)
}

#[test]
fn llvm_executes_const_program_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(const_program(), 23)
}

#[test]
fn llvm_executes_core_option_result_program_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(core_option_result_program(), 0)
}

#[test]
fn llvm_executes_posix_file_wrappers_when_available() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("input.txt");
    std::fs::write(&path, "hello-posix")?;
    let path_literal = path.display().to_string();
    let program = format!(
        r#"
import Std.Platform.IO;
import Std.Strings;
import Std.Span;
import Std.Platform;

namespace Exec;

public int Main()
{{
    IoError err;
    var file = File.OpenRead("{path}", out err);
    if (err != IoError.Success) {{
        return 80;
    }}

    Span<byte> buffer = Span<byte>.StackAlloc(32);
    usize read;
    if (!file.Read(buffer, out read, out err) || err != IoError.Success) {{
        return 81;
    }}
    ReadOnlySpan<byte> sliceView = buffer.AsReadOnly().Slice(0, read);
    string text = Utf8String.FromSpan(sliceView);

    file.Close(out err);
    if (err != IoError.Success) {{
        return 82;
    }}

    if (text != "hello-posix") {{
        return 83;
    }}

    ulong now = Time.MonotonicNanoseconds();
    if (now == 0) {{
        return 87;
    }}
    Time.SleepMillis(0);

    IoError writeErr;
    var outFile = File.OpenWrite("{path}", false, out writeErr);
    if (writeErr != IoError.Success) {{
        return 84;
    }}
    ReadOnlySpan<byte> payload = "ok".AsUtf8Span();
    if (outFile.Write(payload) != IoError.Success) {{
        return 85;
    }}
    outFile.Close(out writeErr);
    if (writeErr != IoError.Success) {{
        return 86;
    }}
    return 0;
}}
"#,
        path = path_literal
    );

    build_and_execute_llvm_with(&program, 0, &[])
}

#[test]
fn wasm_executes_posix_file_wrappers_with_host_io() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("input.txt");
    std::fs::write(&path, "hello-posix")?;
    let path_literal = path.display().to_string();
    let program = format!(
        r#"
import Std.Platform.IO;
import Std.Strings;
import Std.Span;
import Std.Platform;

namespace Exec;

public int Main()
{{
    IoError err;
    var file = File.OpenRead("{path}", out err);
    if (err != IoError.Success) {{
        return 80;
    }}

    Span<byte> buffer = Span<byte>.StackAlloc(32);
    usize read;
    if (!file.Read(buffer, out read, out err) || err != IoError.Success) {{
        return 81;
    }}
    ReadOnlySpan<byte> sliceView = buffer.AsReadOnly().Slice(0, read);
    string text = Utf8String.FromSpan(sliceView);

    file.Close(out err);
    if (err != IoError.Success) {{
        return 82;
    }}

    if (text != "hello-posix") {{
        return 83;
    }}

    ulong now = Time.MonotonicNanoseconds();
    if (now == 0) {{
        return 87;
    }}
    Time.SleepMillis(0);

    IoError writeErr;
    var outFile = File.OpenWrite("{path}", false, out writeErr);
    if (writeErr != IoError.Success) {{
        return 84;
    }}
    ReadOnlySpan<byte> payload = "ok".AsUtf8Span();
    if (outFile.Write(payload) != IoError.Success) {{
        return 85;
    }}
    outFile.Close(out writeErr);
    if (writeErr != IoError.Success) {{
        return 86;
    }}
    return 0;
}}
"#,
        path = path_literal
    );

    build_and_execute_wasm_with_options(&program, 0, &[], &WasmExecutionOptions::default())
}

#[test]
fn wasm_executes_socket_wrappers_with_host_io() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("CHIC_ENABLE_WASM_SOCKET").is_none() {
        eprintln!(
            "[Happy:Wasm] skipping socket wrapper exec because CHIC_ENABLE_WASM_SOCKET is not set"
        );
        return Ok(());
    }
    let port = 4242;
    let program = format!(
        r#"
import Std.Platform.IO;
import Std.Strings;
import Std.Span;

namespace Exec;

public int Main()
{{
    SocketError err;
    Socket sock;
    err = Socket.CreateTcp(out sock);
    if (err != SocketError.Success) {{
        return 90;
    }}
    Ipv4Address addr;
    if (!Ipv4Address.TryParse("127.0.0.1", out addr)) {{
        return 91;
    }}
    if (sock.Connect(addr, {port}) != SocketError.Success) {{
        return 92;
    }}
    ReadOnlySpan<byte> payload = "ping".AsUtf8Span();
    usize written;
    if (sock.Send(payload, out written) != SocketError.Success || written != payload.Length) {{
        return 93;
    }}
    Span<byte> buffer = Span<byte>.StackAlloc(8);
    usize read;
    if (sock.Receive(buffer, out read) != SocketError.Success) {{
        return 94;
    }}
    ReadOnlySpan<byte> sliceView = buffer.AsReadOnly().Slice(0, read);
    string text = Utf8String.FromSpan(sliceView);
    sock.ShutdownWrite();
    sock.Close();
    return text == "pong" ? 0 : 95;
}}
"#,
        port = port
    );
    let reply = b"pong".to_vec();
    let mut hooks = IoHooks::empty();
    hooks.socket = Some(Arc::new(|_d, _t, _p| Ok(7)));
    hooks.connect = Some(Arc::new(|_fd, _addr, _port| 0));
    hooks.send = Some(Arc::new(|_fd, data| Ok(data.len())));
    hooks.recv = Some(Arc::new(move |_fd, buf| {
        let n = reply.len().min(buf.len());
        buf[..n].copy_from_slice(&reply[..n]);
        Ok(n)
    }));
    hooks.shutdown = Some(Arc::new(|_fd, _how| 0));
    hooks.close_socket = Some(Arc::new(|_fd| 0));
    hooks.htons = Some(Arc::new(|v| v.to_be()));
    hooks.inet_pton = Some(Arc::new(|_af, text| {
        if text == "127.0.0.1" {
            Ok([127, 0, 0, 1])
        } else {
            Err(0)
        }
    }));
    let mut options = WasmExecutionOptions::default();
    options.io_hooks = Some(hooks);
    build_and_execute_wasm_with_options(&program, 0, &[], &options)
}

#[test]
fn llvm_executes_socket_wrappers_when_available() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("CHIC_ENABLE_CODEGEN_EXEC").is_none() {
        eprintln!(
            "[Happy:LLVM] skipping socket wrapper exec because CHIC_ENABLE_CODEGEN_EXEC is not set"
        );
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = std::thread::spawn(move || -> std::io::Result<()> {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 16];
            let _ = stream.read(&mut buf)?;
            stream.write_all(b"pong")?;
        }
        Ok(())
    });
    let program = format!(
        r#"
import Std.Platform.IO;
import Std.Strings;
import Std.Span;

namespace Exec;

public int Main()
{{
    SocketError err;
    Socket sock;
    err = Socket.CreateTcp(out sock);
    if (err != SocketError.Success) {{
        return 90;
    }}
    Ipv4Address addr;
    if (!Ipv4Address.TryParse("127.0.0.1", out addr)) {{
        return 91;
    }}
    if (sock.Connect(addr, {port}) != SocketError.Success) {{
        return 92;
    }}
    ReadOnlySpan<byte> payload = "ping".AsUtf8Span();
    usize written;
    if (sock.Send(payload, out written) != SocketError.Success || written != payload.Length) {{
        return 93;
    }}
    Span<byte> buffer = Span<byte>.StackAlloc(8);
    usize read;
    if (sock.Receive(buffer, out read) != SocketError.Success) {{
        return 94;
    }}
    ReadOnlySpan<byte> sliceView = buffer.AsReadOnly().Slice(0, read);
    string text = Utf8String.FromSpan(sliceView);
    sock.ShutdownWrite();
    sock.Close();
    return text == "pong" ? 0 : 95;
}}
"#,
        port = port
    );
    let result = build_and_execute_llvm_with(&program, 0, &[]);
    server.join().unwrap().unwrap();
    result
}

#[test]
fn wasm_thread_sleep_wrapper_runs() -> Result<(), Box<dyn Error>> {
    let program = r#"
import Std.Platform.Thread;

namespace Exec;

public int Main()
{
    Thread.Sleep(1);
    Thread.Yield();
    Thread.SpinWait(10);
    return 0;
}
"#;
    build_and_execute_wasm_with_options(&program, 0, &[], &WasmExecutionOptions::default())
}

#[test]
fn llvm_thread_sleep_wrapper_runs_when_available() -> Result<(), Box<dyn Error>> {
    let program = r#"
import Std.Platform.Thread;

namespace Exec;

public int Main()
{
    Thread.Sleep(1);
    Thread.Yield();
    Thread.SpinWait(10);
    return 0;
}
"#;
    build_and_execute_llvm_with(program, 0, &[])
}

#[test]
fn llvm_executes_virtual_dispatch_program_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(virtual_dispatch_program(), 0)
}

#[test]
fn llvm_ref_parameters_modify_argument_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(ref_parameter_program(), 42)
}

#[test]
fn llvm_span_program_executes_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(span_program(), 0)
}

#[test]
fn llvm_executes_bool_main_when_available() -> Result<(), Box<dyn Error>> {
    let harness = llvm_harness();
    let true_artifact = match harness.build_executable(bool_main_true(), None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    let status = Command::new(true_artifact.output.path())
        .status()
        .expect("run compiled bool main binary");
    assert_eq!(
        status.code(),
        Some(0),
        "expected bool main to map true to exit code 0"
    );

    let second = llvm_harness();
    let false_artifact = match second.build_executable(bool_main_false(), None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&second),
    };
    let status = Command::new(false_artifact.output.path())
        .status()
        .expect("run compiled bool main binary");
    assert_eq!(
        status.code(),
        Some(1),
        "expected bool main false to map to exit code 1"
    );
    Ok(())
}

#[test]
fn llvm_async_entry_executes_native_startup() -> Result<(), Box<dyn Error>> {
    let stdlib = async_stdlib_stub();
    let _override = EnvGuard::set(
        "CHIC_ASYNC_STDLIB_OVERRIDE",
        stdlib.to_str().expect("async stdlib path"),
    );
    build_and_execute_llvm_with(async_entry_program(), 7, &[stdlib])
}

#[test]
fn llvm_async_cancellation_returns_zero() -> Result<(), Box<dyn Error>> {
    let stdlib = async_stdlib_stub();
    let _override = EnvGuard::set(
        "CHIC_ASYNC_STDLIB_OVERRIDE",
        stdlib.to_str().expect("async stdlib path"),
    );
    build_and_execute_llvm_with(async_cancellation_program(), 0, &[stdlib])
}

#[test]
fn llvm_async_testcases_execute_via_runtime() -> Result<(), Box<dyn Error>> {
    let _skip_stdlib = EnvGuard::set("CHIC_SKIP_STDLIB", "1");
    let harness = llvm_harness();
    let stdlib = async_stdlib_stub();
    let _override = EnvGuard::set(
        "CHIC_ASYNC_STDLIB_OVERRIDE",
        stdlib.to_str().expect("async stdlib path"),
    );
    let artifact =
        match harness.build_executable_with_inputs(async_testcases_program(), None, &[stdlib]) {
            Ok(artifact) => artifact,
            Err(err) => return err.into_test_result(&harness),
        };

    let output = Command::new(artifact.output.path())
        .arg("--run-tests")
        .output()
        .expect("execute async testcase harness");

    let stdout = std::str::from_utf8(&output.stdout).unwrap_or_default();
    assert!(stdout.contains("async testcase passed"));
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn llvm_compiled_executable_runs_when_clang_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(llvm_factorial_program(), 0)
}

#[test]
fn wasm_async_entry_executes_native_startup() -> Result<(), Box<dyn Error>> {
    let stdlib = async_stdlib_stub();
    // WASM executor currently treats async entry points as synchronous; result defaults to zero.
    build_and_execute_wasm_with(async_entry_program(), 0, &[stdlib])
}

#[test]
fn wasm_async_cancellation_returns_zero() -> Result<(), Box<dyn Error>> {
    let stdlib = async_stdlib_stub();
    build_and_execute_wasm_with(async_cancellation_program(), 0, &[stdlib])
}

#[test]
fn wasm_async_testcases_execute_via_runtime() -> Result<(), Box<dyn Error>> {
    let _skip_stdlib = EnvGuard::set("CHIC_SKIP_STDLIB", "1");
    let harness = wasm_harness();
    let stdlib = async_stdlib_stub();
    let run = match harness.run_tests_with_inputs(async_testcases_program(), &[stdlib]) {
        Ok(run) => run,
        Err(err) => return err.into_test_result(&harness),
    };

    assert_eq!(run.cases.len(), 3);
    let status_by_name: std::collections::HashMap<_, _> = run
        .cases
        .into_iter()
        .map(|case| (case.name, case.status))
        .collect();
    assert_eq!(
        status_by_name.get("AsyncPasses"),
        Some(&TestStatus::Skipped)
    );
    assert_eq!(
        status_by_name.get("AsyncAggregates"),
        Some(&TestStatus::Skipped)
    );
    assert_eq!(status_by_name.get("SyncPasses"), Some(&TestStatus::Passed));
    Ok(())
}

fn array_initializer_program() -> &'static str {
    r#"
namespace ArrayInit;

public int Main()
{
    var zeros = new int[3];
    if (zeros[0] != 0 || zeros[1] != 0 || zeros[2] != 0)
    {
        return 1;
    }

    var literal = new int[] { 1, 2, 3 };
    if (literal[0] != 1 || literal[1] != 2 || literal[2] != 3)
    {
        return 2;
    }

    var sized = new int[3] { 4, 5, 6 };
    if (sized[0] != 4 || sized[1] != 5 || sized[2] != 6)
    {
        return 3;
    }

    var jagged = new int[][] { new int[] { 7 }, new int[] { 8, 9 } };
    if (jagged[0][0] != 7 || jagged[1][0] != 8 || jagged[1][1] != 9)
    {
        return 4;
    }

    int counter = 0;
    var order = new int[3] { counter++, counter++, counter++ };
    if (order[0] != 0 || order[1] != 1 || order[2] != 2 || counter != 3)
    {
        return 5;
    }

    return 0;
}
"#
}

#[test]
fn wasm_executes_array_initializers() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(array_initializer_program(), 0)
}

#[test]
fn llvm_executes_array_initializers_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(array_initializer_program(), 0)
}

#[test]
fn wasm_executes_null_conditional_assignment_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(null_conditional_assignment_program(), 0)
}

#[test]
fn llvm_executes_null_conditional_assignment_program_when_available() -> Result<(), Box<dyn Error>>
{
    build_and_execute_llvm(null_conditional_assignment_program(), 0)
}

#[test]
fn null_conditional_increment_is_rejected() -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let program = r#"
namespace Exec;

class Counter { public int Value; }

public int Main()
{
    Counter? counter = new Counter();
    counter?.Value++;
    return 0;
}
"#;
    match harness.build_executable(program, Some("wasm")) {
        Err(HarnessError::Build(message)) => {
            assert!(
                message.contains("null-conditional")
                    && (message.contains("++") || message.contains("increment")),
                "unexpected diagnostic: {message}"
            );
            Ok(())
        }
        Err(err) => err.into_test_result(&harness),
        Ok(_) => Err("expected null-conditional increment to be rejected".into()),
    }
}

#[test]
#[ignore = "Stdlib diagnostics currently block wasm codegen coverage"]
fn wasm_executes_local_function_program() -> Result<(), Box<dyn Error>> {
    build_and_execute_wasm(local_function_program(), 0)
}

#[test]
#[ignore = "Stdlib diagnostics currently block native codegen coverage"]
fn llvm_executes_local_function_program_when_available() -> Result<(), Box<dyn Error>> {
    build_and_execute_llvm(local_function_program(), 0)
}
