use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

mod common;
use common::{clang_available, write_source};

fn static_library_filename(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.lib")
    } else {
        format!("lib{base}.a")
    }
}

fn dynamic_library_filename(base: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{base}.dylib")
    } else if cfg!(windows) {
        format!("{base}.dll")
    } else {
        format!("lib{base}.so")
    }
}

fn platform_executable_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn link_name_for(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("dynamic library filename");
    let trimmed = file_name.strip_prefix("lib").unwrap_or(file_name);
    trimmed.split('.').next().expect("library stem").to_string()
}

#[test]
fn emit_header_without_clrlib_packaging() {
    if !clang_available() {
        eprintln!("skipping header emission test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let src_path = dir.path().join("header_only.cl");
    write_source(
        &src_path,
        r"
namespace HeaderOnly;

/// <summary>Returns the sum of two integers.</summary>
public int Sum(int left, int right)
{
    return left + right;
}
",
    );

    let artifact_path = dir.path().join(static_library_filename("header_only"));
    cargo_bin_cmd!("chic")
        .arg("build")
        .arg(&src_path)
        .env("CHIC_SKIP_STDLIB", "1")
        .args([
            "--backend",
            "llvm",
            "--crate-type",
            "lib",
            "-o",
            artifact_path.to_str().expect("utf8 artifact path"),
            "--emit-header",
        ])
        .assert()
        .success();

    let header_path = artifact_path.with_extension("h");
    assert!(
        header_path.exists(),
        "expected header file at {}",
        header_path.display()
    );
    let clrlib_path = artifact_path.with_extension("clrlib");
    assert!(
        !clrlib_path.exists(),
        "did not expect a clrlib archive when --emit-lib is omitted"
    );

    let header_file = header_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("header filename");
    let consumer_path = dir.path().join("header_check.c");
    let consumer_source = format!(
        "#include \"{header_file}\"\nint main(void) {{ return HeaderOnly__Sum(19, 23) == 42 ? 0 : 1; }}\n"
    );
    write_source(&consumer_path, &consumer_source);

    let object_path = dir.path().join("header_check.o");
    let status = StdCommand::new("clang")
        .arg("-std=c11")
        .arg("-c")
        .arg(&consumer_path)
        .arg("-I")
        .arg(dir.path())
        .arg("-o")
        .arg(&object_path)
        .status()
        .expect("compile consumer");
    assert!(status.success(), "clang compilation failed: {status:?}");
    assert!(object_path.exists(), "expected consumer object file");
}

#[test]
fn static_library_links_via_generated_header() {
    if !clang_available() {
        eprintln!("skipping static linkage test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let src_path = dir.path().join("static_api.cl");
    write_source(
        &src_path,
        r"
namespace StaticInterop;

/// <summary>Returns the product of two integers.</summary>
public int Multiply(int left, int right)
{
    return left * right;
}
",
    );

    let artifact_path = dir.path().join(static_library_filename("static_api"));
    cargo_bin_cmd!("chic")
        .arg("build")
        .arg(&src_path)
        .env("CHIC_SKIP_STDLIB", "1")
        .args([
            "--backend",
            "llvm",
            "--crate-type",
            "lib",
            "-o",
            artifact_path.to_str().expect("utf8 artifact path"),
            "--emit-header",
            "--emit-lib",
        ])
        .assert()
        .success();

    let header_path = artifact_path.with_extension("h");
    let clrlib_path = artifact_path.with_extension("clrlib");
    assert!(
        header_path.exists(),
        "header missing at {}",
        header_path.display()
    );
    assert!(
        clrlib_path.exists(),
        "expected clrlib archive at {}",
        clrlib_path.display()
    );

    let header_file = header_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("header filename");
    let consumer_path = dir.path().join("static_consumer.c");
    let consumer_source = format!(
        "#include \"{header_file}\"\nint main(void) {{ return StaticInterop__Multiply(6, 7) == 42 ? 0 : 1; }}\n"
    );
    write_source(&consumer_path, &consumer_source);

    let exe_path = dir.path().join(platform_executable_name("static_consumer"));
    let runtime_lib = native_runtime_path();
    let status = StdCommand::new("clang")
        .arg("-std=c11")
        .arg(&consumer_path)
        .arg(&artifact_path)
        .arg(&runtime_lib)
        .arg("-fprofile-instr-generate")
        .arg("-I")
        .arg(dir.path())
        .arg("-o")
        .arg(&exe_path)
        .status()
        .expect("link static consumer");
    assert!(status.success(), "clang linking failed: {status:?}");
    assert!(exe_path.exists(), "expected linked executable");

    let status = StdCommand::new(&exe_path)
        .status()
        .expect("run static consumer");
    assert!(status.success(), "static consumer exited with {status:?}");
}

#[test]
fn dynamic_library_links_via_generated_header() {
    if cfg!(windows) {
        eprintln!("skipping dynamic linkage test on Windows – platform setup requires import libs");
        return;
    }
    if !clang_available() {
        eprintln!("skipping dynamic linkage test: clang not available");
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let src_path = dir.path().join("dynamic_api.cl");
    write_source(
        &src_path,
        r"
namespace SharedInterop;

/// <summary>Returns the input value multiplied by three.</summary>
public int Triple(int value)
{
    return value * 3;
}
",
    );

    let artifact_path = dir.path().join(dynamic_library_filename("dynamic_api"));
    cargo_bin_cmd!("chic")
        .arg("build")
        .arg(&src_path)
        .env("CHIC_SKIP_STDLIB", "1")
        .args([
            "--backend",
            "llvm",
            "--crate-type",
            "dylib",
            "-o",
            artifact_path.to_str().expect("utf8 artifact path"),
            "--emit-header",
            "--emit-lib",
        ])
        .assert()
        .success();

    let header_path = artifact_path.with_extension("h");
    assert!(
        header_path.exists(),
        "expected header at {}",
        header_path.display()
    );

    let header_file = header_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("header filename");
    let consumer_path = dir.path().join("dynamic_consumer.c");
    let consumer_source = format!(
        "#include \"{header_file}\"\nint main(void) {{ return SharedInterop__Triple(14) == 42 ? 0 : 1; }}\n"
    );
    write_source(&consumer_path, &consumer_source);

    let exe_path = dir
        .path()
        .join(platform_executable_name("dynamic_consumer"));
    let lib_name = link_name_for(&artifact_path);
    let status = StdCommand::new("clang")
        .arg("-std=c11")
        .arg(&consumer_path)
        .arg("-I")
        .arg(dir.path())
        .arg("-L")
        .arg(dir.path())
        .arg("-fprofile-instr-generate")
        .arg(format!("-l{lib_name}"))
        .arg("-o")
        .arg(&exe_path)
        .status()
        .expect("link dynamic consumer");
    assert!(status.success(), "clang linking failed: {status:?}");
    assert!(exe_path.exists(), "expected dynamic consumer executable");

    let mut run = StdCommand::new(&exe_path);
    if cfg!(target_os = "macos") {
        run.env("DYLD_LIBRARY_PATH", dir.path());
    } else {
        run.env("LD_LIBRARY_PATH", dir.path());
    }
    let status = run.status().expect("run dynamic consumer");
    assert!(status.success(), "dynamic consumer exited with {status:?}");
}

fn native_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("runtime")
        .join("native")
        .join("runtime.native@0.1.0")
        .join("libchic_rt_native.a")
}
