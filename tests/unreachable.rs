use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn run_check(source: &str, skip_stdlib: bool) -> assert_cmd::assert::Assert {
    let mut file = NamedTempFile::new().expect("temp file");
    file.write_all(source.as_bytes()).expect("write source");
    file.flush().expect("flush source");

    let mut cmd = cargo_bin_cmd!("chic");
    if skip_stdlib {
        cmd.env("CHIC_SKIP_STDLIB", "1");
    }
    cmd.env("CHIC_DIAGNOSTICS_FATAL", "1")
        .env("NO_COLOR", "1")
        .args([
            "check",
            file.path().to_str().unwrap(),
            "--error-format",
            "human",
            "--log-level",
            "error",
        ])
        .assert()
}

#[test]
fn unreachable_after_return_is_reported() {
    let src = r#"
namespace Demo;

public int Bad()
{
    return 1;
    var x = 2;
}

public int Main() => Bad();
"#;

    run_check(src, true)
        .failure()
        .stderr(predicate::str::contains("error[E0400]: unreachable code"))
        .stderr(predicate::str::contains("return 1;"))
        .stderr(predicate::str::contains("var x = 2;"))
        .stderr(predicate::str::contains("^ unreachable code"));
}

#[test]
fn constant_false_branch_is_unreachable() {
    let src = r#"
namespace Demo;

public int Main()
{
    if (false)
    {
        var y = 3;
    }
    return 0;
}
"#;

    run_check(src, true)
        .failure()
        .stdout(predicate::str::contains("warning[E0400]: unreachable code"))
        .stdout(predicate::str::contains(
            "the condition is always false at compile time",
        ));
}

#[test]
fn while_false_body_is_unreachable() {
    let src = r#"
namespace Demo;

public int Main()
{
    while (false)
    {
        var count = 1;
    }
    return 0;
}
"#;

    run_check(src, true)
        .failure()
        .stdout(predicate::str::contains("warning[E0400]: unreachable code"))
        .stdout(predicate::str::contains(
            "the condition is always false at compile time",
        ));
}

#[test]
fn constant_switch_marks_other_cases_unreachable() {
    let src = r#"
namespace Demo;

public int Main()
{
    switch (1)
    {
        case 0:
            return 0;
        case 1:
            return 2;
        default:
            return 3;
    }
}
"#;

    run_check(src, true)
        .failure()
        .stdout(predicate::str::contains("warning[E0400]: unreachable code"))
        .stdout(predicate::str::contains("condition is always false"))
        .stdout(predicate::str::contains("condition is always true"));
}

#[test]
fn code_after_throw_is_unreachable() {
    let src = r#"
@allow(dead_code)
namespace Demo;

public class Exception { @allow(dead_code) public init() {} }

public class Boom : Exception
{
    public init() {}
}

public int Thrower() throws Boom
{
    throw new Boom();
    var after = 1;
    return after;
}

public int Main() throws Boom
{
    return Thrower();
}
"#;

    run_check(src, true)
        .failure()
        .stderr(predicate::str::contains("error[E0400]: unreachable code"))
        .stderr(predicate::str::contains("throw new Boom();"))
        .stderr(predicate::str::contains("control flow always exits here"));
}
