use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd
}

#[test]
fn smoke_help_and_version_commands() {
    chic_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("USAGE:"));

    chic_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("chic "));
}

#[test]
fn smoke_check_and_format_simple_program() {
    let tempdir = tempdir().expect("tempdir");
    let source = tempdir.path().join("smoke_simple.cl");
    fs::copy(fixture("tests/testdate/wasm_simple_add.cl"), &source).expect("copy fixture");

    chic_cmd()
        .args(["check", source.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(contains("check passed for"));

    chic_cmd()
        .args(["format", source.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(contains("formatted").or(contains("already formatted")));
}

fn fixture(rel_path: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join(rel_path)
}
