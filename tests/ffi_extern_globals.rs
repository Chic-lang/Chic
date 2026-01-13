use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{LazyLock, Mutex};
use tempfile::tempdir;

mod common;

static BUILD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

fn build_static_library(dir: &Path, stem: &str, c_source: &Path, flags: &[&str]) -> PathBuf {
    let object_path = dir.join(format!("{stem}.o"));
    let mut clang = Command::new("clang");
    if !flags.is_empty() {
        clang.args(flags);
    }
    clang
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
        let _ = StdCommand::new("ranlib").arg(&lib_path).status();
    }
    lib_path
}

fn run_artifact(path: &Path) -> i32 {
    let status = StdCommand::new(path)
        .status()
        .unwrap_or_else(|err| panic!("execute {}: {err}", path.display()));
    status.code().unwrap_or_else(|| {
        panic!(
            "binary {} terminated without exit code: {:?}",
            path.display(),
            status
        )
    })
}

fn chic_build(
    manifest: &Path,
    output: &Path,
    search_paths: &[&Path],
    backend: &str,
    target: &str,
) -> std::process::Output {
    let _guard = BUILD_LOCK.lock().expect("build lock");
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DISABLE_BUILD_SERVERS", "1")
        .arg("build")
        .arg(manifest)
        .args(["--backend", backend, "--target", target])
        .args(["-o", output.to_str().expect("utf8 artifact path")]);
    if backend.eq_ignore_ascii_case("wasm") {
        cmd.args(["--runtime", "wasm"]);
    }
    for path in search_paths {
        cmd.args(["--ffi-search", path.to_str().expect("utf8 search")]);
    }
    cmd.output().expect("spawn chic build")
}

#[test]
fn extern_globals_roundtrip_across_c_boundary() {
    if !common::clang_available() {
        eprintln!("skipping extern globals test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping extern globals test on Windows");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let c_path = dir.path().join("extern_globals_basic.c");
    common::write_source(&c_path, include_str!("ffi/extern_globals_basic.c"));
    let lib_path = build_static_library(dir.path(), "ffi_extern_globals_basic", &c_path, &[]);

    let chic_root = dir.path().join("extern_globals_basic");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/extern_globals/basic/manifest.yaml"),
            ),
            ("Main.cl", include_str!("ffi/extern_globals/basic/Main.cl")),
        ],
    );
    let manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("extern_globals_basic_bin"));

    let output = chic_build(
        &manifest,
        &artifact,
        &[lib_path.parent().expect("lib dir")],
        "llvm",
        &host_target(),
    );
    if !output.status.success() {
        panic!(
            "chic build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let code = run_artifact(&artifact);
    assert_eq!(code, 0, "extern globals roundtrip failed");
}

#[test]
fn extern_tls_globals_are_thread_isolated() {
    if !common::clang_available() {
        eprintln!("skipping extern TLS globals test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping extern TLS globals test on Windows");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let c_path = dir.path().join("extern_globals_tls.c");
    common::write_source(&c_path, include_str!("ffi/extern_globals_tls.c"));
    let lib_path =
        build_static_library(dir.path(), "ffi_extern_globals_tls", &c_path, &["-pthread"]);

    let chic_root = dir.path().join("extern_globals_tls");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/extern_globals/tls/manifest.yaml"),
            ),
            ("Main.cl", include_str!("ffi/extern_globals/tls/Main.cl")),
        ],
    );
    let manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("extern_globals_tls_bin"));

    let output = chic_build(
        &manifest,
        &artifact,
        &[lib_path.parent().expect("lib dir")],
        "llvm",
        &host_target(),
    );
    if !output.status.success() {
        panic!(
            "chic build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let code = run_artifact(&artifact);
    assert_eq!(code, 0, "extern TLS globals test failed");
}

#[test]
fn weak_import_globals_resolve_when_present() {
    if !common::clang_available() {
        eprintln!("skipping weak extern globals test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping weak extern globals test on Windows");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let present_dir = dir.path().join("present");
    std::fs::create_dir_all(&present_dir).expect("create present dir");
    let missing_dir = dir.path().join("missing");
    std::fs::create_dir_all(&missing_dir).expect("create missing dir");

    let present_c = present_dir.join("extern_globals_weak.c");
    common::write_source(
        &present_c,
        include_str!("ffi/extern_globals_weak_present.c"),
    );
    let _present_lib = build_static_library(&present_dir, "ffi_extern_global_opt", &present_c, &[]);

    let missing_c = missing_dir.join("extern_globals_weak.c");
    common::write_source(
        &missing_c,
        include_str!("ffi/extern_globals_weak_missing.c"),
    );
    let _missing_lib = build_static_library(&missing_dir, "ffi_extern_global_opt", &missing_c, &[]);

    let chic_root = dir.path().join("extern_globals_weak");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/extern_globals/weak/manifest.yaml"),
            ),
            ("Main.cl", include_str!("ffi/extern_globals/weak/Main.cl")),
        ],
    );
    let manifest = chic_root.join("manifest.yaml");

    let present_artifact = dir
        .path()
        .join(platform_executable_name("extern_globals_weak_present"));
    let present_output = chic_build(
        &manifest,
        &present_artifact,
        &[&present_dir],
        "llvm",
        &host_target(),
    );
    if !present_output.status.success() {
        panic!(
            "present weak build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&present_output.stdout),
            String::from_utf8_lossy(&present_output.stderr)
        );
    }

    let missing_artifact = dir
        .path()
        .join(platform_executable_name("extern_globals_weak_missing"));
    let missing_output = chic_build(
        &manifest,
        &missing_artifact,
        &[&missing_dir],
        "llvm",
        &host_target(),
    );
    if !missing_output.status.success() {
        panic!(
            "missing weak build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&missing_output.stdout),
            String::from_utf8_lossy(&missing_output.stderr)
        );
    }

    let present_code = run_artifact(&present_artifact);
    let missing_code = run_artifact(&missing_artifact);

    assert_eq!(present_code, 42, "weak present build should see the symbol");
    assert_eq!(
        missing_code, 5,
        "weak missing build should null out the symbol"
    );
}

#[test]
fn wasm_backend_rejects_extern_globals() {
    if !common::clang_available() {
        eprintln!("skipping wasm extern globals test: clang not available");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let chic_root = dir.path().join("extern_globals_wasm");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/extern_globals/basic/manifest.yaml"),
            ),
            ("Main.cl", include_str!("ffi/extern_globals/basic/Main.cl")),
        ],
    );
    let manifest = chic_root.join("manifest.yaml");
    let artifact = dir.path().join("extern_globals_wasm.wasm");

    let output = chic_build(&manifest, &artifact, &[], "wasm", &host_target());
    assert!(
        !output.status.success(),
        "wasm build unexpectedly succeeded for extern globals"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("extern globals are unsupported on the WASM backend"),
        "expected wasm backend to reject extern globals, stderr:\n{stderr}"
    );
}
