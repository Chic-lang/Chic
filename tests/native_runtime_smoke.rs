use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn native_archive() -> Option<PathBuf> {
    std::env::var_os("CHIC_NATIVE_RUNTIME_ARCHIVE").map(PathBuf::from)
}

fn include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("runtime")
        .join("include")
}

fn compiler_builtins(cc: &str) -> Option<String> {
    let output = Command::new(cc)
        .arg("--print-libgcc-file-name")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let path = text.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

fn should_run() -> bool {
    std::env::var("CHIC_NATIVE_RUNTIME_TEST").as_deref() == Ok("1")
}

#[test]
fn native_runtime_c_smoke_links_and_runs() {
    if !should_run() {
        eprintln!(
            "skipping; set CHIC_NATIVE_RUNTIME_TEST=1 and provide CHIC_NATIVE_RUNTIME_ARCHIVE"
        );
        return;
    }
    let Some(archive) = native_archive() else {
        eprintln!("missing CHIC_NATIVE_RUNTIME_ARCHIVE; ensure build.rs built the native archive");
        return;
    };
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let work = std::env::temp_dir().join("native_runtime_smoke.c");
    let exe = work.with_extension("out");
    let source = r#"
#include "chic_rt.h"
#include <stdatomic.h>
void chic_thread_drop(ValueMutPtr ctx) { (void)ctx; }
void chic_thread_invoke(ValueMutPtr ctx) { (void)ctx; }
void chic_rt_closure_env_free(void* ptr) { (void)ptr; }
void chic_rt_install_type_metadata(void) {}
int main(void) {
    if (chic_rt_string_inline_capacity() != CHIC_RT_STRING_INLINE_CAPACITY) return 11;
    ChicString s = chic_rt_string_new();
    if (s.len != 0) return 12;
    chic_rt_string_drop(&s);

    ChicVec v = chic_rt_vec_new(sizeof(uint64_t), alignof(uint64_t), 0);
    size_t inline_cap = chic_rt_vec_inline_capacity(&v);
    if (inline_cap == 0 || inline_cap > 8) return 13;
    chic_rt_vec_drop(&v);

    ChicArc arc;
    uint64_t payload = 1234;
    if (chic_rt_arc_new(&arc, (const uint8_t*)&payload, sizeof(payload), alignof(uint64_t), 0, 0) != 0) return 14;
    if (chic_rt_arc_strong_count(&arc) != 1) return 15;
    ChicArc arc2;
    if (chic_rt_arc_clone(&arc2, &arc) != 0) return 16;
    if (chic_rt_arc_strong_count(&arc) != 2) return 17;
    chic_rt_arc_drop(&arc);
    chic_rt_arc_drop(&arc2);

    return 0;
}
    "#;
    fs::write(&work, source).expect("write smoke source");
    let include = include_dir();
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-I",
        include.to_str().expect("include dir utf8"),
        "-std=c11",
        "-fno-builtin",
        work.to_str().unwrap(),
        "-o",
        exe.to_str().unwrap(),
    ]);
    if cfg!(target_os = "macos") {
        cmd.args(["-Wl,-force_load", archive.to_str().unwrap()]);
    } else {
        cmd.args([
            "-Wl,--whole-archive",
            archive.to_str().unwrap(),
            "-Wl,--no-whole-archive",
        ]);
    }
    if let Some(builtins) = compiler_builtins(&cc) {
        cmd.arg(builtins);
    }
    let status = cmd.status().expect("failed to run cc");
    assert!(status.success(), "cc failed with status {status:?}");
    let status = Command::new(&exe).status().expect("run smoke exe");
    assert!(status.success(), "smoke exe failed {status:?}");
}
