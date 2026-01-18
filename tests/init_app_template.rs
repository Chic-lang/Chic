use std::fs;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    cargo_bin_cmd!("chic")
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn clang_available() -> bool {
    StdCommand::new("clang").arg("--version").output().is_ok()
}

#[test]
fn chic_init_app_writes_files() {
    let temp = tempdir().expect("tempdir");
    chic_cmd()
        .current_dir(temp.path())
        .args(["init", "--template", "app", "MyApp"])
        .assert()
        .success();

    let project = temp.path().join("MyApp");
    for path in [
        "manifest.yaml",
        "src/App.cl",
        "tests/AppTests.cl",
        "README.md",
        ".github/workflows/ci.yml",
    ] {
        assert!(project.join(path).exists(), "expected {path} to be created");
    }

    let manifest = fs::read_to_string(project.join("manifest.yaml")).expect("manifest");
    assert!(
        manifest.contains("kind: exe"),
        "manifest should declare build.kind: exe"
    );
    assert!(
        manifest.contains("name: MyApp"),
        "manifest should include package name"
    );
}

#[test]
fn chic_init_rejects_unknown_template() {
    let temp = tempdir().expect("tempdir");
    chic_cmd()
        .current_dir(temp.path())
        .args(["init", "--template", "unknown", "Sample"])
        .assert()
        .failure()
        .stderr(contains("unknown template"));
}

#[test]
fn app_template_builds_runs_and_tests() {
    if !codegen_exec_enabled() {
        eprintln!("skipping init app template exec test because CHIC_ENABLE_CODEGEN_EXEC is not set");
        return;
    }
    if !clang_available() {
        eprintln!("skipping init app template exec test because clang is not available");
        return;
    }

    let temp = tempdir().expect("tempdir");
    chic_cmd()
        .current_dir(temp.path())
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["init", "--template", "app", "ConsoleApp"])
        .assert()
        .success();

    let project = temp.path().join("ConsoleApp");

    chic_cmd()
        .current_dir(&project)
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "0")
        .arg("build")
        .assert()
        .success();

    chic_cmd()
        .current_dir(&project)
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "0")
        .arg("test")
        .assert()
        .success();

    chic_cmd()
        .current_dir(&project)
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "0")
        .arg("run")
        .assert()
        .success()
        .stdout(contains("Hello from Chic!"));
}
