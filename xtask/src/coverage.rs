use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CoverageOptions {
    pub min_percent: f64,
    pub output: Option<PathBuf>,
}

pub fn run(options: CoverageOptions) -> Result<(), Box<dyn std::error::Error>> {
    ensure_tooling()?;
    let output_path = options
        .output
        .unwrap_or_else(|| PathBuf::from("coverage/coverage.json"));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    status("cargo llvm-cov clean --workspace")?;
    let mut cov_cmd = Command::new("cargo");
    cov_cmd.args([
        "llvm-cov",
        "--workspace",
        "--json",
        "--output-path",
        output_path
            .to_str()
            .ok_or("coverage output path is not valid UTF-8")?,
        "--summary-only",
    ]);
    status_command(&mut cov_cmd, "cargo llvm-cov --workspace")?;

    let json = fs::read_to_string(&output_path)?;
    let payload: serde_json::Value = serde_json::from_str(&json)?;
    let percent = payload["data"][0]["totals"]["lines"]["percent"]
        .as_f64()
        .ok_or("coverage JSON missing totals.lines.percent")?;
    println!(
        "[coverage] lines percent: {:.2}% (min {:.2}%)",
        percent, options.min_percent
    );
    if percent + f64::EPSILON < options.min_percent {
        return Err(format!(
            "coverage {:.2}% below required {:.2}%",
            percent, options.min_percent
        )
        .into());
    }
    Ok(())
}

fn ensure_tooling() -> Result<(), Box<dyn std::error::Error>> {
    status("cargo llvm-cov --version")?;
    status("rustup component add llvm-tools-preview")?;
    Ok(())
}

fn status(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = command.split_whitespace();
    let program = parts.next().ok_or("missing command")?;
    let mut cmd = Command::new(program);
    cmd.args(parts);
    status_command(&mut cmd, command)
}

fn status_command(cmd: &mut Command, display: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = cmd.status()?;
    if !status.success() {
        return Err(format!("command `{display}` failed with {status}").into());
    }
    Ok(())
}
