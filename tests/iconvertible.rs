use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;

mod common;
use common::write_source;

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().is_ok()
}

#[test]
fn iconvertible_happy_path() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping iconvertible exec test because CHIC_ENABLE_CODEGEN_EXEC is not set");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping iconvertible exec test because clang is not available");
        return Ok(());
    }

    let source = r#"
namespace ConvertibleSuccess;

import Std;
import Std.Globalization;

public class Program
{
    public int Main()
    {
        let inv = new InvariantFormatProvider();

        let big = Int128.Parse("1234567890");
        if (big.ToInt32(inv) != 1234567890)
        {
            return 1;
        }

        let bigUnsigned = UInt128.Parse("9876543210");
        if (bigUnsigned.ToUInt64(inv) != 9876543210ul)
        {
            return 2;
        }

        let quad = new Float128(42.5d);
        if (quad.ToInt32(inv) != 42)
        {
            return 3;
        }

        let largeText = "123456789012345678901234567890";
        let parsed = largeText.ToInt128(inv);
        let roundtrip = parsed.ToString(inv);
        if (roundtrip != largeText)
        {
            return 4;
        }

        let frNumber = "1234,56";
        let fr = new CultureFormatProvider("fr-FR");
        let dec = frNumber.ToDecimal(fr);
        let decText = dec.ToString(fr);
        if (decText != frNumber)
        {
            return 5;
        }

        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let path = dir.path().join("iconvertible_success.cl");
    write_source(&path, source);

	cargo_bin_cmd!("chic")
	    .arg("run")
	    .arg(path.to_str().unwrap())
	    .env("CHIC_SKIP_MIR_VERIFY", "1")
	    .env("CHIC_LOG_LEVEL", "error")
	    .env("CHIC_TRACE_PIPELINE", "0")
	    .env("NO_COLOR", "1")
	    .args(["--backend", "llvm"])
	    .assert()
	    .success();

    Ok(())
}

#[test]
fn iconvertible_overflow_and_format_fail() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping iconvertible exec test because CHIC_ENABLE_CODEGEN_EXEC is not set");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping iconvertible exec test because clang is not available");
        return Ok(());
    }

    let overflow = r#"
namespace ConvertibleOverflow;

import Std;
import Std.Globalization;

public class Program
{
    public int Main()
    {
        let inv = new InvariantFormatProvider();
        let huge = UInt128.Parse("340282366920938463463374607431768211455");
        let _ = huge.ToInt32(inv);
        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let overflow_path = dir.path().join("iconvertible_overflow.cl");
    write_source(&overflow_path, overflow);

	cargo_bin_cmd!("chic")
	    .arg("run")
	    .arg(overflow_path.to_str().unwrap())
	    .env("CHIC_SKIP_MIR_VERIFY", "1")
	    .env("CHIC_LOG_LEVEL", "error")
	    .env("CHIC_TRACE_PIPELINE", "0")
	    .env("NO_COLOR", "1")
	    .args(["--backend", "llvm"])
	    .assert()
	    .failure()
	    .stderr(contains("OverflowException"));

    let format_fail = r#"
	namespace ConvertibleFormat;

	import Std;
	import Std.Globalization;

public class Program
{
    public int Main()
    {
        let inv = new InvariantFormatProvider();
        let bad = "abc";
        let _ = bad.ToInt128(inv);
        return 0;
    }
}
"#;

    let format_path = dir.path().join("iconvertible_format.cl");
    write_source(&format_path, format_fail);

	cargo_bin_cmd!("chic")
	    .arg("run")
	    .arg(format_path.to_str().unwrap())
	    .env("CHIC_SKIP_MIR_VERIFY", "1")
	    .env("CHIC_LOG_LEVEL", "error")
	    .env("CHIC_TRACE_PIPELINE", "0")
	    .env("NO_COLOR", "1")
	    .args(["--backend", "llvm"])
	    .assert()
	    .failure()
	    .stderr(contains("FormatException"));

    let invalid_cast = r#"
	namespace ConvertibleInvalidCast;

	import Std;
	import Std.Globalization;

public class Program
{
    public int Main()
    {
        let inv = new InvariantFormatProvider();
        let flag = true;
        let _ = flag.ToDateTime(inv);
        return 0;
    }
}
"#;

    let cast_path = dir.path().join("iconvertible_cast.cl");
    write_source(&cast_path, invalid_cast);

	cargo_bin_cmd!("chic")
	    .arg("run")
	    .arg(cast_path.to_str().unwrap())
	    .env("CHIC_SKIP_MIR_VERIFY", "1")
	    .env("CHIC_LOG_LEVEL", "error")
	    .env("CHIC_TRACE_PIPELINE", "0")
	    .env("NO_COLOR", "1")
	    .args(["--backend", "llvm"])
	    .assert()
	    .failure()
	    .stderr(contains("InvalidCastException"));

    Ok(())
}
