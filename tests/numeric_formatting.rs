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
fn numeric_formatting_supports_two_string_api_and_cultures()
-> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping numeric formatting exec test because CHIC_ENABLE_CODEGEN_EXEC is not set"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping numeric formatting exec test because clang is not available");
        return Ok(());
    }

    let source = r#"
    namespace NumericFormatting;

import Std;
import Std.Numeric;

public static class Helpers
{
    public static string FormatNumber<T>(T value, string format, string culture)
        where T : INumber<T>, IFormattable
    {
        return value.Format(format, culture);
    }
}

public class Program
{
    public int Main()
    {
        if (Int32.From(1234).Format(null, null) != "1234")
        {
            return 1;
        }
        if (Int32.From(42).Format("D5", "invariant") != "00042")
        {
            return 2;
        }
        if (UInt32.From(255u).Format("X", "invariant") != "FF")
        {
            return 3;
        }
        if (Int32.From(1234567).Format("N", "invariant") != "1,234,567")
        {
            return 4;
        }
        if (new Float64(1234.56d).Format("N", "invariant") != "1,234.56")
        {
            return 5;
        }
        if (new Float64(1234.56d).Format("N", "fr-FR") != "1 234,56")
        {
            return 6;
        }
        if (1234.56m.Format("N", "fr-FR") != "1 234,56")
        {
            return 7;
        }
        if (Helpers.FormatNumber<Int32>(Int32.From(99), "D3", "en-US") != "099")
        {
            return 8;
        }
        if (Helpers.FormatNumber<Float64>(new Float64(1.25d), "F2", "en-US") != "1.25")
        {
            return 9;
        }
        let exp = new Float64(12.5d).Format("E3", "invariant");
        if (exp != "1.250E+001")
        {
            return 10;
        }
        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let path = dir.path().join("numeric_formatting.cl");
    write_source(&path, source);

    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(path.to_str().unwrap())
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["--backend", "llvm"])
        .assert()
        .success();

    Ok(())
}

#[test]
fn numeric_formatting_reports_invalid_input() -> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping numeric formatting exec test because CHIC_ENABLE_CODEGEN_EXEC is not set"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping numeric formatting exec test because clang is not available");
        return Ok(());
    }

    let invalid_format = r#"
namespace NumericFormatting;

import Std;
import Std.Numeric;

public class Program
{
    public int Main()
    {
        let ignored = Int32.From(42).Format("Z", "invariant");
        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let format_path = dir.path().join("numeric_format_error.cl");
    write_source(&format_path, invalid_format);

    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(format_path.to_str().unwrap())
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["--backend", "llvm"])
        .assert()
        .failure()
        .stderr(contains("FormatException"));

    let invalid_culture = r#"
namespace NumericFormatting;

import Std;
import Std.Numeric;

public class Program
{
    public int Main()
    {
        let ignored = new Float64(1.0d).Format("G", "zz-ZZ");
        return 0;
    }
}
"#;

    let culture_path = dir.path().join("numeric_culture_error.cl");
    write_source(&culture_path, invalid_culture);

    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(culture_path.to_str().unwrap())
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["--backend", "llvm"])
        .assert()
        .failure()
        .stderr(contains("Unsupported culture"));

    Ok(())
}

#[test]
fn numeric_formatting_covers_all_numeric_types_and_span_overloads()
-> Result<(), Box<dyn std::error::Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping numeric formatting exec test because CHIC_ENABLE_CODEGEN_EXEC is not set"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping numeric formatting exec test because clang is not available");
        return Ok(());
    }

    let source = r#"
namespace NumericFormattingAllTypes;

import Std;
import Std.Numeric;
import Std.Span;
import Std.Memory;
import Std.Strings;

public class Program
{
    public int Main()
    {
        if (SByte.From(-12i8).Format("D4", "invariant") != "-0012") return 1;
        if (Byte.From(10u8).Format("X4", "invariant") != "000A") return 2;

        let int16Val = Int16.From(12345i16);
        if (int16Val.Format("N", "invariant") != "12,345") return 3;
        if (int16Val.ToString("N") != "12,345") return 4;

        let ushortVal = UInt16.From(48879u16);
        if (ushortVal.Format("X4", "invariant") != "BEEF") return 5;

        let int32Val = Int32.From(1234);
        if (int32Val.Format("N", "") != "1,234") return 6;

        if (UInt32.From(0xBEEFu).Format("x", "invariant") != "beef") return 7;

        let int64Val = Int64.From(-42L);
        if (int64Val.Format("D20", "invariant") != "-00000000000000000042") return 8;

        if (UInt64.From(12345678901234ul).Format("N", "invariant") != "12,345,678,901,234") return 9;

        if (Int128.From(123456i128).Format("N2", "fr-FR") != "123 456,00") return 10;

        if (UInt128.From(18446744073709551615u128).Format("D", "invariant") != "18446744073709551615") return 11;

        if (IntPtr.From((nint)0x2A).Format("X", "invariant") != "2A") return 12;

        if (UIntPtr.From((nuint)123456u).Format("N", "fr-FR") != "123 456") return 13;

        let float32Val = new Float32(12.5f);
        if (float32Val.Format("F3", "invariant") != "12.500") return 14;

        if (new Float64(0.0000123d).Format("G", "invariant") != "1.230000E-005") return 15;

        if (new Float128(0.125d).Format("E2", "invariant") != "1.25E-001") return 16;

        let decimalVal = 1234.5m;
        if (decimalVal.Format("N", "fr-FR") != "1 234,50") return 17;

        var buffer64 = StackAlloc.Span<byte>(64);
        if (!int64Val.TryFormat(buffer64, out var written64, "D20", "invariant")) return 20;
        var int64Text = Utf8String.FromSpan(buffer64.AsReadOnly().Slice(0, written64));
        if (int64Text != "-00000000000000000042") return 21;
        var tiny64 = StackAlloc.Span<byte>(1);
        if (int64Val.TryFormat(tiny64, out var tinyWritten64, "D20", "invariant")) return 22;
        if (tinyWritten64 != 0usize) return 23;

        var floatBuf = StackAlloc.Span<byte>(32);
        if (!float32Val.TryFormat(floatBuf, out var floatWritten, "F3", "invariant")) return 24;
        var floatText = Utf8String.FromSpan(floatBuf.AsReadOnly().Slice(0, floatWritten));
        if (floatText != "12.500") return 25;

        var decimalBuf = StackAlloc.Span<byte>(32);
        if (!decimalVal.TryFormat(decimalBuf, out var decimalWritten, "N", "fr-FR")) return 26;
        var decimalText = Utf8String.FromSpan(decimalBuf.AsReadOnly().Slice(0, decimalWritten));
        if (decimalText != "1 234,50") return 27;

        return 0;
    }
}
"#;

    let dir = tempdir()?;
    let path = dir.path().join("numeric_formatting_all.cl");
    write_source(&path, source);

    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(path.to_str().unwrap())
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("NO_COLOR", "1")
        .args(["--backend", "llvm"])
        .assert()
        .success();

    Ok(())
}
