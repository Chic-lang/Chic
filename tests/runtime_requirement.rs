use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.env("CHIC_LOG_LEVEL", "error");
    cmd
}

fn write_manifest(
    root: &Path,
    kind: &str,
    require_runtime: Option<bool>,
    include_runtime: bool,
    runtime_path: Option<&Path>,
) -> std::path::PathBuf {
    let package_root = root.join("packages").join("demo");
    let src_root = package_root.join("src");
    fs::create_dir_all(&src_root).expect("create src");
    let mut manifest = format!(
        "package:\n  name: demo\n  namespace: Demo\n  version: 0.0.1\n\nbuild:\n  kind: {kind}\n\nsources:\n  - path: ./src\n    namespace_prefix: Demo\n"
    );
    if include_runtime {
        let runtime_root = runtime_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new("packages/runtime.native").to_path_buf());
        manifest.push_str("\ntoolchain:\n  runtime:\n");
        manifest.push_str("    kind: native\n");
        manifest.push_str("    package: runtime.native\n");
        manifest.push_str("    abi: rt-abi-1\n");
        manifest.push_str(&format!("    path: {}\n", runtime_root.display()));
        if let Some(required) = require_runtime {
            manifest.push_str("    policy:\n");
            manifest.push_str(&format!(
                "      require_native_runtime: {}\n",
                if required { "true" } else { "false" }
            ));
        }
    }
    fs::write(package_root.join("manifest.yaml"), manifest).expect("write manifest");
    fs::write(
        src_root.join("main.cl"),
        r#"
#![no_std]
namespace Demo;

public static class Program
{
    public static int Main() { return 0; }
}
"#,
    )
    .expect("write source");
    package_root.join("manifest.yaml")
}

#[test]
fn executable_defaults_require_runtime_and_fail_without_selection() {
    let dir = tempdir().expect("tempdir");
    let manifest = write_manifest(dir.path(), "exe", None, false, None);

    chic_cmd()
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&manifest)
        .arg("--emit-object")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "toolchain.runtime must be specified in manifest.yaml",
        ));
}

#[test]
fn executable_can_opt_out_of_runtime_requirement() {
    let dir = tempdir().expect("tempdir");
    let bogus_runtime = dir.path().join("packages").join("missing-runtime");
    let manifest = write_manifest(dir.path(), "exe", Some(false), true, Some(&bogus_runtime));

    chic_cmd()
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&manifest)
        .arg("--emit-object")
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn libraries_default_to_no_runtime_requirement() {
    let dir = tempdir().expect("tempdir");
    let bogus_runtime = dir.path().join("packages").join("missing-runtime");
    let manifest = write_manifest(dir.path(), "lib", None, true, Some(&bogus_runtime));

    chic_cmd()
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&manifest)
        .arg("--emit-object")
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn runtime_resolution_failure_is_fatal_when_required() {
    let dir = tempdir().expect("tempdir");
    let bogus_runtime = dir.path().join("packages").join("missing-runtime");
    let manifest = write_manifest(dir.path(), "exe", Some(true), true, Some(&bogus_runtime));

    chic_cmd()
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&manifest)
        .arg("--emit-object")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("runtime manifest"));
}
