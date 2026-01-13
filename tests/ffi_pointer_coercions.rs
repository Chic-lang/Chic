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
fn pointer_coercions_and_null_round_trip() {
    if !common::clang_available() {
        eprintln!("skipping ffi pointer test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi pointer test: static archive build not wired for windows yet");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let c_path = dir.path().join("ffi_pointers.c");
    common::write_source(&c_path, include_str!("ffi/pointers.c"));
    let _lib_path = build_static_library(dir.path(), "ffi_pointers", &c_path);

    let chic_root = dir.path().join("ffi_pointer_coercions_chic");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/pointer_coercions_chic/manifest.yaml"),
            ),
            (
                "Main.cl",
                include_str!("ffi/pointer_coercions_chic/Main.cl"),
            ),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("ffi_pointer_coercions"));

    cargo_bin_cmd!("chic")
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

#[test]
fn implicit_void_to_typed_pointer_is_rejected() {
    let dir = tempdir().expect("temp dir");

    let chic_root = dir.path().join("ffi_pointer_invalid");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                "package:\n  name: ffi-pointer-invalid\n  namespace: Tests.FfiPointersInvalid\n\nbuild:\n  kind: exe\n\nsources:\n  - path: .\n    namespace_prefix: Tests.FfiPointersInvalid\nwerror: true\n",
            ),
            (
                "Main.cl",
                r#"
namespace Tests.FfiPointersInvalid;

public struct Value { public long marker; }

public static class Native
{
    @extern("C")
    public static extern void* get_void_pointer();
}

public static int Main()
{
    unsafe
    {
        let *mut Value typed = Native.get_void_pointer();
    }
    return 0;
}
"#,
            ),
        ],
    );

    let chic_manifest = chic_root.join("manifest.yaml");
    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&chic_manifest)
        .args(["--backend", "llvm", "--target", host_target().as_str()])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "cannot implicitly convert `void*` to",
        ));
}
