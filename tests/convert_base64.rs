#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "harness/mod.rs"]
mod harness;

use std::error::Error;
use std::fs;
use std::process::Command;

use chic::runtime::execute_wasm;
use fixtures::fixture;
use harness::{Category, ExecHarness};

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn perf_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_PERF").unwrap_or(false)
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

fn wasm_harness() -> ExecHarness {
    ExecHarness::wasm(Category::Happy)
}

fn llvm_harness() -> ExecHarness {
    ExecHarness::llvm(Category::Happy)
}

fn base64_program() -> &'static str {
    fixture!("convert/convert_base64.cl")
}

fn base64_perf_program() -> &'static str {
    fixture!("convert/convert_base64_perf.cl")
}

fn build_and_execute_wasm(program: &str, expected_exit: i32) -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let artifact = match harness.build_executable_with_inputs(program, Some("wasm"), &[]) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    let wasm_bytes = fs::read(artifact.output.path())?;
    let outcome = execute_wasm(&wasm_bytes, "chic_main")?;
    assert_eq!(outcome.exit_code, expected_exit);
    assert!(outcome.termination.is_none());
    Ok(())
}

fn build_and_execute_llvm(program: &str, expected_exit: i32) -> Result<(), Box<dyn Error>> {
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let status = Command::new(artifact.output.path()).status()?;
    assert_eq!(status.code(), Some(expected_exit));
    Ok(())
}

#[test]
fn convert_base64_executes_on_llvm() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping convert_base64_executes_on_llvm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping convert_base64_executes_on_llvm (clang not available)");
        return Ok(());
    }

    build_and_execute_llvm(base64_program(), 0)
}

#[test]
fn convert_base64_executes_on_wasm() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping convert_base64_executes_on_wasm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping convert_base64_executes_on_wasm (clang not available)");
        return Ok(());
    }

    build_and_execute_wasm(base64_program(), 0)
}

#[test]
fn convert_base64_perf_executes_on_llvm() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() || !perf_enabled() {
        eprintln!(
            "skipping convert_base64_perf_executes_on_llvm (set CHIC_ENABLE_CODEGEN_EXEC=1 and CHIC_ENABLE_CODEGEN_PERF=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping convert_base64_perf_executes_on_llvm (clang not available)");
        return Ok(());
    }

    build_and_execute_llvm(base64_perf_program(), 0)
}
