use std::fs;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    cargo_bin_cmd!("chic")
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
    let temp = tempdir().expect("tempdir");
    chic_cmd()
        .current_dir(temp.path())
        .args(["init", "--template", "app", "ConsoleApp"])
        .assert()
        .success();

    let project = temp.path().join("ConsoleApp");

    chic_cmd()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    chic_cmd()
        .current_dir(&project)
        .arg("test")
        .assert()
        .success();

    chic_cmd()
        .current_dir(&project)
        .arg("run")
        .assert()
        .success()
        .stdout(contains("Hello from Chic!"));
}
