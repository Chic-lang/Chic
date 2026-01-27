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
fn json_roundtrip_serializes_and_deserializes() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping json_roundtrip_serializes_and_deserializes (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping json_roundtrip_serializes_and_deserializes (clang not available)");
        return Ok(());
    }

    let program = fixture!("json/json_roundtrip.ch");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"Id\":7"),
        "expected serialized json, got `{stdout}`"
    );
    assert!(
        stdout.contains("7:Nova"),
        "expected roundtrip values, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn json_camelcase_applies_policy() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping json_camelcase_applies_policy (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping json_camelcase_applies_policy (clang not available)");
        return Ok(());
    }

    let program = fixture!("json/json_camelcase.ch");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"projectId\""),
        "expected camelCase property, got `{stdout}`"
    );
    assert!(
        stdout.contains("99:Delta"),
        "expected property parsing, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn json_stream_deserializes_from_memory_stream() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping json_stream_deserializes_from_memory_stream (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping json_stream_deserializes_from_memory_stream (clang not available)");
        return Ok(());
    }

    let program = fixture!("json/json_stream.ch");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("42"),
        "expected streamed value, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn json_writer_reader_sums_numbers() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping json_writer_reader_sums_numbers (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping json_writer_reader_sums_numbers (clang not available)");
        return Ok(());
    }

    let program = fixture!("json/json_writer_reader.ch");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("12"),
        "expected sum of array elements, got `{stdout}`"
    );
    Ok(())
}
