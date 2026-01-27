#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "harness/mod.rs"]
mod harness;

use chic::runtime::execute_wasm;
use fixtures::fixture;
use harness::{Category, ExecHarness, HarnessBackend};
use std::fs;
use std::process::Command;

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().is_ok()
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn perf_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_PERF").unwrap_or(false)
}

fn wasm_harness() -> ExecHarness {
    ExecHarness::new(HarnessBackend::Wasm, Category::Happy)
}

fn llvm_harness() -> ExecHarness {
    ExecHarness::new(HarnessBackend::Llvm, Category::Happy)
}

#[test]
fn uuid_executes_on_llvm() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping uuid_executes_on_llvm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping uuid_executes_on_llvm (clang not available)");
        return Ok(());
    }

    let program = fixture!("uuid/uuid_tests.ch");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn uuid_executes_on_wasm() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping uuid_executes_on_wasm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }

    let program = fixture!("uuid/uuid_tests.ch");
    let harness = wasm_harness();
    let artifact = match harness.build_executable_with_inputs(program, Some("wasm"), &[]) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let wasm_bytes = fs::read(artifact.output.path())?;
    let outcome = execute_wasm(&wasm_bytes, "chic_main")?;
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.termination.is_none());
    Ok(())
}
