#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "harness/mod.rs"]
mod harness;

use std::error::Error;
use std::fs;
use std::path::PathBuf;
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

fn std_data_program() -> &'static str {
    fixture!("std_data_program.ch")
}

fn std_data_extra_inputs() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate");
    vec![
        root.join("std_data_fake_result_set.ch"),
        root.join("std_data_fake_command_script.ch"),
        root.join("std_data_fake_parameter.ch"),
        root.join("std_data_fake_parameter_collection.ch"),
        root.join("std_data_fake_transaction.ch"),
        root.join("std_data_fake_data_reader.ch"),
        root.join("std_data_fake_connection.ch"),
        root.join("std_data_fake_provider_factory.ch"),
        root.join("std_data_model_user_row.ch"),
        root.join("std_data_model_user_class.ch"),
        root.join("std_data_model_snake_case.ch"),
        root.join("std_data_model_parameter_args.ch"),
    ]
}

fn build_and_execute_wasm(program: &str) -> Result<(), Box<dyn Error>> {
    let harness = wasm_harness();
    let extras = std_data_extra_inputs();
    let artifact = match harness.build_executable_with_inputs(program, Some("wasm"), &extras) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    let wasm_bytes = fs::read(artifact.output.path())?;
    let outcome = execute_wasm(&wasm_bytes, "chic_main")?;
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.termination.is_none());
    Ok(())
}

fn build_and_execute_llvm(program: &str) -> Result<(), Box<dyn Error>> {
    let harness = llvm_harness();
    let extras = std_data_extra_inputs();
    let artifact = match harness.build_executable_with_inputs(program, None, &extras) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let status = Command::new(artifact.output.path()).status()?;
    assert_eq!(status.code(), Some(0));
    Ok(())
}

#[test]
fn std_data_executes_on_llvm() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping std_data_executes_on_llvm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping std_data_executes_on_llvm (clang not available)");
        return Ok(());
    }
    build_and_execute_llvm(std_data_program())
}

#[test]
fn std_data_executes_on_wasm() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping std_data_executes_on_wasm (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    build_and_execute_wasm(std_data_program())
}
