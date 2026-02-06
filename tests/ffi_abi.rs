use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod common;

fn host_target() -> String {
    target_lexicon::HOST.to_string()
}

fn platform_executable_name(base: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn repo_native_runtime_archive() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("runtime")
        .join("native")
        .join("runtime.native@0.1.0")
        .join("libchic_rt_native.a")
}

fn build_static_library(dir: &Path, stem: &str, c_source: &Path) -> PathBuf {
    let object_path = dir.join(format!("{stem}.o"));
    Command::new("clang")
        .args(["-c", c_source.to_str().expect("utf8 source")])
        .args(["-o", object_path.to_str().expect("utf8 obj")])
        .assert()
        .success();

    let lib_path = dir.join(format!("lib{stem}.a"));
    Command::new("ar")
        .args(["rcs", lib_path.to_str().expect("utf8 lib")])
        .arg(&object_path)
        .assert()
        .success();
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("ranlib").arg(&lib_path).status();
    }
    lib_path
}

fn collect_object_outputs(output_dir: &Path) -> Vec<PathBuf> {
    let mut outputs = Vec::new();
    let entries = std::fs::read_dir(output_dir).expect("read artifact directory");
    for entry in entries {
        let entry = entry.expect("dir entry");
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".o") {
            outputs.push(entry.path());
        }
    }
    outputs.sort();
    outputs
}

fn assert_exec_success(path: &Path) {
    let output = Command::new(path)
        .output()
        .unwrap_or_else(|err| panic!("failed to execute {}: {err}", path.display()));
    if output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = output.status.signal() {
            panic!(
                "executable {} terminated by signal {signal}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                path.display()
            );
        }
    }

    panic!(
        "executable {} exited with status {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        path.display(),
        output.status
    );
}

#[test]
fn chic_calls_c_aggregate_returns_and_byval_params() {
    if !common::clang_available() {
        eprintln!("skipping ffi abi test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi abi test: static archive build not wired for windows yet");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let c_path = dir.path().join("ffi_aggs.c");
    common::write_source(&c_path, include_str!("ffi/aggs.c"));
    let _lib_path = build_static_library(dir.path(), "ffi_aggs", &c_path);

    let chic_root = dir.path().join("chic_calls_c");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/chic_calls_c/manifest.yaml"),
            ),
            ("S1.ch", include_str!("ffi/chic_calls_c/S1.ch")),
            ("S2.ch", include_str!("ffi/chic_calls_c/S2.ch")),
            ("S3.ch", include_str!("ffi/chic_calls_c/S3.ch")),
            ("S4.ch", include_str!("ffi/chic_calls_c/S4.ch")),
            ("S8.ch", include_str!("ffi/chic_calls_c/S8.ch")),
            ("S16.ch", include_str!("ffi/chic_calls_c/S16.ch")),
            ("S24.ch", include_str!("ffi/chic_calls_c/S24.ch")),
            ("S32.ch", include_str!("ffi/chic_calls_c/S32.ch")),
            ("S48.ch", include_str!("ffi/chic_calls_c/S48.ch")),
            ("S64.ch", include_str!("ffi/chic_calls_c/S64.ch")),
            ("S72.ch", include_str!("ffi/chic_calls_c/S72.ch")),
            ("Hfa4d.ch", include_str!("ffi/chic_calls_c/Hfa4d.ch")),
            ("Mix.ch", include_str!("ffi/chic_calls_c/Mix.ch")),
            ("Outer.ch", include_str!("ffi/chic_calls_c/Outer.ch")),
            ("Native.ch", include_str!("ffi/chic_calls_c/Native.ch")),
            ("Main.ch", include_str!("ffi/chic_calls_c/Main.ch")),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("chic_calls_c_ffi"));

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        // Keep integration tests deterministic: CI defaults enable strict formatter enforcement,
        // which can cause tests that generate/inline Chic sources to fail before exercising the
        // behavior under test.
        .env("CHIC_CI", "0")
        .arg("build")
        .arg(&chic_manifest)
        .args([
            "--backend",
            "llvm",
            "--target",
            host_target().as_str(),
            "-o",
            artifact.to_str().expect("utf8 artifact path"),
            "--ffi-search",
            dir.path().to_str().expect("utf8 search path"),
        ])
        .assert()
        .success();

    assert_exec_success(&artifact);
}

#[test]
fn c_calls_chic_aggregate_returns_and_byval_params() {
    if !common::clang_available() {
        eprintln!("skipping ffi abi test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi abi test: static archive build not wired for windows yet");
        return;
    }

    let runtime_archive = repo_native_runtime_archive();
    assert!(
        runtime_archive.exists(),
        "native runtime archive missing at {}",
        runtime_archive.display()
    );

    let dir = tempdir().expect("temp dir");
    let chic_root = dir.path().join("c_calls_chic");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/c_calls_chic/manifest.yaml"),
            ),
            ("S48.ch", include_str!("ffi/c_calls_chic/S48.ch")),
            ("S64.ch", include_str!("ffi/c_calls_chic/S64.ch")),
            ("Hfa4d.ch", include_str!("ffi/c_calls_chic/Hfa4d.ch")),
            ("Mix.ch", include_str!("ffi/c_calls_chic/Mix.ch")),
            ("Exports.ch", include_str!("ffi/c_calls_chic/Exports.ch")),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");

    let artifacts_path = dir.path().join("artifacts");
    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        // Keep integration tests deterministic: CI defaults enable strict formatter enforcement,
        // which can cause tests that generate/inline Chic sources to fail before exercising the
        // behavior under test.
        .env("CHIC_CI", "0")
        .arg("build")
        .arg(&chic_manifest)
        .args([
            "--backend",
            "llvm",
            "--target",
            host_target().as_str(),
            "--crate-type",
            "lib",
            "--emit=obj",
            "--artifacts-path",
            artifacts_path.to_str().expect("utf8 artifacts path"),
        ])
        .assert()
        .success();

    let obj_dir = artifacts_path
        .join("obj")
        .join(host_target())
        .join("Debug")
        .join("llvm")
        .join("runtime.native@0.1.0");
    let chic_objects = collect_object_outputs(&obj_dir);
    assert!(
        !chic_objects.is_empty(),
        "chic build did not produce any objects under {}",
        obj_dir.display()
    );

    let c_main = dir.path().join("main.c");
    common::write_source(&c_main, include_str!("ffi/c_calls_chic_main.c"));
    let artifact = dir
        .path()
        .join(platform_executable_name("c_calls_chic_ffi"));

    let mut link = Command::new("clang");
    link.arg("-target").arg(host_target());
    link.arg(&c_main);
    for obj in &chic_objects {
        link.arg(obj);
    }
    link.arg(&runtime_archive);
    if cfg!(target_os = "linux") {
        link.args(["-lm", "-ldl", "-lpthread"]);
    }
    link.arg("-o").arg(&artifact);
    link.assert().success();

    assert_exec_success(&artifact);
}
