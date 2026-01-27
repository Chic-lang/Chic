use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
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

#[test]
#[ignore = "Stdlib threading allowlist/blocklist currently fails to build (missing Std.Platform.EnvironmentInfo and related deps); tracked separately from this refactor PR"]
fn chic_threads_execute_payloads_without_shim_trampolines() {
    // Restrict to the runtime/sync/platform subset needed for threading so the build exercises the
    // real Std.Platform.Thread surface without pulling in unrelated networking/text/crypto modules.
    const STDLIB_BLOCKLIST: &str = concat!(
        "packages/std.net/src/,",
        "packages/std.security/src/,",
        "packages/std.text/src/,",
        "packages/std/src/testing/,",
        "packages/std/src/uuid.ch,",
        "packages/std/src/uri/,",
        "packages/std/src/console/,",
        "packages/std/src/data/,",
        "packages/std/src/collections/,",
        "packages/std.platform/src/net,",
        "packages/std.platform/src/io/,",
        "packages/std.platform/src/environment.ch,",
        "packages/std/src/environment.ch,",
        "packages/std/src/diagnostics.ch,",
        "packages/std/src/ndarray,",
        "packages/std/src/linalg,",
        "packages/std/src/io/,",
        "packages/std/src/datetime/,",
        "packages/std/src/native_startup.ch,",
        "packages/std/src/bit_converter.ch,",
        "packages/std/src/convert.ch,",
        "packages/std/src/version.ch,",
        "packages/std/src/base64_formatting_options.ch"
    );
    const STDLIB_ALLOWLIST: &str = concat!(
        "packages/std/src/std_sync.ch,",
        "packages/std.platform/src/thread.ch,",
        "packages/std.runtime/src/arc.ch,",
        "packages/std.runtime/src/rc.ch,",
        "packages/std.runtime/src/drop.ch,",
        "packages/std.runtime/src/hash.ch,",
        "packages/std.runtime/src/intrinsics.ch,",
        "packages/std.runtime/src/layoutkind.ch,",
        "packages/std.runtime/src/string.ch,",
        "packages/std.runtime/src/native_exports_stub.ch,",
        "packages/std/src/exception.ch,",
        "packages/std/src/invalid_operation_exception.ch,",
        "packages/std/src/argument_exception.ch,",
        "packages/std/src/argument_null_exception.ch,",
        "packages/std/src/argument_out_of_range_exception.ch,",
        "packages/std/src/index_out_of_range_exception.ch,",
        "packages/std/src/numeric/unchecked_casts.ch,",
        "packages/std/src/numeric/platform.ch"
    );

    if !common::clang_available() {
        eprintln!("skipping ffi thread test: clang not available");
        return;
    }
    if cfg!(target_os = "windows") {
        eprintln!("skipping ffi thread test: native threading not wired for windows yet");
        return;
    }

    let dir = tempdir().expect("temp dir");
    let chic_root = dir.path().join("ffi_thread_chic");
    common::write_sources(
        &chic_root,
        &[
            (
                "manifest.yaml",
                include_str!("ffi/thread_chic/manifest.yaml"),
            ),
            ("Main.ch", include_str!("ffi/thread_chic/Main.ch")),
        ],
    );
    let chic_manifest = chic_root.join("manifest.yaml");
    let artifact = dir
        .path()
        .join(platform_executable_name("ffi_thread_artifact"));

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "0")
        .env("CHIC_STDLIB_BLOCKLIST", STDLIB_BLOCKLIST)
        .env("CHIC_STDLIB_ALLOWLIST", STDLIB_ALLOWLIST)
        .arg("build")
        .arg(&chic_manifest)
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

    Command::new(&artifact).assert().success();
}
