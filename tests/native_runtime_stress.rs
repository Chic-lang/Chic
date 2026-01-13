use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn archive_path() -> Option<PathBuf> {
    env::var_os("CHIC_NATIVE_RUNTIME_ARCHIVE").map(PathBuf::from)
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
    env::var("CHIC_NATIVE_RUNTIME_TEST").as_deref() == Ok("1")
}

#[test]
fn native_runtime_stress_exercises_core_abi_paths() {
    if !should_run() {
        eprintln!(
            "skipping; set CHIC_NATIVE_RUNTIME_TEST=1 and provide CHIC_NATIVE_RUNTIME_ARCHIVE=<path>"
        );
        return;
    }
    let Some(archive) = archive_path() else {
        eprintln!("missing CHIC_NATIVE_RUNTIME_ARCHIVE; ensure the native archive was built");
        return;
    };
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let src = r#"
#include <stdint.h>
#include <string.h>
#include "chic_rt.h"
#include <stdatomic.h>
void chic_thread_drop(ValueMutPtr ctx) { (void)ctx; }
void chic_thread_invoke(ValueMutPtr ctx) { (void)ctx; }
void chic_rt_closure_env_free(void* ptr) { (void)ptr; }
void chic_rt_install_type_metadata(void) {}
static int stress_strings(void) {
    for (int i = 0; i < 500; ++i) {
        ChicString s = chic_rt_string_new();
        const char* msg = "hello";
        ChicStr slice = { (const uint8_t*)msg, strlen(msg) };
        if (chic_rt_string_push_slice(&s, slice) != 0) return 1;
        if (s.len != slice.len) return 2;
        if (chic_rt_string_append_char(&s, (uint32_t)'!', 0, 0, slice) != 0) return 3;
        if (s.len != slice.len + 1) return 4;
        if (chic_rt_string_truncate(&s, 2) != 0) return 5;
        if (s.len != 2) return 6;
        chic_rt_string_drop(&s);
    }
    return 0;
}

static int stress_vec(void) {
    for (int i = 0; i < 400; ++i) {
        ChicVec v = chic_rt_vec_new(sizeof(uint64_t), alignof(uint64_t), 0);
        for (uint64_t n = 0; n < 16; ++n) {
            ValueConstPtr val = { (const uint8_t*)&n, sizeof(uint64_t), alignof(uint64_t) };
            if (chic_rt_vec_push(&v, &val) != 0) return 10;
        }
        if (chic_rt_vec_len(&v) != 16) return 11;
        ChicVecIter iter = chic_rt_vec_iter(&v);
        uint64_t seen = 0;
        while (chic_rt_vec_iter_next_ptr(&iter).ptr != 0) { /* drain */ }
        for (uint64_t n = 0; n < 16; ++n) {
            ValueMutPtr out = { (uint8_t*)&seen, sizeof(uint64_t), alignof(uint64_t) };
            if (chic_rt_vec_pop(&v, &out) != 0) return 12;
        }
        if (chic_rt_vec_len(&v) != 0) return 13;
        chic_rt_vec_drop(&v);
    }
    return 0;
}

static void drop_u64(void* ptr) {
    if (ptr == NULL) return;
    volatile uint64_t* p = (uint64_t*)ptr;
    (void)(*p);
}

static int stress_shared(void) {
    for (int i = 0; i < 300; ++i) {
        uint64_t value = 0xFEEDBEEF + (uint64_t)i;
        ChicArc arc;
        if (chic_rt_arc_new(&arc, (const uint8_t*)&value, sizeof(value), alignof(uint64_t), (uintptr_t)drop_u64, 0xACDC + i) != 0) return 20;
        if (chic_rt_arc_strong_count(&arc) != 1) return 21;
        ChicArc arc2;
        if (chic_rt_arc_clone(&arc2, &arc) != 0) return 22;
        if (chic_rt_arc_strong_count(&arc) != 2) return 23;
        ChicWeak weak;
        if (chic_rt_arc_downgrade(&weak, &arc) != 0) return 24;
        if (chic_rt_arc_weak_count(&arc) < 1) return 25;
        ChicArc upgraded;
        int up = chic_rt_weak_upgrade(&upgraded, &weak);
        if (up != 0) return 26;
        chic_rt_arc_drop(&upgraded);
        chic_rt_weak_drop(&weak);
        chic_rt_arc_drop(&arc);
        chic_rt_arc_drop(&arc2);
    }
    return 0;
}

int main(void) {
    int s = stress_strings();
    if (s != 0) return s;
    int v = stress_vec();
    if (v != 0) return v;
    int sh = stress_shared();
    if (sh != 0) return sh;
    return 0;
}
    "#;

    let tmp_src = env::temp_dir().join("native_runtime_stress.c");
    let exe = tmp_src.with_extension("out");
    fs::write(&tmp_src, src).expect("write stress source");
    let include = include_dir();
    let mut cmd = Command::new(&cc);
    cmd.args([
        "-std=c11",
        "-fno-builtin",
        "-I",
        include.to_str().expect("include dir utf8"),
        tmp_src.to_str().unwrap(),
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
    let status = cmd.status().expect("cc failed");
    assert!(status.success(), "cc failed with {status:?}");
    let status = Command::new(&exe).status().expect("run stress exe");
    assert!(status.success(), "stress exe failed {status:?}");
}
