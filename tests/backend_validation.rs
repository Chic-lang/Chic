use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;

static SKIP_STDLIB_FLAG: Once = Once::new();
const ASYNC_TIMEOUT: Duration = Duration::from_secs(8);

fn write_source(contents: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(contents.as_bytes())?;
    file.flush()?;
    Ok(file)
}

fn read_clrlib_manifest(path: &std::path::Path) -> Result<Value, Box<dyn std::error::Error>> {
    use std::convert::TryInto;

    let bytes = fs::read(path)?;
    assert!(
        bytes.len() >= 16,
        "archive {} too small to contain manifest header",
        path.display()
    );
    assert_eq!(&bytes[..8], b"CLRLIB\0\0", "unexpected clrlib magic prefix");
    let version = u32::from_le_bytes(bytes[8..12].try_into()?);
    assert_eq!(version, 2, "unsupported clrlib version");
    let manifest_len = u32::from_le_bytes(bytes[12..16].try_into()?);
    let start = 16;
    let end = start + manifest_len as usize;
    assert!(end <= bytes.len(), "manifest length exceeds archive size");
    let manifest_bytes = &bytes[start..end];
    let json = serde_json::from_slice(manifest_bytes)?;
    Ok(json)
}

fn find_artifact(root: &Path, stem: &str, ext: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let matches_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s == stem)
                .unwrap_or(false);
            let matches_ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e == ext)
                .unwrap_or(false);
            if matches_stem && matches_ext {
                return Some(path);
            }
        }
    }
    None
}

fn chic_cmd() -> Result<Command, Box<dyn std::error::Error>> {
    SKIP_STDLIB_FLAG.call_once(|| {
        if env::var_os("CHIC_SKIP_STDLIB").is_none() {
            unsafe {
                env::set_var("CHIC_SKIP_STDLIB", "1");
            }
        }
    });
    let mut cmd = cargo_bin_cmd!("chic");
    if let Some(val) = env::var_os("CHIC_SKIP_STDLIB") {
        cmd.env("CHIC_SKIP_STDLIB", val);
    }
    Ok(cmd)
}

fn stdlib_enabled() -> bool {
    SKIP_STDLIB_FLAG.call_once(|| {
        if env::var_os("CHIC_SKIP_STDLIB").is_none() {
            unsafe {
                env::set_var("CHIC_SKIP_STDLIB", "1");
            }
        }
    });
    matches!(env::var("CHIC_SKIP_STDLIB"), Ok(value) if value == "0")
}

#[test]
fn cli_skip_stdlib_guard_runs_tests() -> Result<(), Box<dyn std::error::Error>> {
    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cli_skip_stdlib_guard.cl");
    let output = chic_cmd()?.arg("test").arg(&program).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[PASS] NoStdlibDiagnostics"),
        "expected stub testcase to run without diagnostics; stdout={stdout}"
    );
    assert!(
        output.status.success(),
        "expected non-zero exit on failure; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn object_initializer_program() -> &'static str {
    r"
namespace ObjectInit;

public class Counter
{
    public int Value;

    public init(int start)
    {
        self.Value = start;
    }
}

public struct Pair
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }
}

public int Main()
{
    var counter = new Counter(2) { Value = 9 };
    if (counter.Value != 9)
    {
        return 1;
    }

    var pair = new Pair(3, 4);
    if (pair.X != 3 || pair.Y != 4)
    {
        return 2;
    }

    var viaInit = new Pair(0, 0) { X = 7, Y = 2 };
    if (viaInit.X != 7 || viaInit.Y != 2)
    {
        return 3;
    }

    return 0;
}
"
}

fn stdlib_stub() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/std/src/bootstrap_stub.cl")
}

fn async_stdlib_stub() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate/stdlib_async_stub.cl")
}

fn startup_stdlib_stub() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate/stdlib_startup_stub.cl")
}

fn async_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/testdate")
        .join(name)
}

fn async_chic_cmd() -> Result<Command, Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let startup_stub = startup_stdlib_stub();
    let lint_config =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate/lint_allow_dead_code.yaml");
    let mut cmd = chic_cmd()?;
    cmd.env("CHIC_ASYNC_STDLIB_OVERRIDE", &stub);
    cmd.env("CHIC_STARTUP_STDLIB_OVERRIDE", &startup_stub);
    cmd.env("CHIC_LINT_CONFIG", &lint_config);
    cmd.env("CHIC_RUN_ENTRY", "1");
    Ok(cmd)
}

fn clang_available() -> bool {
    std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn cli_run_wasm_executes_program() -> Result<(), Box<dyn std::error::Error>> {
    if !stdlib_enabled() {
        eprintln!("skipping wasm CLI run test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace Cli;

public int Main()
{
    var total = 0;
    var index = 0;
    while (index < 5)
    {
        total += index;
        index += 1;
    }

    if (total == 10)
    {
        return 0;
    }

    return 1;
}
";

    let file = write_source(program)?;

    chic_cmd()?
        .arg("run")
        .arg(stdlib_stub())
        .arg(file.path())
        .args(["--backend", "wasm", "--log-format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"stage\":\"driver.run.complete\"",
        ));

    Ok(())
}

#[test]
fn cli_run_llvm_executes_program_when_clang_available() -> Result<(), Box<dyn std::error::Error>> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping LLVM CLI run test: clang not available");
        return Ok(());
    }

    if !stdlib_enabled() {
        eprintln!("skipping LLVM CLI run test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace Cli;

public int Main()
{
    var sum = 0;
    var index = 0;
    while (index < 5)
    {
        sum += index;
        index += 1;
    }

    if (sum == 10)
    {
        return 0;
    }

    return 1;
}
";

    let file = write_source(program)?;

    chic_cmd()?
        .arg("run")
        .arg(stdlib_stub())
        .arg(file.path())
        .args(["--backend", "llvm", "--log-format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"stage\":\"driver.run.complete\"",
        ));

    Ok(())
}

#[test]
fn cli_run_handles_object_initializers_wasm() -> Result<(), Box<dyn std::error::Error>> {
    if !stdlib_enabled() {
        eprintln!("skipping wasm object initializer test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let file = write_source(object_initializer_program())?;

    chic_cmd()?
        .arg("run")
        .arg(stdlib_stub())
        .arg(file.path())
        .args(["--backend", "wasm", "--log-format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"stage\":\"driver.run.complete\"",
        ));

    Ok(())
}

#[test]
fn cli_run_handles_object_initializers_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping LLVM object initializer test: clang not available");
        return Ok(());
    }

    if !stdlib_enabled() {
        eprintln!("skipping LLVM object initializer test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let file = write_source(object_initializer_program())?;

    let mut cmd = chic_cmd()?;
    cmd.arg("run");
    for input in stdlib_inputs() {
        cmd.arg(input);
    }
    cmd.arg(file.path())
        .args(["--backend", "llvm", "--log-format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "\"stage\":\"driver.run.complete\"",
        ));

    Ok(())
}

fn stdlib_inputs() -> [PathBuf; 2] {
    [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/std/src/bootstrap_native_main.cl"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/std/src/bootstrap_stub.cl"),
    ]
}

#[test]
fn cli_test_reports_assertion_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    if !stdlib_enabled() {
        eprintln!("skipping wasm test diagnostics scenario: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace CliTests;

testcase Passes()
{
    return true;
}

testcase DividesByZero()
{
    var value = 1;
    value /= 0;
    return true;
}
";

    let file = write_source(program)?;

    chic_cmd()?
        .arg("test")
        .arg(file.path())
        .args(["--backend", "wasm"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("discovered 2 test(s)"))
        .stdout(predicate::str::contains("[PASS] Passes"))
        .stdout(predicate::str::contains(
            "[FAIL] DividesByZero -- division by zero",
        ));

    Ok(())
}

#[test]
fn cli_test_async_runtime_reports_failures() -> Result<(), Box<dyn std::error::Error>> {
    let program = r"
namespace AsyncCli;

import Std.Async;

public async Task<int> AddAsync(int value)
{
    return value + 1;
}

async testcase AsyncWorkflow()
{
    var first = await AddAsync(2);
    var second = await AddAsync(first);
    return second == 4;
}

async testcase AsyncFailure()
{
    var value = await AddAsync(0);
    return value == -1;
}
";

    let file = write_source(program)?;

    let output = async_chic_cmd()?
        .arg("test")
        .arg(file.path())
        .args(["--backend", "llvm"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("[PASS] AsyncWorkflow (async)"),
        "async llvm tests should report pass for workflow; stdout={stdout}"
    );
    assert!(
        stdout.contains("[FAIL] AsyncFailure (async)"),
        "async llvm tests should surface the failing case; stdout={stdout}"
    );
    assert!(
        !stdout.contains("[SKIP]"),
        "async llvm tests should not skip with Task<T> lowering restored; stdout={stdout}"
    );
    assert!(
        !output.status.success(),
        "async llvm test suite should return non-zero when a case fails (status {:?}, stderr {})",
        output.status.code(),
        stderr
    );

    Ok(())
}

#[test]
fn cli_test_async_runtime_reports_failures_wasm() -> Result<(), Box<dyn std::error::Error>> {
    if !stdlib_enabled() {
        eprintln!("skipping async wasm test runner: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace AsyncCli;

import Std.Async;

public async Task<int> CountAsync(int value)
{
    return value + 2;
}

async testcase AsyncWorkflow()
{
    var first = await CountAsync(1);
    var second = await CountAsync(first);
    return second == 5;
}

async testcase AsyncFailure()
{
    return false;
}
";

    let file = write_source(program)?;
    let stub = async_stdlib_stub();

    let output = async_chic_cmd()?
        .arg("test")
        .arg(&stub)
        .arg(file.path())
        .args(["--backend", "wasm"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("[PASS] AsyncWorkflow (async)"),
        "async wasm tests should report pass for workflow; stdout={stdout}"
    );
    assert!(
        stdout.contains("[FAIL] AsyncFailure (async)"),
        "async wasm tests should surface failing case; stdout={stdout}"
    );
    assert!(
        !stdout.contains("[SKIP]"),
        "async wasm tests should not skip with Task<T> lowering restored; stdout={stdout}"
    );
    assert!(
        !output.status.success(),
        "async wasm suite should return non-zero when a case fails (status {:?}, stderr {})",
        output.status.code(),
        stderr
    );

    Ok(())
}

#[test]
fn cli_run_async_entry_executes_llvm_backend() -> Result<(), Box<dyn std::error::Error>> {
    if !clang_available() {
        eprintln!("skipping async LLVM run: clang not available");
        return Ok(());
    }
    let program = async_fixture("async_entry.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&program)
        .args(["--backend", "llvm"])
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(7),
        "expected async main to exit with 7 (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_entry_executes_wasm_backend() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_entry.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(7),
        "expected async wasm main to exit with 7 (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_failure_reports_exit_code_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if !clang_available() {
        eprintln!("skipping async LLVM failure run: clang not available");
        return Ok(());
    }
    let program = async_fixture("async_run_failure.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&program)
        .args(["--backend", "llvm"])
        .output()?;

    let code = output.status.code();
    assert_eq!(
        code,
        Some(3),
        "expected async failure main to return 3 (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_failure_reports_exit_code_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_run_failure.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .output()?;

    let code = output.status.code();
    assert_eq!(
        code,
        Some(3),
        "expected async wasm failure main to return 3 (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_cancellation_completes_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if !clang_available() {
        eprintln!("skipping async LLVM cancellation run: clang not available");
        return Ok(());
    }
    let program = async_fixture("async_runtime_cancel.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&program)
        .args(["--backend", "llvm"])
        .output()?;

    let code = output.status.code();
    assert_eq!(
        code,
        Some(0),
        "expected runtime cancellation to complete (stdout: {}, stderr: {}); got status {:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
    Ok(())
}

#[test]
fn cli_run_async_cancellation_completes_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_runtime_cancel.cl");
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected wasm runtime cancellation to complete with exit code 0 (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_timeout_is_guarded_for_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if !clang_available() {
        eprintln!("skipping async LLVM timeout run: clang not available");
        return Ok(());
    }
    let program = async_fixture("async_timeout.cl");
    let start = Instant::now();
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&program)
        .args(["--backend", "llvm"])
        .args(["--run-timeout", "200"])
        .output()?;

    let elapsed = start.elapsed();
    assert!(
        elapsed < ASYNC_TIMEOUT + Duration::from_secs(30),
        "expected timeout guard to return promptly, elapsed {:?}",
        elapsed
    );
    assert!(
        matches!(output.status.code(), Some(0) | Some(124)),
        "expected async llvm timeout suite to exit with 0 or 124; got {:?} (stdout: {}, stderr: {})",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_run_async_timeout_is_guarded_for_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_timeout.cl");
    let start = Instant::now();
    let output = async_chic_cmd()?
        .arg("run")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .args(["--run-timeout", "200"])
        .output()?;

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(20),
        "expected timeout guard to return promptly, elapsed {:?}",
        elapsed
    );
    assert_eq!(
        output.status.code(),
        Some(124),
        "expected async wasm timeout suite to exit with 124; got {:?} (stdout: {}, stderr: {})",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn cli_test_async_cancellation_passes_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if !stdlib_enabled() {
        eprintln!("skipping async llvm cancellation suite: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = async_fixture("async_test_cancellation.cl");

    let output = async_chic_cmd()?
        .arg("test")
        .arg(&program)
        .args(["--backend", "llvm"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "async llvm cancellation suite should succeed (status {:?}, stdout: {}, stderr: {})",
        output.status.code(),
        stdout,
        stderr
    );
    assert!(
        stdout.contains("[PASS] RuntimeCancelCompletes"),
        "expected RuntimeCancelCompletes to pass; stdout={stdout}"
    );
    assert!(
        stdout.contains("[PASS] TokenCancellationCompletes"),
        "expected TokenCancellationCompletes to pass; stdout={stdout}"
    );
    assert!(
        !stdout.contains("[SKIP]"),
        "cancellation tests should not skip; stdout={stdout}"
    );

    Ok(())
}

#[test]
fn cli_test_async_cancellation_passes_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_test_cancellation.cl");

    let mut cmd = async_chic_cmd()?;
    cmd.arg("test")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[PASS] RuntimeCancelCompletes"))
        .stdout(predicate::str::contains(
            "[PASS] TokenCancellationCompletes",
        ))
        .stdout(predicate::str::contains("[SKIP]").not());

    Ok(())
}

#[test]
fn cli_test_async_timeout_is_guarded_for_llvm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_test_timeout.cl");
    let start = Instant::now();
    let output = async_chic_cmd()?
        .arg("test")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "llvm"])
        .args(["--watchdog-timeout", "200"])
        .output()?;

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(20),
        "expected timeout guard to return promptly, elapsed {:?}",
        elapsed
    );
    assert!(
        !output.status.success(),
        "expected async llvm timeout suite to fail (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn cli_test_async_timeout_is_guarded_for_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let stub = async_stdlib_stub();
    let program = async_fixture("async_test_timeout.cl");
    let start = Instant::now();
    let output = async_chic_cmd()?
        .arg("test")
        .arg(&stub)
        .arg(&program)
        .args(["--backend", "wasm"])
        .args(["--watchdog-timeout", "200"])
        .output()?;

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(20),
        "expected timeout guard to return promptly, elapsed {:?}",
        elapsed
    );
    assert!(
        !output.status.success(),
        "expected async wasm timeout suite to fail (stdout: {}, stderr: {})",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("watchdog timeout"),
        "expected watchdog timeout message in stdout; stdout={stdout}"
    );

    Ok(())
}

#[test]
fn cli_build_emits_native_library_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping native library build test: clang not available");
        return Ok(());
    }

    let program = r"
namespace Interop;

/// <summary>Add two integers for interop validation.</summary>
public int Add(int left, int right)
{
    return left + right;
}
";

    let build_dir = tempfile::tempdir()?;
    let source_path = build_dir.path().join("interop.cl");
    fs::write(&source_path, program)?;

    chic_cmd()?
        .arg("build")
        .arg(&source_path)
        .args(["--backend", "llvm", "--crate-type", "lib", "--emit-lib"])
        .assert()
        .success();

    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("source file has a valid stem");
    let archive_path = find_artifact(build_dir.path(), stem, "a").unwrap_or_else(|| {
        panic!(
            "static archive was not produced under {}",
            build_dir.path().display()
        )
    });
    assert!(
        archive_path.exists(),
        "static archive was not produced at {}",
        archive_path.display()
    );
    let clrlib_path = find_artifact(build_dir.path(), stem, "clrlib").unwrap_or_else(|| {
        panic!(
            "clrlib archive was not produced under {}",
            build_dir.path().display()
        )
    });

    let manifest = read_clrlib_manifest(&clrlib_path)?;
    assert_eq!(
        manifest["kind"].as_str(),
        Some("static-library"),
        "unexpected archive kind metadata"
    );
    let exports = manifest["exports"]
        .as_array()
        .expect("exports array present");
    assert!(
        exports
            .iter()
            .any(|entry| entry["symbol"].as_str() == Some("Interop::Add")),
        "expected Interop::Add export in manifest"
    );
    let files = manifest["files"].as_array().expect("files array present");
    assert!(
        files
            .iter()
            .any(|entry| entry["role"].as_str() == Some("object")),
        "expected at least one object file entry"
    );
    Ok(())
}

#[test]
fn static_library_links_into_c_host() -> Result<(), Box<dyn std::error::Error>> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping C linkage test: clang not available");
        return Ok(());
    }

    if !stdlib_enabled() {
        eprintln!("skipping C linkage test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace Interop;

/// <summary>Add two integers for interop validation.</summary>
public int Add(int left, int right)
{
    return left + right;
}
";

    let build_dir = tempfile::tempdir()?;
    let source_path = build_dir.path().join("interop.cl");
    fs::write(&source_path, program)?;

    chic_cmd()?
        .arg("build")
        .arg(&source_path)
        .args(["--backend", "llvm", "--crate-type", "lib"])
        .assert()
        .success();

    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("source file has a valid stem");
    let archive_path = find_artifact(build_dir.path(), stem, "a").unwrap_or_else(|| {
        panic!(
            "static archive missing under {}",
            build_dir.path().display()
        )
    });

    let temp_dir = tempfile::tempdir()?;
    let c_path = temp_dir.path().join("main.c");
    fs::write(
        &c_path,
        "#include <stdint.h>\nextern int32_t Interop__Add(int32_t left, int32_t right);\nint main(void) { return Interop__Add(3, 4) == 7 ? 0 : 1; }\n",
    )?;

    let exe_path = temp_dir.path().join("interop_main");
    let status = std::process::Command::new("clang")
        .arg("-std=c11")
        .arg(&c_path)
        .arg(&archive_path)
        .arg("-o")
        .arg(&exe_path)
        .status()?;
    assert!(status.success(), "clang failed to link static library");

    let run_status = std::process::Command::new(&exe_path).status()?;
    assert!(
        run_status.success(),
        "linked binary returned failure status"
    );

    Ok(())
}

#[test]
fn cli_header_generates_compilable_header() -> Result<(), Box<dyn std::error::Error>> {
    if std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping header compilation test: clang not available");
        return Ok(());
    }

    if !stdlib_enabled() {
        eprintln!("skipping header compilation test: CHIC_SKIP_STDLIB set");
        return Ok(());
    }

    let program = r"
namespace Interop;

public int Add(int left, int right)
{
    return left + right;
}

public int Accumulate(ref int accumulator, int value)
{
    accumulator += value;
    return accumulator;
}
";

    let source = write_source(program)?;
    let temp_dir = tempfile::tempdir()?;
    let header_path = temp_dir.path().join("interop.h");

    chic_cmd()?
        .arg("header")
        .arg(source.path())
        .arg("-o")
        .arg(&header_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote header"));

    let contents = fs::read_to_string(&header_path)?;
    assert!(contents.contains("int32_t Interop__Add"));
    assert!(contents.contains("int32_t Interop__Accumulate(int32_t *accumulator"));

    let c_path = temp_dir.path().join("interop.c");
    fs::write(
        &c_path,
        "#include \"interop.h\"\nint main(void) { int acc = 1; acc = Interop__Accumulate(&acc, 2); return Interop__Add(acc, 3) != 6; }\n",
    )?;

    let status = std::process::Command::new("clang")
        .arg("-std=c11")
        .arg("-c")
        .arg(&c_path)
        .arg("-o")
        .arg(temp_dir.path().join("interop.o"))
        .arg("-I")
        .arg(temp_dir.path())
        .status()?;
    assert!(status.success(), "clang failed to compile generated header");

    Ok(())
}

#[test]
fn cli_header_fails_when_diagnostics_present() -> Result<(), Box<dyn std::error::Error>> {
    let program = r"
namespace Broken;

public int Add(int value)
{
    return value + ;
}
";

    let source = write_source(program)?;

    chic_cmd()?
        .arg("header")
        .arg(source.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("encountered errors while parsing"));

    Ok(())
}
