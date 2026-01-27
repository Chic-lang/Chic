use assert_cmd::cargo::cargo_bin_cmd;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn json_logs_surface_overload_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace OverloadLsp;

public int Add(int x, int y)
{
    return x + y;
}

public int Main()
{
    return Add();
}
"#,
    )?;
    file.flush()?;

    let output = cargo_bin_cmd!("chic")
        .env("NO_COLOR", "1")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .args([
            "check",
            file.path().to_str().unwrap(),
            "--log-format",
            "json",
            "--log-level",
            "info",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "expected check to report overload diagnostics"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no overload of `OverloadLsp::Add` matches")
            || stderr.contains("missing an argument for parameter `x`"),
        "missing overload/missing argument diagnostic in stderr: {stderr}"
    );
    assert!(
        stderr.contains("\"stage\":\"driver.check.complete\""),
        "missing JSON log entry for check completion: {stderr}"
    );

    Ok(())
}
