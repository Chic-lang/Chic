use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn native_runtime_builds_when_toolchain_present() {
    let enabled = std::env::var("CHIC_NATIVE_RUNTIME_TEST").ok();
    if enabled.as_deref() != Some("1") {
        eprintln!("skipping native runtime build test (set CHIC_NATIVE_RUNTIME_TEST=1)");
        return;
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compiler = compiler_path(&manifest_dir);
    if compiler.is_none() {
        eprintln!(
            "skipping native runtime build test (workspace compiler missing at target/{{debug,release}}/chic)"
        );
        return;
    }
    let compiler = compiler.expect("compiler path");
    let manifest = manifest_dir
        .join("packages")
        .join("runtime.native")
        .join("manifest.yaml");
    let identity = runtime_identity(&manifest_dir);
    let archive = runtime_archive_path(&manifest_dir, &identity);
    let artifacts_root = manifest_dir.join("target").join("runtime").join("native");
    std::fs::create_dir_all(archive.parent().expect("archive parent"))
        .expect("create runtime archive parent");
    let mut cmd = Command::new(&compiler);
    cmd.arg("build")
        .arg(&manifest)
        .arg("--backend")
        .arg("llvm")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--artifacts-path")
        .arg(&artifacts_root)
        .arg("--output")
        .arg(&archive)
        .arg("--configuration")
        .arg(default_configuration());
    cmd.current_dir(&manifest_dir);
    cmd.arg("--target").arg(requested_target());
    for flag in compiler_flags() {
        cmd.arg(flag);
    }
    cmd.env("CHIC_SKIP_STDLIB", "1");
    let status = cmd.status().expect("failed to spawn chic build");
    assert!(status.success(), "native runtime build failed");
    assert!(
        archive.exists(),
        "expected archive at {}",
        archive.display()
    );
}

fn runtime_archive_path(manifest_dir: &Path, identity: &str) -> PathBuf {
    manifest_dir
        .join("target")
        .join("runtime")
        .join("native")
        .join(identity)
        .join("libchic_rt_native.a")
}

fn runtime_version(manifest_dir: &Path) -> String {
    if let Ok(version) = std::env::var("RUNTIME_VERSION") {
        if !version.trim().is_empty() {
            return version;
        }
    }
    let manifest = manifest_dir
        .join("packages")
        .join("runtime.native")
        .join("manifest.yaml");
    let contents = std::fs::read_to_string(&manifest).expect("read runtime manifest");
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("version:") {
            let version = value.trim();
            if !version.is_empty() {
                return version.to_string();
            }
        }
    }
    "0.0.0".into()
}

fn runtime_identity(manifest_dir: &Path) -> String {
    if let Ok(identity) = std::env::var("RUNTIME_IDENTITY") {
        if !identity.trim().is_empty() {
            return identity;
        }
    }
    let version = runtime_version(manifest_dir);
    format!("runtime.native@{version}")
}

fn compiler_path(manifest_dir: &Path) -> Option<PathBuf> {
    let debug_compiler = manifest_dir.join("target").join("debug").join("chic");
    if debug_compiler.exists() {
        return Some(debug_compiler);
    }
    let release_compiler = manifest_dir.join("target").join("release").join("chic");
    release_compiler.exists().then_some(release_compiler)
}

fn compiler_flags() -> Vec<String> {
    let raw = std::env::var("CHIC_NATIVE_RUNTIME_FLAGS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("CHIC_COMPILER_FLAGS").ok());
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

fn requested_target() -> String {
    std::env::var("CHIC_NATIVE_RUNTIME_TARGET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("CHIC_TARGET").ok())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("TARGET").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_target)
}

fn default_target() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
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

fn default_configuration() -> String {
    match std::env::var("PROFILE")
        .unwrap_or_else(|_| "debug".into())
        .to_ascii_lowercase()
        .as_str()
    {
        "release" => "Release".into(),
        _ => "Debug".into(),
    }
}
