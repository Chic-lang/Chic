use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod common;

fn dynamic_ffi_enabled() -> bool {
    matches!(
        std::env::var("CHIC_ENABLE_DYNAMIC_FFI"),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}

#[test]
fn dynamic_ffi_executes_native_function() {
    if !dynamic_ffi_enabled() {
        eprintln!("skipping dynamic ffi test: CHIC_ENABLE_DYNAMIC_FFI is not set");
        return;
    }
    if !common::clang_available() {
        eprintln!("skipping dynamic ffi test: clang not available");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let lib_path = compile_shared_library(dir.path(), "ffi_math");
    let source_path = dir.path().join("ffi_test.cl");
    fs::write(&source_path, chic_program()).expect("write chic program");
    let artifact = dir.path().join(platform_executable_name("ffi_app"));

    Command::cargo_bin("chic")
        .expect("chic binary")
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&source_path)
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

    let run = Command::new(&artifact).output().expect("execute ffi app");
    assert!(
        run.status.success(),
        "program should succeed: status={:?}, stderr={}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("FFI-W0001"),
        "expected optional binding warning, stderr:\n{stderr}"
    );

    assert!(lib_path.exists(), "shared library should remain in place");
}

#[test]
fn ffi_package_copies_shared_library() {
    if !dynamic_ffi_enabled() {
        eprintln!("skipping ffi package test: CHIC_ENABLE_DYNAMIC_FFI is not set");
        return;
    }
    if !common::clang_available() {
        eprintln!("skipping ffi package test: clang not available");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let lib_path = compile_shared_library(dir.path(), "ffi_math");
    let source_path = dir.path().join("ffi_pack.cl");
    fs::write(&source_path, chic_program()).expect("write chic program");
    let artifact = dir.path().join(platform_executable_name("ffi_pack_app"));

    Command::cargo_bin("chic")
        .expect("chic binary")
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&source_path)
        .args([
            "--backend",
            "llvm",
            "--target",
            host_target().as_str(),
            "-o",
            artifact.to_str().expect("utf8 artifact path"),
            "--ffi-package",
            lib_path.to_str().expect("utf8 lib path"),
        ])
        .assert()
        .success();

    let deps_dir = artifact.with_extension("deps");
    assert!(
        deps_dir.exists(),
        "package directory {} should exist",
        deps_dir.display()
    );
    let packaged = deps_dir.join(lib_path.file_name().expect("library filename"));
    assert!(
        packaged.exists(),
        "expected shared library copy at {}",
        packaged.display()
    );

    // Remove the original so resolution depends on packaged assets.
    fs::remove_file(&lib_path).expect("remove source library after packaging");

    let run = Command::new(&artifact)
        .output()
        .expect("execute packaged ffi app");
    assert!(
        run.status.success(),
        "packaged program should succeed: status={:?}, stderr={}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("FFI-W0001"),
        "expected optional binding warning, stderr:\n{stderr}"
    );
}

#[test]
fn ffi_required_binding_without_library_errors() {
    if !dynamic_ffi_enabled() {
        eprintln!("skipping ffi required test: CHIC_ENABLE_DYNAMIC_FFI is not set");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let source_path = dir.path().join("ffi_missing_required.cl");
    fs::write(&source_path, chic_missing_required_program()).expect("write chic program");
    let artifact = dir
        .path()
        .join(platform_executable_name("ffi_missing_required_app"));

    Command::cargo_bin("chic")
        .expect("chic binary")
        .env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(&source_path)
        .args([
            "--backend",
            "llvm",
            "--target",
            host_target().as_str(),
            "-o",
            artifact.to_str().expect("utf8 artifact path"),
        ])
        .assert()
        .success();

    let run = Command::new(&artifact)
        .output()
        .expect("execute missing ffi app");
    assert!(
        !run.status.success(),
        "program should fail to start when required binding is missing"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("unable to locate library `ffi_missing_required`"),
        "expected missing library diagnostic, stderr:\n{stderr}"
    );
}

fn compile_shared_library(dir: &Path, stem: &str) -> PathBuf {
    let source_path = dir.join(format!("{stem}.c"));
    fs::write(&source_path, c_program()).expect("write c program");
    let lib_path = dir.join(dynamic_library_filename(stem));

    let mut cmd = Command::new("clang");
    if cfg!(target_os = "macos") {
        cmd.args([
            "-dynamiclib",
            source_path.to_str().expect("utf8 source"),
            "-o",
            lib_path.to_str().expect("utf8 output"),
        ]);
    } else if cfg!(target_os = "linux") {
        cmd.args([
            "-shared",
            "-fPIC",
            source_path.to_str().expect("utf8 source"),
            "-o",
            lib_path.to_str().expect("utf8 output"),
        ]);
    } else if cfg!(target_os = "windows") {
        cmd.args([
            "-shared",
            source_path.to_str().expect("utf8 source"),
            "-o",
            lib_path.to_str().expect("utf8 output"),
        ]);
    } else {
        panic!("dynamic ffi test is not supported on this platform");
    }
    cmd.assert().success();
    lib_path
}

fn dynamic_library_filename(base: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{base}.dylib")
    } else if cfg!(target_os = "linux") {
        format!("lib{base}.so")
    } else {
        format!("{base}.dll")
    }
}

fn platform_executable_name(base: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn host_target() -> String {
    target_lexicon::HOST.to_string()
}

fn c_program() -> &'static str {
    r#"
#if defined(_WIN32)
#define EXPORT __declspec(dllexport)
#else
#define EXPORT
#endif

EXPORT int ffi_add(int left, int right) { return left + right; }
EXPORT int ffi_optional(void) { return 42; }
"#
}

fn chic_program() -> &'static str {
    r#"
namespace Tests.Interop;

@extern(
    convention = "system",
    library = "ffi_math",
    alias = "ffi_add",
    binding = "lazy"
)
internal static extern int NativeAdd(int left, int right);

@extern(
    convention = "system",
    library = "ffi_missing",
    alias = "ffi_optional",
    binding = "lazy",
    optional = true
)
internal static extern int MissingOptional();

public static int Main()
{
    if (NativeAdd(19, 23) != 42)
    {
        return 1;
    }

    if (MissingOptional() != 0)
    {
        return 2;
    }

    return 0;
}
"#
}

fn chic_missing_required_program() -> &'static str {
    r#"
namespace Tests.Interop;

@extern(
    convention = "system",
    library = "ffi_missing_required",
    alias = "ffi_missing_required",
    binding = "eager"
)
internal static extern int MissingRequired();

public static int Main()
{
    return MissingRequired();
}
"#
}
