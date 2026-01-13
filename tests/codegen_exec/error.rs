use chic::codegen::CpuIsaConfig;
use chic::driver::TestStatus;
use std::error::Error;

use super::fixtures::*;
use super::harness::{Category, ExecHarness};

fn llvm_error_harness() -> ExecHarness {
    ExecHarness::llvm(Category::Error)
}

#[test]
fn llvm_test_runner_executes_testcases_when_available() -> Result<(), Box<dyn Error>> {
    let harness = llvm_error_harness();
    let run = match harness.run_tests(llvm_test_runner_program()) {
        Ok(run) => run,
        Err(err) => return err.into_test_result(&harness),
    };

    assert!(
        run.cases
            .iter()
            .any(|case| case.status == TestStatus::Passed),
        "expected at least one passing testcase"
    );
    assert!(
        run.cases
            .iter()
            .any(|case| case.status == TestStatus::Failed),
        "expected at least one failing testcase"
    );
    Ok(())
}

#[test]
fn llvm_properties_execute_correctly() -> Result<(), Box<dyn Error>> {
    let harness = llvm_error_harness().with_cpu_isa(CpuIsaConfig::baseline());
    let artifact = match harness.build_executable(wasm_properties_program(), None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    println!("llvm property binary: {}", artifact.output.path().display());
    let persisted = std::env::temp_dir().join("llvm_property_bin");
    std::fs::copy(artifact.output.path(), &persisted)?;
    println!("persisted llvm property binary at {}", persisted.display());

    let report = artifact
        .report
        .as_ref()
        .expect("build report available for diagnostics");

    let parse_diags: Vec<_> = report
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .collect();

    assert!(
        !report.has_diagnostics(),
        "unexpected diagnostics: parse={:?}, lowering={:?}, type={:?}",
        parse_diags,
        report.mir_lowering_diagnostics,
        report.type_diagnostics
    );

    let status = std::process::Command::new(&persisted).status()?;
    assert!(status.success(), "program exited with {:?}", status.code());
    Ok(())
}

#[test]
fn wasm_properties_execute_correctly() -> Result<(), Box<dyn Error>> {
    let harness = ExecHarness::wasm(Category::Error);
    let artifact = match harness.build_executable(wasm_properties_program(), Some("wasm")) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };
    println!("wasm property module: {}", artifact.output.path().display());
    let persisted = std::env::temp_dir().join("wasm_property_bin.wasm");
    std::fs::copy(artifact.output.path(), &persisted)?;
    println!("persisted wasm property binary at {}", persisted.display());

    let report = artifact
        .report
        .as_ref()
        .expect("build report available for diagnostics");

    let parse_diags: Vec<_> = report
        .modules
        .iter()
        .flat_map(|module| module.parse.diagnostics.iter())
        .collect();

    assert!(
        !report.has_diagnostics(),
        "unexpected diagnostics: parse={:?}, lowering={:?}, type={:?}",
        parse_diags,
        report.mir_lowering_diagnostics,
        report.type_diagnostics
    );

    let wasm_bytes = std::fs::read(artifact.output.path())?;
    let outcome = chic::runtime::execute_wasm(&wasm_bytes, "chic_main")?;
    assert_eq!(outcome.exit_code, 0);
    Ok(())
}
