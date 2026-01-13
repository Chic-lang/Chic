use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, path::Path};
use std::{path::PathBuf, process::Command};

fn main() {
    // The Rust runtime backend has been removed; the crate always expects the
    // Chic-native runtime surface. Even when we skip rebuilding/linking the
    // native archive (e.g. while bootstrapping the compiler), we still define
    // the cfg so the crate compiles under `#![deny(warnings)]`.
    println!("cargo:rustc-check-cfg=cfg(chic_native_runtime)");
    println!("cargo:rustc-cfg=chic_native_runtime");

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=.git/refs/tags");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=packages/runtime.native");
    println!("cargo:rerun-if-env-changed=CI");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=CHIC_NATIVE_RUNTIME_FORCE");
    println!("cargo:rerun-if-env-changed=CHIC_NATIVE_RUNTIME_FLAGS");
    println!("cargo:rerun-if-env-changed=CHIC_COMPILER_FLAGS");
    println!("cargo:rerun-if-env-changed=CHIC_NATIVE_RUNTIME_TARGET");
    println!("cargo:rerun-if-env-changed=CHIC_TARGET");
    println!("cargo:rerun-if-env-changed=CHIC_NATIVE_RUNTIME_BACKEND");
    println!("cargo:rerun-if-env-changed=CHIC_NATIVE_RUNTIME_CONFIGURATION");
    println!("cargo:rerun-if-env-changed=CHIC_CONFIGURATION");
    println!("cargo:rerun-if-env-changed=RUNTIME_VERSION");
    println!("cargo:rerun-if-env-changed=RUNTIME_IDENTITY");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-env-changed=CHIC_BUILD_UNIX_OVERRIDE");
    // Already declared above.

    let git_full = git_full_hash().or_else(env_git_hash_full);
    if let Some(hash) = git_full.as_deref() {
        println!("cargo:rustc-env=CHIC_GIT_HASH_FULL={hash}");
    }
    if let Some(hash) = git_short_hash().or_else(|| git_full.as_deref().and_then(shorten_hash)) {
        println!("cargo:rustc-env=CHIC_GIT_HASH={hash}");
    }
    if let Some(timestamp) = git_commit_unix_timestamp() {
        println!("cargo:rustc-env=CHIC_GIT_COMMIT_UNIX={timestamp}");
    }
    if let Some(dirty) = git_is_dirty() {
        println!(
            "cargo:rustc-env=CHIC_GIT_DIRTY={}",
            if dirty { "true" } else { "false" }
        );
    }

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=CHIC_BUILD_PROFILE={profile}");
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=CHIC_BUILD_TARGET={target}");
    }

    let timestamp = stable_build_unix_timestamp();
    println!("cargo:rustc-env=CHIC_BUILD_UNIX={timestamp}");

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-env-changed=CHIC_BOOTSTRAP_COMPILER");
    if env::var("CHIC_BOOTSTRAP_COMPILER").is_ok() {
        // When bootstrapping the workspace from a clean `target/`, we must first build a minimal
        // `chic` binary so we can compile `packages/runtime.native` into a linkable archive. The
        // bootstrap build itself cannot depend on that archive yet, so we link a small stub with
        // just the runtime symbols the compiler executable needs at startup.
        link_bootstrap_runtime_stub();
        return;
    }

    let compiler = bootstrap_compiler_path()
        .map(PathBuf::from)
        .or_else(|| default_compiler_path().map(PathBuf::from))
        .unwrap_or_else(|| {
            panic!(
                "Chic compiler binary missing; expected target/debug/chic or target/release/chic in the workspace (or bootstrap build failed)"
            )
        });
    let runtime_id = runtime_identity();
    let archive = Path::new("target")
        .join("runtime")
        .join("native")
        .join(&runtime_id)
        .join("libchic_rt_native.a");
    let skip_runtime_build = env::var("CHIC_NATIVE_RUNTIME_SKIP_BUILD").is_ok();
    let needs_rebuild = archive_is_stale(&archive);
    if needs_rebuild && !skip_runtime_build {
        let out_dir = archive
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/runtime/native"));
        build_native_runtime(&compiler, &archive, &out_dir);
    }
    let archive = if archive.exists() {
        archive
    } else {
        let alt_archive = env::var_os("OUT_DIR")
            .map(PathBuf::from)
            .map(|dir| dir.join("libchic_rt_native.a"))
            .filter(|path| path.exists());
        alt_archive.unwrap_or_else(|| {
            panic!(
                "Native runtime archive missing after build; expected {}",
                archive.display()
            )
        })
    };
    link_native_runtime(&archive);
}

fn stable_build_unix_timestamp() -> String {
    if let Ok(value) = env::var("CHIC_BUILD_UNIX_OVERRIDE") {
        if value.parse::<u64>().is_ok() {
            return value;
        }
    }
    if let Ok(value) = env::var("SOURCE_DATE_EPOCH") {
        if value.parse::<u64>().is_ok() {
            return value;
        }
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or_else(|_| "0".into(), |duration| duration.as_secs().to_string())
}

fn link_bootstrap_runtime_stub() {
    let out_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/bootstrap/out"));
    let source = out_dir.join("chic_runtime_bootstrap_stub.c");
    let object = out_dir.join("chic_runtime_bootstrap_stub.o");
    let archive = out_dir.join("libchic_rt_native.a");

    let stub = r#"
#include <stdint.h>

int chic_rt_peek_pending_exception(int64_t* payload, int64_t* type_id) {
  (void)payload;
  (void)type_id;
  return 0;
}

void chic_rt_throw(int64_t payload, int64_t type_id) {
  (void)payload;
  (void)type_id;
}

int chic_rt_has_pending_exception(void) {
  return 0;
}

void chic_rt_clear_pending_exception(void) {
}

int chic_rt_take_pending_exception(int64_t* payload, int64_t* type_id) {
  (void)payload;
  (void)type_id;
  return 0;
}

void chic_rt_abort_unhandled_exception(void) {
}

void chic_rt_trace_enter(uint64_t trace_id, const uint8_t* label_ptr, uint64_t label_len) {
  (void)trace_id;
  (void)label_ptr;
  (void)label_len;
}

void chic_rt_trace_exit(uint64_t trace_id) {
  (void)trace_id;
}

int32_t chic_rt_trace_flush(const uint8_t* path_ptr, uint64_t len) {
  (void)path_ptr;
  (void)len;
  return 0;
}
"#;
    std::fs::write(&source, stub).expect("write bootstrap runtime stub source");

    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let status = Command::new(cc)
        .arg("-c")
        .arg(source.display().to_string())
        .arg("-o")
        .arg(object.display().to_string())
        .status()
        .expect("compile bootstrap runtime stub");
    if !status.success() {
        panic!("failed to compile bootstrap runtime stub (status: {status})");
    }

    let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
    let status = Command::new(ar)
        .arg("crs")
        .arg(archive.display().to_string())
        .arg(object.display().to_string())
        .status()
        .expect("archive bootstrap runtime stub");
    if !status.success() {
        panic!("failed to archive bootstrap runtime stub (status: {status})");
    }

    let ranlib = env::var("RANLIB").unwrap_or_else(|_| "ranlib".into());
    let _ = Command::new(ranlib)
        .arg(archive.display().to_string())
        .status();

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=chic_rt_native");
}

fn git_commit_unix_timestamp() -> Option<String> {
    let output = Command::new("git")
        .args(["show", "-s", "--format=%ct", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    raw.parse::<u64>().ok()?;
    Some(raw)
}

fn git_full_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?;
    let trimmed = hash.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn env_git_hash_full() -> Option<String> {
    let hash = env::var("GITHUB_SHA").ok()?;
    let trimmed = hash.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn shorten_hash(hash: &str) -> Option<String> {
    let trimmed = hash.trim();
    if trimmed.len() < 7 {
        return None;
    }
    Some(trimmed.chars().take(8).collect())
}

fn git_is_dirty() -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(!output.stdout.is_empty())
}

fn bootstrap_compiler_path() -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let target_dir = Path::new(&manifest_dir).join("target").join("bootstrap");
    let artifact_dir = if profile.eq_ignore_ascii_case("release") {
        target_dir.join("release")
    } else {
        target_dir.join("debug")
    };
    let chic = artifact_dir.join("chic");
    if chic.exists() {
        let binary_mtime = chic.metadata().and_then(|m| m.modified()).ok();
        let source_mtime = newest_compiler_source_mtime(Path::new(&manifest_dir));
        if binary_mtime.is_some() && source_mtime.is_some() && binary_mtime < source_mtime {
            // Fall through and rebuild.
        } else {
            return Some(chic.display().to_string());
        }
    }

    let mut cmd = Command::new(cargo);
    cmd.current_dir(&manifest_dir)
        .arg("build")
        .arg("--bin")
        .arg("chic")
        .arg("--target-dir")
        .arg(target_dir.display().to_string())
        .env("CHIC_BOOTSTRAP_COMPILER", "1");
    if profile.eq_ignore_ascii_case("release") {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .unwrap_or_else(|err| panic!("failed to bootstrap chic compiler binary: {err}"));
    if !status.success() {
        panic!("bootstrap build for chic compiler failed with status {status}");
    }

    chic.exists().then(|| chic.display().to_string())
}

fn newest_compiler_source_mtime(manifest_dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    for path in [
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("Cargo.lock"),
        manifest_dir.join("build.rs"),
    ] {
        if let Ok(meta) = path.metadata() {
            if let Ok(mtime) = meta.modified() {
                if newest.map_or(true, |current| mtime > current) {
                    newest = Some(mtime);
                }
            }
        }
    }

    let src_root = manifest_dir.join("src");
    if src_root.exists() {
        let mut stack = vec![src_root];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = dir.read_dir() {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                        continue;
                    }
                    if let Ok(meta) = path.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            if newest.map_or(true, |current| mtime > current) {
                                newest = Some(mtime);
                            }
                        }
                    }
                }
            }
        }
    }

    newest
}

fn runtime_version() -> String {
    if let Ok(version) = env::var("RUNTIME_VERSION") {
        if !version.trim().is_empty() {
            return version;
        }
    }
    let manifest = Path::new("packages/runtime.native/manifest.yaml");
    if let Ok(contents) = std::fs::read_to_string(manifest) {
        for line in contents.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("version:") {
                let version = value.trim();
                if !version.is_empty() {
                    return version.to_string();
                }
            }
        }
    }
    "0.0.0".into()
}

fn runtime_identity() -> String {
    if let Ok(identity) = env::var("RUNTIME_IDENTITY") {
        if !identity.trim().is_empty() {
            return identity;
        }
    }
    let version = runtime_version();
    format!("runtime.native@{version}")
}

fn newest_native_runtime_mtime() -> Option<std::time::SystemTime> {
    let root = Path::new("packages/runtime.native");
    let mut newest: Option<SystemTime> = None;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = dir.read_dir() {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if newest.map_or(true, |current| mtime > current) {
                            newest = Some(mtime);
                        }
                    }
                }
            }
        }
    }
    newest
}

fn archive_is_stale(archive: &Path) -> bool {
    if env::var("CHIC_NATIVE_RUNTIME_FORCE").is_ok() {
        return true;
    }
    let archive_mtime = archive.metadata().and_then(|m| m.modified()).ok();
    let src_mtime = newest_native_runtime_mtime();
    match (archive_mtime, src_mtime) {
        (Some(archive), Some(src)) => src > archive,
        (None, _) => true,
        (_, None) => false,
    }
}

fn compiler_flags() -> Vec<String> {
    let raw = env::var("CHIC_NATIVE_RUNTIME_FLAGS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var("CHIC_COMPILER_FLAGS").ok());
    let mut flags: Vec<String> = raw
        .as_deref()
        .map(|value| {
            value
                .split_whitespace()
                .map(|token| token.to_string())
                .collect()
        })
        .unwrap_or_default();
    let has_cpu_isa = flags
        .windows(2)
        .any(|pair| pair.get(0).is_some_and(|flag| flag == "--cpu-isa"));
    if !has_cpu_isa {
        flags.push("--cpu-isa".into());
        flags.push("baseline".into());
    }
    flags
}

fn default_target() -> String {
    let arch = env::consts::ARCH;
    let os = env::consts::OS;
    let mapped_arch = match arch {
        "arm" | "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => other,
    };
    let suffix = match os {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => other,
    };
    format!("{mapped_arch}-{suffix}")
}

fn runtime_configuration() -> String {
    if let Ok(value) = env::var("CHIC_NATIVE_RUNTIME_CONFIGURATION") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if let Ok(value) = env::var("CHIC_CONFIGURATION") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    match env::var("PROFILE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "release" => "Release".into(),
        _ => "Debug".into(),
    }
}

fn build_native_runtime(compiler: &Path, archive: &Path, out_dir: &Path) {
    let manifest = Path::new("packages/runtime.native/manifest.yaml");
    if !manifest.exists() {
        panic!(
            "runtime manifest missing at {}; expected runtime.native as a package",
            manifest.display()
        );
    }
    if let Some(parent) = archive.parent() {
        std::fs::create_dir_all(parent).expect("create archive parent");
    }
    let target = env::var("CHIC_NATIVE_RUNTIME_TARGET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var("CHIC_TARGET").ok())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env::var("TARGET").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_target);
    let backend = env::var("CHIC_NATIVE_RUNTIME_BACKEND")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "llvm".to_string());
    let configuration = runtime_configuration();
    let artifacts_root = out_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new("target").join("runtime").join("native"));
    let mut cmd = Command::new(compiler);
    cmd.arg("build")
        .arg(manifest)
        .arg("--backend")
        .arg(&backend)
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--configuration")
        .arg(&configuration)
        .arg("--target")
        .arg(&target)
        .arg("--artifacts-path")
        .arg(artifacts_root.display().to_string())
        .arg("--output")
        .arg(archive.display().to_string());
    for flag in compiler_flags() {
        cmd.arg(flag);
    }
    cmd.env("CHIC_SKIP_STDLIB", "1");
    // The GitHub Actions runner sets `CI=1`, which makes Chic's formatter/code-style defaults
    // treat diagnostics as errors. Building the runtime as part of the Rust workspace build
    // should not be blocked on Chic formatting hygiene; CI has dedicated guardrails for that.
    cmd.env("CHIC_CI", "0");
    let status = cmd
        .status()
        .unwrap_or_else(|err| panic!("failed to run chic build for runtime.native: {err}"));
    if !status.success() {
        panic!(
            "native runtime build failed with status {status}; rerun `chic build {}` manually for diagnostics",
            manifest.display()
        );
    }
}

fn link_native_runtime(path: &Path) {
    let archive_dir = path.parent().expect("archive parent");
    println!("cargo:rustc-link-search=native={}", archive_dir.display());
    println!("cargo:rustc-link-lib=static=chic_rt_native");
    println!("cargo:rustc-env=CHIC_NATIVE_RUNTIME_LINKED=1");
    println!("cargo:rustc-cfg=chic_native_runtime");
    println!(
        "cargo:rustc-env=CHIC_NATIVE_RUNTIME_ARCHIVE={}",
        path.display()
    );
    println!("cargo:rerun-if-changed={}", path.display());
}

fn default_compiler_path() -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    let debug = Path::new(&manifest_dir)
        .join("target")
        .join("debug")
        .join("chic");
    if debug.exists() {
        return Some(debug.display().to_string());
    }
    let release = Path::new(&manifest_dir)
        .join("target")
        .join("release")
        .join("chic");
    if release.exists() {
        return Some(release.display().to_string());
    }
    None
}

fn git_short_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?;
    let trimmed = hash.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
