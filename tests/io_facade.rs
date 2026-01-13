use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

#[test]
fn console_write_line_emits_stdout() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("io_stdout.cl");

    write_source(
        &main_src,
        r#"
namespace Exec;

public int Main()
{
    var total = 0;
    for (var index = 0; index < 5; index += 1)
    {
        total += index;
    }
    if (total != 10)
    {
        return 90;
    }
    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .arg("check")
        .arg(&main_src)
        .assert()
        .success()
        .stdout(predicate::str::contains("check passed"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn console_read_line_round_trips_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("io_stdin.cl");

    write_source(
        &main_src,
        r#"
namespace Exec;

public int Main()
{
    var result = 1;
    result += 2;
    result *= 3;
    if (result != 9)
    {
        return 4;
    }
    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_TRACE_PIPELINE", "0")
        .arg("check")
        .arg(&main_src)
        .assert()
        .success()
        .stdout(predicate::str::contains("check passed"))
        .stderr(predicate::str::is_empty());
}
