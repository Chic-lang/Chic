use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.env("CHIC_LOG_LEVEL", "error");
    cmd
}

fn write_manifest(path: &Path, namespace: &str, runtime_path: &Path) {
    let contents = format!(
        "package:\n  name: demo\n  namespace: {namespace}\n\nbuild:\n  kind: exe\n\nsources:\n  - path: ./src\n    namespace_prefix: {namespace}\n\ntoolchain:\n  runtime:\n    kind: native\n    package: runtime.native\n    version: 0.1.0\n    compat: allow_minor\n    abi: rt-abi-1\n    path: {}\n",
        runtime_path.display()
    );
    fs::write(path, contents).expect("write manifest");
}

fn write_source(path: &Path) {
    fs::write(
        path,
        r#"
namespace Demo;

public static class Program
{
    public static int Main() { return 0; }
}
"#,
    )
    .expect("write source");
}

#[test]
fn nested_manifest_under_src_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let package_root = dir.path().join("packages").join("demo");
    let src_root = package_root.join("src");
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packages")
        .join("runtime.native");
    fs::create_dir_all(&src_root).expect("create src");
    let manifest_path = src_root.join("manifest.yaml");
    write_manifest(&manifest_path, "Demo", &runtime_root);
    write_source(&src_root.join("main.cl"));

    chic_cmd()
        .arg("build")
        .arg(&manifest_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "manifest.yaml to packages/demo/manifest.yaml",
        ));
}

#[test]
fn root_manifest_builds() {
    let dir = tempdir().expect("tempdir");
    let package_root = dir.path().join("packages").join("demo");
    let src_root = package_root.join("src");
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packages")
        .join("runtime.native");
    fs::create_dir_all(&src_root).expect("create src");
    let manifest_path = package_root.join("manifest.yaml");
    write_manifest(&manifest_path, "Demo", &runtime_root);
    write_source(&src_root.join("main.cl"));

    chic_cmd()
        .arg("build")
        .arg(&manifest_path)
        .arg("--emit-object")
        .assert()
        .success();
}
