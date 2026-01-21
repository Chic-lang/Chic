use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::{clang_available, write_source};

fn read_clrlib_manifest(path: &Path) -> Value {
    use std::convert::TryInto;

    let bytes = fs::read(path).unwrap_or_else(|err| panic!("read clrlib: {err}"));
    assert!(
        bytes.len() >= 16,
        "archive {} too small to contain manifest header",
        path.display()
    );
    assert_eq!(&bytes[..8], b"CLRLIB\0\0", "unexpected clrlib magic prefix");
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    assert!(matches!(version, 1 | 2), "unsupported clrlib version");
    let manifest_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let start = 16;
    let end = start + manifest_len as usize;
    assert!(end <= bytes.len(), "manifest length exceeds archive size");
    let manifest_bytes = &bytes[start..end];
    serde_json::from_slice(manifest_bytes).expect("parse manifest json")
}

fn find_file_with_suffix(root: &Path, suffix: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
            {
                return Some(path);
            }
        }
    }
    None
}

#[test]
fn multi_file_check_succeeds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let first = dir.path().join("module_a.cl");
    let second = dir.path().join("module_b.cl");

    write_source(
        &first,
        r"
namespace Multi;

/// <summary>Entry point for the multi-module executable.</summary>
public int Main()
{
    if (Add(2, 3) == 5)
    {
        return 0;
    }
    return 1;
}
",
    );

    write_source(
        &second,
        r"
namespace Multi;

/// <summary>Adds two integers.</summary>
public int Add(int left, int right)
{
    return left + right;
}
",
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .arg("check")
        .arg(&first)
        .arg(&second)
        .assert()
        .success()
        .stdout(predicate::str::contains("check passed for"));
}

#[test]
fn multi_file_run_wasm_executes_program() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("entry.cl");
    let util_src = dir.path().join("math.cl");
    let artifact = dir.path().join("multi_run.wasm");

    write_source(
        &main_src,
        r"
namespace MultiRun;

/// <summary>Entry point that exercises a helper across modules.</summary>
public int Main()
{
    if (Double(21) == 42)
    {
        return 0;
    }
    return 1;
}
",
    );

    write_source(
        &util_src,
        r"
namespace MultiRun;

/// <summary>Doubles the supplied integer.</summary>
public int Double(int value)
{
    return value * 2;
}
",
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .arg("build")
        .arg(&main_src)
        .arg(&util_src)
        .args(["--backend", "wasm", "-o", artifact.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("frontend succeeded for"));

    assert!(
        artifact.exists(),
        "wasm artifact missing at {}",
        artifact.display()
    );
}

#[test]
fn multi_file_library_emits_per_module_artifacts() {
    if !clang_available() {
        eprintln!("skipping multi-file LLVM build test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let first = dir.path().join("iter.cl");
    let second = dir.path().join("math.cl");
    let lint_config =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/testdate/lint_allow_dead_code.yaml");

    write_source(
        &first,
        r"
namespace MultiLib;

/// <summary>Halves the supplied integer.</summary>
@allow(dead_code)
public int Half(int value)
{
    return value / 2;
}
",
    );

    write_source(
        &second,
        r"
namespace MultiLib;

/// <summary>Doubles the supplied integer.</summary>
@allow(dead_code)
public int Double(int value)
{
    return value * 2;
}
",
    );

    let artifact: PathBuf = if cfg!(windows) {
        dir.path().join("multi_static.lib")
    } else {
        dir.path().join("multi_static.a")
    };

    let status = cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_LINT_CONFIG", &lint_config)
        .arg("build")
        .arg(&first)
        .arg(&second)
        .args([
            "--backend",
            "llvm",
            "--crate-type",
            "lib",
            "-o",
            artifact.to_str().expect("utf8 path"),
            "--emit-lib",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(status.get_output().stdout.as_ref());
    assert!(
        stdout.contains("frontend succeeded for")
            || stdout.contains("build completed with diagnostics:"),
        "Unexpected stdout from library build:\n{stdout}"
    );
    assert!(
        artifact.exists(),
        "static library artifact missing at {}",
        artifact.display()
    );

    let obj_root = dir.path().join("obj");
    assert!(
        obj_root.exists(),
        "expected obj directory to be created at {}",
        obj_root.display()
    );

    let module0_suffix = format!(
        "{}.module0.o",
        first
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("module0")
    );
    let module1_suffix = format!(
        "{}.module1.o",
        second
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("module1")
    );
    let object0 = find_file_with_suffix(&obj_root, &module0_suffix)
        .unwrap_or_else(|| panic!("expected per-module object ending with {module0_suffix}"));
    let object1 = find_file_with_suffix(&obj_root, &module1_suffix)
        .unwrap_or_else(|| panic!("expected per-module object ending with {module1_suffix}"));

    assert!(
        object0.exists(),
        "expected per-module object at {}",
        object0.display()
    );
    assert!(
        object1.exists(),
        "expected per-module object at {}",
        object1.display()
    );

    let clrlib = artifact.with_extension("clrlib");
    assert!(
        clrlib.exists(),
        "clrlib archive missing at {}",
        clrlib.display()
    );

    let manifest = read_clrlib_manifest(&clrlib);
    let files = manifest["files"]
        .as_array()
        .expect("clrlib manifest files array");
    let object_entries = files
        .iter()
        .filter(|entry| entry["role"].as_str() == Some("object"))
        .count();
    assert!(
        object_entries >= 2,
        "expected multiple object members in clrlib manifest, found {object_entries}"
    );
}
