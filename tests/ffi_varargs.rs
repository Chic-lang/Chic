use assert_cmd::Command;
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

#[test]
fn chic_calls_c_variadic_function() {
    if !common::clang_available() {
        eprintln!("skipping ffi varargs test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi varargs test: static archive build not wired for windows yet");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let c_path = dir.path().join("ffi_varargs.c");
    common::write_source(&c_path, include_str!("ffi/varargs.c"));
    let _lib_path = build_static_library(dir.path(), "ffi_varargs", &c_path);

    let chic_root = dir.path().join("ffi_varargs");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/varargs_chic/manifest.yaml"),
            ),
            ("Main.cl", include_str!("ffi/varargs_chic/Main.cl")),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("ffi_varargs_artifact"));

    Command::cargo_bin("chic")
        .expect("chic binary")
        .env("CHIC_SKIP_STDLIB", "1")
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

    Command::new(&artifact).assert().success();
}
