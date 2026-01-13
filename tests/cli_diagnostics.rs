use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn check_reports_mir_and_borrow_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br"
namespace Demo;

public UnknownType Foo(in int x)
{
    let y = missing + 1;
    x += 5;
    return y;
}
",
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("type diagnostics"))
        .stdout(predicate::str::contains("unknown type `UnknownType`"))
        .stdout(predicate::str::contains("mir lowering diagnostics"))
        .stdout(predicate::str::contains("unknown identifier `missing`"))
        .stdout(predicate::str::contains("borrow checker diagnostics"))
        .stdout(predicate::str::contains(
            "cannot assign to `in` parameter `x`",
        ));

    Ok(())
}

#[test]
fn check_reports_overload_ambiguity() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace CliOverloads;

public class Formatter
{
    public static string Format(int value, string template = "") => template;
    // Avoids stdlib dependencies under CHIC_SKIP_STDLIB while keeping the overload shape.
    public static string Format(int value, int digits = 2) => "";
}

public string Main() => Formatter.Format(5);
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("NO_COLOR", "1")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "call to `Formatter::Format` is ambiguous",
        ));

    Ok(())
}

#[test]
fn mir_dump_outputs_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br"
namespace Demo;

public int Increment(int x)
{
    return x + 1;
}
",
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .args(["mir-dump", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("fn Demo::Increment"))
        .stdout(predicate::str::contains("bb0"));

    Ok(())
}

#[test]
fn check_reports_nullable_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace Demo;

public int UseNullable()
{
    var maybe = (string?)null;
    return maybe.Length;
}

public int AssignNull()
{
    var name = (string)null;
    return name.Length;
}
"#,
    )?;
    file.flush()?;

    let message_ret = "Demo::UseNullable: cannot assign `null` to non-nullable binding";
    let message_name = "Demo::AssignNull: cannot assign `null` to non-nullable binding";

    let mut cmd = cargo_bin_cmd!("chic");
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.args(["check", file.path().to_str().unwrap()]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(message_ret))
        .stdout(predicate::str::contains(message_name));

    Ok(())
}

#[test]
fn closure_to_fn_pointer_reports_capture_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace FnPtr;

public int Main()
{
    var value = 5;
    return InvokeFn(() => value);
}

private static int InvokeFn(fn() -> int callback) { return callback(); }
"#,
    )?;
    file.flush()?;

    // Capturing closures now bridge to fn pointers successfully; the build should succeed.
    cargo_bin_cmd!("chic")
        .arg("build")
        .arg(file.path())
        .args(["--backend", "wasm"])
        .env("CHIC_SKIP_STDLIB", "1")
        .assert()
        .success();

    Ok(())
}

#[test]
fn mutable_ref_requires_unique_borrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace RefDiagnostics;

public ref int Alias(in int value)
{
    return ref value;
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "cannot take a mutable borrow of immutable binding `value`",
        ))
        .stdout(predicate::str::contains("RefDiagnostics::Alias"));

    Ok(())
}

#[test]
fn ref_referent_mismatch_reports_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace RefDiagnostics;

public ref readonly int Wrong(ref string value)
{
    return ref value;
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "referent type `string` does not match `int`",
        ));

    Ok(())
}

#[test]
fn deinit_destructor_hook_emits_fixit_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace Demo;

public struct Widget
{
    public void deinit(ref this) { }
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .env("NO_COLOR", "1")
        .args([
            "check",
            file.path().to_str().unwrap(),
            "--log-level",
            "error",
            "--error-format",
            "human",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[DISPOSE0001]"))
        .stderr(predicate::str::contains(
            "`deinit` is forbidden; use `dispose`",
        ))
        .stderr(predicate::str::contains(
            "help: replace `deinit` with `dispose`",
        ))
        .stderr(predicate::str::contains("replace with `dispose`"));

    Ok(())
}

#[test]
fn invalid_dispose_signature_emits_fixit_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace Demo;

public struct Widget
{
    public void dispose() { }
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_DIAGNOSTICS_FATAL", "1")
        .env("NO_COLOR", "1")
        .args([
            "check",
            file.path().to_str().unwrap(),
            "--log-level",
            "error",
            "--error-format",
            "human",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error[DISPOSE0002]"))
        .stderr(predicate::str::contains(
            "invalid `dispose` signature; expected `dispose(ref this)` returning void",
        ))
        .stderr(predicate::str::contains(
            "help: change this declaration to `dispose(ref this)`",
        ));

    Ok(())
}

#[test]
fn ref_values_can_downgrade_to_readonly() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace RefDiagnostics;

public ref readonly int Downgrade(ref int value)
{
    return ref value;
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .success();

    Ok(())
}

#[test]
fn run_executes_overloaded_program() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(
        br#"
namespace OverloadCli;

public int Scale(int value) { return value * 2; }
public int Scale(int value, int factor) { return value * factor; }
public int Scale(int value, int factor, int offset) { return (value * factor) + offset; }

public int Main()
{
    var doubled = 0;
    doubled = Scale(6);
    var scaled = 0;
    scaled = Scale(3, 4);
    var staticCall = 0;
    staticCall = Scale(2, 5, 3);
    return (doubled == 12 && scaled == 12 && staticCall == 13) ? 0 : 1;
}
"#,
    )?;
    file.flush()?;

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(file.path())
        .args(["--log-format", "json"])
        .env("CHIC_SKIP_STDLIB", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("check passed"))
        .stderr(predicate::str::contains(
            "\"stage\":\"driver.check.complete\"",
        ));

    Ok(())
}
