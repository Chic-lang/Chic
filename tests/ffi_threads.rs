use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
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
fn chic_threads_execute_payloads_without_shim_trampolines() {
    // Restrict to the runtime/sync/platform subset needed for threading so the build exercises the
    // real Std.Platform.Thread surface without pulling in unrelated networking/text/crypto modules.
    const STDLIB_BLOCKLIST: &str = concat!(
        "packages/std.net/src/,",
        "packages/std.security/src/,",
        "packages/std.text/src/,",
        "packages/std/src/testing/,",
        "packages/std/src/uuid.cl,",
        "packages/std/src/uri/,",
        "packages/std/src/console/,",
        "packages/std/src/data/,",
        "packages/std/src/collections/,",
        "packages/std.platform/src/net,",
        "packages/std.platform/src/io/,",
        "packages/std.platform/src/environment.cl,",
        "packages/std/src/environment.cl,",
        "packages/std/src/diagnostics.cl,",
        "packages/std/src/ndarray,",
        "packages/std/src/linalg,",
        "packages/std/src/io/,",
        "packages/std/src/datetime/,",
        "packages/std/src/native_startup.cl,",
        "packages/std/src/bit_converter.cl,",
        "packages/std/src/convert.cl,",
        "packages/std/src/version.cl,",
        "packages/std/src/base64_formatting_options.cl"
    );
    const STDLIB_ALLOWLIST: &str = concat!(
        "packages/std/src/std_sync.cl,",
        "packages/std.platform/src/thread.cl,",
        "packages/std.runtime/src/arc.cl,",
        "packages/std.runtime/src/rc.cl,",
        "packages/std.runtime/src/drop.cl,",
        "packages/std.runtime/src/hash.cl,",
        "packages/std.runtime/src/intrinsics.cl,",
        "packages/std.runtime/src/layoutkind.cl,",
        "packages/std.runtime/src/string.cl,",
        "packages/std.runtime/src/native_exports_stub.cl,",
        "packages/std/src/exception.cl,",
        "packages/std/src/invalid_operation_exception.cl,",
        "packages/std/src/argument_exception.cl,",
        "packages/std/src/argument_null_exception.cl,",
        "packages/std/src/argument_out_of_range_exception.cl,",
        "packages/std/src/index_out_of_range_exception.cl,",
        "packages/std/src/numeric/unchecked_casts.cl,",
        "packages/std/src/numeric/platform.cl"
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
            ("Main.cl", include_str!("ffi/thread_chic/Main.cl")),
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
