#[path = "codegen_exec/error.rs"]
mod error;
#[path = "codegen_exec/fixtures.rs"]
mod fixtures;
#[path = "codegen_exec/happy.rs"]
mod happy;
#[path = "codegen_exec/harness.rs"]
mod harness;
#[path = "codegen_exec/perf.rs"]
mod perf;

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn perf_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_PERF").unwrap_or(false)
}

fn clang_available() -> bool {
    std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok()
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}
