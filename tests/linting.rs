use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn lint_errors_on_dead_code_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let file = dir.path().join("dead_code.cl");
    fs::write(
        &file,
        r#"
namespace Lint;

public int Main() { return 0; }

public int UnusedHelper() { return 1; }
"#,
    )?;

    cargo_bin_cmd!("chic")
        .current_dir(dir.path())
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["lint", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("dead_code"))
        .stdout(predicate::str::contains("UnusedHelper"))
        .stdout(predicate::str::contains(
            "function `Lint::UnusedHelper` is never used",
        ));
    Ok(())
}

#[test]
fn lint_respects_allow_attribute_and_config_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let file = dir.path().join("suppressed.cl");
    fs::write(dir.path().join("lint.yaml"), "rules:\n  dead_code: allow\n")?;
    fs::write(
        &file,
        r#"
namespace Lint;

@allow(dead_code)
public int UnusedHelper() { return 2; }

public int Main() { return 0; }
"#,
    )?;

    cargo_bin_cmd!("chic")
        .current_dir(dir.path())
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["lint", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("lint passed"))
        .stdout(predicate::str::contains("lint passed for"));
    Ok(())
}

#[test]
fn unused_parameter_warns_with_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let file = dir.path().join("unused_param.cl");
    fs::write(
        &file,
        r#"
namespace Lint;

public int Helper(int unused, int value) { return value; }

public int Main() { return Helper(1, 2); }
"#,
    )?;

    cargo_bin_cmd!("chic")
        .current_dir(dir.path())
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["lint", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("unused_param"))
        .stdout(predicate::str::contains("parameter `unused` is never used"))
        .stdout(predicate::str::contains("prefix `unused` with `_`"));
    Ok(())
}
