use std::error::Error;
use std::process::Command;

const TARGET_PATTERNS: &[&str] = &[
    "src/codegen/isa.rs",
    "src/syntax/expr/precedence.rs",
    "src/syntax/expr/builders.rs",
    "src/syntax/expr/parser/calls.rs",
    "src/syntax/expr/parser/operators.rs",
    "src/syntax/expr/parser/lambda.rs",
    "src/syntax/expr/parser/inline_asm.rs",
    "src/syntax/expr/parser/primary.rs",
];

pub fn run() -> Result<(), Box<dyn Error>> {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "-p",
            "chic",
            "--message-format=short",
            "--",
            "--cap-lints",
            "warn",
            "-W",
            "clippy::pedantic",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut flagged = Vec::new();

    for line in stdout.lines().chain(stderr.lines()) {
        if !line.contains("warning:") {
            continue;
        }
        if TARGET_PATTERNS.iter().any(|pattern| line.contains(pattern)) {
            flagged.push(line.to_string());
        }
    }

    if !flagged.is_empty() {
        eprintln!("pedantic warnings detected in guarded modules:");
        for warning in flagged {
            eprintln!("  {warning}");
        }
        return Err("clippy pedantic reported warnings in codegen/parser modules".into());
    }

    if !output.status.success() {
        return Err(format!("cargo clippy failed: {}", output.status).into());
    }

    Ok(())
}
