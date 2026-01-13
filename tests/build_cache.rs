use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use blake3::Hash;
use chic::target::Target;
use filetime::{FileTime, set_file_mtime};
use serde_json::Value;
use tempfile::tempdir;

fn chic_cmd() -> Command {
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.env("CHIC_LOG_LEVEL", "error");
    cmd
}

fn host_target() -> String {
    Target::host().triple().to_string()
}

fn manifest_path(root: &Path, profile: &str, backend: &str) -> PathBuf {
    root.join("obj")
        .join(host_target())
        .join(profile)
        .join(backend)
        .join("cache")
        .join("cache_manifest.json")
}

fn read_manifest(path: &Path) -> Value {
    let contents =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read manifest {path:?}: {err}"));
    serde_json::from_str(&contents).expect("parse manifest json")
}

fn find_manifest(root: &Path, backend: &str, profile: &str) -> (PathBuf, Value) {
    let mut stack = vec![root.join("obj")];
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
                let manifest = read_manifest(&path);
                let backend_match = manifest
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let profile_match = manifest
                    .get("profile")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if backend_match == backend && profile_match == profile {
                    return (path, manifest);
                }
            }
        }
    }
    panic!("no manifest found for backend {backend} profile {profile}");
}

fn file_hashes(manifest: &Value) -> HashMap<String, String> {
    let mut hashes = HashMap::new();
    let files = manifest
        .get("files")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("manifest missing files array: {manifest}"));
    for file in files {
        let path = file
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("manifest file missing path: {file}"));
        let hash = file
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("manifest file missing hash: {file}"));
        hashes.insert(path.to_string(), hash.to_string());
    }
    hashes
}

fn object_paths(manifest: &Value) -> Vec<PathBuf> {
    manifest
        .get("artifacts")
        .and_then(|v| v.get("objects"))
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("manifest missing artifacts.objects: {manifest}"))
        .iter()
        .map(|v| {
            let raw = v
                .as_str()
                .unwrap_or_else(|| panic!("object path not a string: {v}"));
            PathBuf::from(raw)
        })
        .collect()
}

fn hash_file(path: &Path) -> Hash {
    let bytes = fs::read(path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
    blake3::hash(&bytes)
}

#[test]
fn content_change_with_same_mtime_updates_manifest_hash() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("main.cl");
    fs::write(
        &source,
        r#"
namespace Cache;

public int Main() { return Value(); }
public int Value() { return 1; }
"#,
    )
    .expect("write source");

    chic_cmd()
        .arg("build")
        .arg(&source)
        .args(["--backend", "llvm", "--emit-object"])
        .current_dir(dir.path())
        .assert()
        .success();

    let manifest_path = manifest_path(dir.path(), "Debug", "llvm");
    let manifest = read_manifest(&manifest_path);
    let mut hashes = file_hashes(&manifest);
    let first_hash = hashes
        .remove("main.cl")
        .expect("manifest should contain main.cl");
    let objects = object_paths(&manifest);
    let first_object = objects.first().expect("object path recorded");
    let first_object_hash = hash_file(&dir.path().join(first_object));

    let metadata = fs::metadata(&source).expect("metadata");
    let original_mtime = FileTime::from_last_modification_time(&metadata);
    fs::write(
        &source,
        r#"
namespace Cache;

public int Main() { return Value(); }
public int Value() { return 2; }
"#,
    )
    .expect("rewrite source");
    set_file_mtime(&source, original_mtime).expect("set mtime");

    chic_cmd()
        .arg("build")
        .arg(&source)
        .args(["--backend", "llvm", "--emit-object"])
        .current_dir(dir.path())
        .assert()
        .success();

    let updated_manifest = read_manifest(&manifest_path);
    let updated_hashes = file_hashes(&updated_manifest);
    let second_hash = updated_hashes
        .get("main.cl")
        .unwrap_or_else(|| panic!("manifest missing updated main.cl entry: {updated_manifest}"));
    assert_ne!(
        first_hash, *second_hash,
        "content change with unchanged mtime should update recorded hash"
    );
    let updated_objects = object_paths(&updated_manifest);
    let updated_object = updated_objects
        .first()
        .expect("updated object path recorded");
    let updated_object_hash = hash_file(&dir.path().join(updated_object));
    assert_ne!(
        first_object_hash, updated_object_hash,
        "object should be rebuilt when source content changes even if mtime is unchanged"
    );
}

#[test]
fn deleting_input_invalidates_cache_and_fails_build() {
    let dir = tempdir().expect("tempdir");
    let main = dir.path().join("main.cl");
    let helper = dir.path().join("helper.cl");
    fs::write(
        &main,
        r#"
namespace Cache;

public static class Wrapper
{
    public static int AddWrapped(int a, int b) { return Helper.Add(a, b); }
}
"#,
    )
    .expect("write main");
    fs::write(
        &helper,
        r#"
namespace Cache.Helper;

public static class Helper
{
    public static int Add(int a, int b) { return a + b; }
}
"#,
    )
    .expect("write helper");

    chic_cmd()
        .arg("build")
        .arg(&main)
        .arg(&helper)
        .args(["--backend", "llvm", "--emit-object", "--crate-type", "lib"])
        .current_dir(dir.path())
        .assert()
        .success();

    let manifest_path = manifest_path(dir.path(), "Debug", "llvm");
    let manifest = read_manifest(&manifest_path);
    let hashes = file_hashes(&manifest);
    assert!(
        hashes.contains_key("helper.cl"),
        "manifest should record helper.cl before deletion"
    );

    fs::remove_file(&helper).expect("delete helper");

    let deletion_output = chic_cmd()
        .arg("build")
        .arg(&main)
        .args(["--backend", "llvm", "--emit-object", "--crate-type", "lib"])
        .current_dir(dir.path())
        .output()
        .expect("run chic build after deletion");
    assert!(
        !deletion_output.status.success(),
        "deleting a referenced source should fail the build"
    );
}

#[test]
fn profile_and_backend_outputs_are_partitioned() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("main.cl");
    fs::write(
        &source,
        r#"
namespace Cache;

public int Main() { return 0; }
"#,
    )
    .expect("write source");

    chic_cmd()
        .arg("build")
        .arg(&source)
        .args(["--backend", "llvm", "--emit-object"])
        .current_dir(dir.path())
        .assert()
        .success();

    chic_cmd()
        .arg("build")
        .arg(&source)
        .args([
            "--backend",
            "llvm",
            "--emit-object",
            "--configuration",
            "Release",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    chic_cmd()
        .arg("build")
        .arg(&source)
        .args(["--backend", "wasm", "--emit-object"])
        .current_dir(dir.path())
        .assert()
        .success();

    let (_debug_manifest_path, debug_manifest) = find_manifest(dir.path(), "llvm", "Debug");
    let (_release_manifest_path, release_manifest) = find_manifest(dir.path(), "llvm", "Release");
    let (_wasm_manifest_path, wasm_manifest_json) = find_manifest(dir.path(), "wasm", "Debug");

    let debug_objects = object_paths(&debug_manifest);
    let release_objects = object_paths(&release_manifest);
    let wasm_objects = object_paths(&wasm_manifest_json);

    for path in debug_objects.iter() {
        assert!(
            path.to_string_lossy().contains("/Debug/llvm/")
                || path.to_string_lossy().contains("\\Debug\\llvm\\"),
            "debug llvm object should live under debug/llvm partition: {path:?}"
        );
    }
    for path in release_objects.iter() {
        assert!(
            path.to_string_lossy().contains("/Release/llvm/")
                || path.to_string_lossy().contains("\\Release\\llvm\\"),
            "release llvm object should live under release/llvm partition: {path:?}"
        );
    }
    for path in wasm_objects.iter() {
        assert!(
            path.to_string_lossy().contains("/Debug/wasm/")
                || path.to_string_lossy().contains("\\Debug\\wasm\\"),
            "wasm object should live under wasm partition: {path:?}"
        );
    }
}
