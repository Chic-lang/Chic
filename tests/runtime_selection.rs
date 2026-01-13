use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

const EXPECTED_RUNTIME_ABI: &str = "rt-abi-1";

fn chic_cmd() -> Command {
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.env("CHIC_LOG_LEVEL", "error");
    cmd
}

fn write_runtime_package(root: &Path, name: &str, kind: &str, version: &str, abi: &str) -> PathBuf {
    let runtime_root = root.join("packages").join(name);
    fs::create_dir_all(runtime_root.join("src")).expect("create runtime src dir");
    let namespace = match kind {
        "no_std" => "Std.Runtime.NoStd",
        _ => "Std.Runtime.Native",
    };
    let manifest = format!(
        "package:\n  name: {name}\n  namespace: {namespace}\n  version: {version}\n\nbuild:\n  kind: lib\n\nruntime:\n  provides:\n    kind: {kind}\n    abi: {abi}\n\ntoolchain:\n  runtime:\n    kind: {kind}\n    package: {name}\n    version: {version}\n    compat: allow_minor\n    abi: {abi}\n\nsources:\n  - path: ./src\n    namespace_prefix: {namespace}\n"
    );
    fs::write(runtime_root.join("manifest.yaml"), manifest).expect("write runtime manifest");
    fs::write(
        runtime_root.join("src").join("lib.cl"),
        "namespace Runtime; public static class Marker { }\n",
    )
    .expect("write runtime stub");
    runtime_root
}

fn write_app(root: &Path, runtime_root: &Path, kind: &str, version: &str, no_std: bool) -> PathBuf {
    let app_root = root.join("app");
    fs::create_dir_all(app_root.join("src")).expect("create app src dir");
    let manifest = format!(
        "package:\n  name: app\n  namespace: App\n  version: 0.0.1\n\nbuild:\n  kind: exe\n\ntoolchain:\n  runtime:\n    kind: {kind}\n    package: runtime.{kind}\n    version: {version}\n    compat: allow_minor\n    abi: {EXPECTED_RUNTIME_ABI}\n    path: {}\n\nsources:\n  - path: ./src\n    namespace_prefix: App\n",
        runtime_root.display()
    );
    fs::write(app_root.join("manifest.yaml"), manifest).expect("write app manifest");
    let mut source = String::new();
    if no_std {
        source.push_str("#![no_std]\n");
    }
    source.push_str(
        r#"
namespace App;

public static class Program
{
    public static int Main() { return 0; }
}
"#,
    );
    fs::write(app_root.join("src").join("main.cl"), source).expect("write app source");
    app_root.join("manifest.yaml")
}

fn collect_manifests(root: &Path) -> Vec<(PathBuf, Value)> {
    let mut manifests = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .file_name()
                .is_some_and(|name| name == "cache_manifest.json")
            {
                let contents =
                    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
                let manifest: Value = serde_json::from_str(&contents)
                    .unwrap_or_else(|err| panic!("parse manifest {path:?}: {err}"));
                manifests.push((path.clone(), manifest));
            }
        }
    }
    manifests
}

fn manifest_for_identity<'a>(
    manifests: &'a [(PathBuf, Value)],
    identity: &str,
) -> Option<&'a (PathBuf, Value)> {
    manifests.iter().find(|(_, manifest)| {
        manifest.get("runtime_identity").and_then(|v| v.as_str()) == Some(identity)
    })
}

fn object_paths(manifest: &Value) -> Vec<String> {
    manifest
        .get("artifacts")
        .and_then(|v| v.get("objects"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn build_app(
    manifest: &Path,
    artifacts_root: &Path,
    runtime_flag: Option<&str>,
) -> Vec<(PathBuf, Value)> {
    let mut cmd = chic_cmd();
    cmd.arg("build")
        .arg(manifest)
        .arg("--emit-object")
        .arg("--artifacts-path")
        .arg(artifacts_root);
    if let Some(runtime) = runtime_flag {
        cmd.arg("--runtime").arg(runtime);
    }
    cmd.current_dir(manifest.parent().unwrap())
        .assert()
        .success();
    collect_manifests(artifacts_root)
}

#[test]
fn runtime_identity_is_recorded() {
    let dir = tempdir().expect("tempdir");
    let runtime_root = write_runtime_package(
        dir.path(),
        "runtime.native",
        "native",
        "9.9.9",
        EXPECTED_RUNTIME_ABI,
    );
    let app_manifest = write_app(dir.path(), &runtime_root, "native", "9.9.9", false);
    let artifacts = dir.path().join("artifacts");

    let manifests = build_app(&app_manifest, &artifacts, None);
    let runtime_identity = "runtime.native@9.9.9";
    let manifest = manifest_for_identity(&manifests, runtime_identity)
        .unwrap_or_else(|| panic!("no manifest found for {runtime_identity}"));
    let runtime = manifest
        .1
        .get("runtime_identity")
        .and_then(|v| v.as_str())
        .expect("runtime_identity");
    assert_eq!(runtime, runtime_identity);
    assert_eq!(
        manifest
            .1
            .get("runtime_abi")
            .and_then(|v| v.as_str())
            .unwrap_or_default(),
        EXPECTED_RUNTIME_ABI
    );
    let manifest_hash = manifest
        .1
        .get("runtime_manifest_hash")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        !manifest_hash.is_empty(),
        "manifest hash should be recorded"
    );
    let object_paths = object_paths(&manifest.1);
    assert!(
        object_paths
            .iter()
            .any(|path| path.contains(runtime_identity)),
        "artifact paths should include the runtime identity"
    );
}

#[test]
fn no_std_runtime_selection_partitions_outputs() {
    let dir = tempdir().expect("tempdir");
    let runtime_root = write_runtime_package(
        dir.path(),
        "runtime.no_std",
        "no_std",
        "2.0.0",
        EXPECTED_RUNTIME_ABI,
    );
    let app_manifest = write_app(dir.path(), &runtime_root, "no_std", "2.0.0", true);
    let artifacts = dir.path().join("artifacts");

    let manifests = build_app(&app_manifest, &artifacts, Some("native-no_std"));
    let runtime_identity = "runtime.no_std@2.0.0";
    let manifest = manifest_for_identity(&manifests, runtime_identity)
        .unwrap_or_else(|| panic!("no manifest found for {runtime_identity}"));
    assert_eq!(
        manifest.1.get("runtime_identity").and_then(|v| v.as_str()),
        Some(runtime_identity)
    );
    let object_paths = object_paths(&manifest.1);
    assert!(
        object_paths
            .iter()
            .all(|path| path.contains(runtime_identity)),
        "no_std build outputs must live under the runtime-partitioned directory"
    );
}

#[test]
fn abi_mismatch_fails_fast() {
    let dir = tempdir().expect("tempdir");
    let runtime_root = write_runtime_package(
        dir.path(),
        "runtime.native",
        "native",
        "1.2.3",
        "rt-abi-mismatch",
    );
    let app_manifest = write_app(dir.path(), &runtime_root, "native", "1.2.3", false);
    let artifacts = dir.path().join("artifacts");

    let mut cmd = chic_cmd();
    cmd.arg("build")
        .arg(&app_manifest)
        .arg("--emit-object")
        .arg("--artifacts-path")
        .arg(&artifacts)
        .current_dir(app_manifest.parent().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(EXPECTED_RUNTIME_ABI))
        .stderr(predicate::str::contains("runtime.native"));
}

#[test]
fn runtime_version_change_invalidate_cache_partition() {
    let dir = tempdir().expect("tempdir");
    let runtime_root = write_runtime_package(
        dir.path(),
        "runtime.native",
        "native",
        "1.0.0",
        EXPECTED_RUNTIME_ABI,
    );
    let app_manifest = write_app(dir.path(), &runtime_root, "native", "1.0.0", false);
    let artifacts = dir.path().join("artifacts");

    let first_manifests = build_app(&app_manifest, &artifacts, None);
    let first_identity = "runtime.native@1.0.0";
    let first = manifest_for_identity(&first_manifests, first_identity)
        .unwrap_or_else(|| panic!("no manifest found for {first_identity}"));

    // Bump runtime version and rebuild with the updated selection.
    let updated_runtime_root = write_runtime_package(
        dir.path(),
        "runtime.native",
        "native",
        "2.0.0",
        EXPECTED_RUNTIME_ABI,
    );
    let updated_manifest = write_app(dir.path(), &updated_runtime_root, "native", "2.0.0", false);
    let second_manifests = build_app(&updated_manifest, &artifacts, None);
    let second_identity = "runtime.native@2.0.0";
    let second = manifest_for_identity(&second_manifests, second_identity)
        .unwrap_or_else(|| panic!("no manifest found for {second_identity}"));

    assert_ne!(
        first.0, second.0,
        "cache manifests must be partitioned by runtime identity"
    );
    let first_objects = object_paths(&first.1);
    let second_objects = object_paths(&second.1);
    assert!(
        first_objects
            .iter()
            .all(|path| !second_objects.contains(path)),
        "runtime version changes must rebuild into a fresh partition"
    );
}
