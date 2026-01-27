use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
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

fn build_library_from_source(root: &Path, subdir: &str, stem: &str, source: &str) -> PathBuf {
    let dir = root.join(subdir);
    std::fs::create_dir_all(&dir).expect("create lib dir");
    let c_path = dir.join(format!("{stem}.c"));
    common::write_source(&c_path, source);
    build_static_library(&dir, stem, &c_path)
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

fn run_chic_build(manifest: &Path, output: &Path, search_paths: &[&Path]) {
    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1")
        .arg("build")
        .arg(manifest)
        .args(["--backend", "llvm", "--target", host_target().as_str()])
        .args(["-o", output.to_str().expect("utf8 artifact path")]);
    for path in search_paths {
        cmd.args(["--ffi-search", path.to_str().expect("utf8 search")]);
    }
    let output = cmd.output().expect("spawn chic build");
    if !output.status.success() {
        panic!(
            "chic build failed (status {:?}):\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn weak_imports_and_definitions_follow_presence() {
    if !common::clang_available() {
        eprintln!("skipping ffi weak linkage test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi weak linkage test: weak imports are unsupported on Windows");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let import_missing_lib = build_library_from_source(
        dir.path(),
        "import_missing",
        "ffi_weak_import",
        include_str!("ffi/weak_import_missing.c"),
    );
    let import_present_lib = build_library_from_source(
        dir.path(),
        "import_present",
        "ffi_weak_import",
        include_str!("ffi/weak_import_present.c"),
    );
    let override_missing_lib = build_library_from_source(
        dir.path(),
        "override_missing",
        "ffi_weak_override",
        include_str!("ffi/weak_override_missing.c"),
    );
    let override_present_lib = build_library_from_source(
        dir.path(),
        "override_present",
        "ffi_weak_override",
        include_str!("ffi/weak_override_present.c"),
    );

    let chic_root = dir.path().join("ffi_weak_chic");
    common::write_sources(
        &chic_root,
        &[
            ("manifest.yaml", include_str!("ffi/weak_chic/manifest.yaml")),
            ("Main.ch", include_str!("ffi/weak_chic/Main.ch")),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");

    let present_artifact = dir
        .path()
        .join(platform_executable_name("ffi_weak_present"));
    let import_present_dir = import_present_lib.parent().expect("import present dir");
    let override_present_dir = override_present_lib.parent().expect("override present dir");
    run_chic_build(
        &chic_manifest,
        &present_artifact,
        &[import_present_dir, override_present_dir],
    );

    let missing_artifact = dir
        .path()
        .join(platform_executable_name("ffi_weak_missing"));
    let import_missing_dir = import_missing_lib.parent().expect("import missing dir");
    let override_missing_dir = override_missing_lib.parent().expect("override missing dir");
    run_chic_build(
        &chic_manifest,
        &missing_artifact,
        &[import_missing_dir, override_missing_dir],
    );

    let present_code = run_artifact(&present_artifact);
    let missing_code = run_artifact(&missing_artifact);

    assert_eq!(
        present_code, 84,
        "present build should see optional symbol and strong override"
    );
    assert_eq!(
        missing_code, 5,
        "missing build should fall back to Chic weak definition and null weak import"
    );
}
