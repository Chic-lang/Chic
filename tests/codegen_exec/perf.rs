use std::error::Error;
use std::fs;

use assert_cmd::Command;
use tempfile::{NamedTempFile, tempdir};

use super::fixtures::*;
use super::harness::{Category, ExecHarness};

fn compile_wasm_with_timeout(program: &str, output_name: &str) -> Result<(), Box<dyn Error>> {
    let harness = ExecHarness::wasm(Category::Perf);
    if let Err(err) = harness.guard() {
        return err.into_test_result(&harness);
    }

    let source = NamedTempFile::new().expect("create temp source");
    fs::write(source.path(), program).expect("write program");
    let output_dir = tempdir().expect("create temp dir");
    let output_path = output_dir.path().join(output_name);

    let mut cmd = Command::cargo_bin("chic")?;
    cmd.timeout(WASM_TIMEOUT)
        .arg("build")
        .arg(source.path())
        .args(["--backend", "wasm"])
        .arg("--output")
        .arg(&output_path);
    cmd.assert().success();

    Ok(())
}

#[test]
fn wasm_build_handles_complex_control_flow_compiles() -> Result<(), Box<dyn Error>> {
    compile_wasm_with_timeout(complex_control_flow_program(), "complex.wasm")
}

#[test]
fn wasm_build_handles_guarded_match() -> Result<(), Box<dyn Error>> {
    compile_wasm_with_timeout(guarded_match_program(), "guarded_match.wasm")
}
