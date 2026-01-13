#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "harness/mod.rs"]
mod harness;

use std::process::Command;

use fixtures::fixture;
use harness::{Category, ExecHarness, HarnessBackend};

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().is_ok()
}

fn perf_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_PERF").unwrap_or(false)
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn llvm_harness() -> ExecHarness {
    ExecHarness::new(HarnessBackend::Llvm, Category::Happy)
}

#[test]
fn memory_stream_basic_io() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping memory_stream_basic_io (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping memory_stream_basic_io (clang not available)");
        return Ok(());
    }

    let program = fixture!("io/memory_stream_basic.cl");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("4"),
        "expected length marker, got `{stdout}`"
    );
    assert!(
        stdout.contains("1,9,7,8"),
        "expected payload bytes, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn memory_stream_copy_to() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping memory_stream_copy_to (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping memory_stream_copy_to (clang not available)");
        return Ok(());
    }

    let program = fixture!("io/memory_stream_copyto.cl");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("copy-ok"),
        "expected copy-ok, got `{stdout}`"
    );
    Ok(())
}
