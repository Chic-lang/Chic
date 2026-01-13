use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::write_source;

fn run_chic(source: &str, name: &str) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    write_source(&path, source);
    cargo_bin_cmd!("chic")
        .arg("run")
        .arg(path.to_str().unwrap())
        .args(["--backend", "llvm"])
        // Force stdlib loading even if another test set CHIC_SKIP_STDLIB=1.
        .env("CHIC_SKIP_STDLIB", "0")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().or(predicate::str::contains(
            "warning: overriding the module target triple",
        )));
}

#[test]
fn environment_variables_and_process_info() {
    let enable_stdlib = std::env::var("CHIC_ENABLE_STDLIB_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !enable_stdlib {
        eprintln!(
            "skipping environment test: set CHIC_ENABLE_STDLIB_TESTS=1 to enable stdlib-dependent assertions"
        );
        return;
    }

    let program = r#"
namespace EnvironmentTests;

using Std;
import Std.Collections;

public class Program
{
    public int Main()
    {
        if (Environment.ProcessId() <= 0)
        {
            return 1;
        }

        if (Environment.NewLine() != "\n")
        {
            return 2;
        }

        string cwd = Environment.WorkingDirectory();
        if (cwd == null || cwd == "")
        {
            return 3;
        }

        if (!Environment.SetEnvironmentVariable("CHIC_ENV_TEST", "ok"))
        {
            return 4;
        }
        string value = Environment.GetEnvironmentVariable("CHIC_ENV_TEST");
        if (value != "ok")
        {
            return 5;
        }
        if (!Environment.RemoveEnvironmentVariable("CHIC_ENV_TEST"))
        {
            return 6;
        }

        VecPtr env = Environment.EnumerateEnvironment();
        if (Vec.IsEmpty(in env))
        {
            return 7;
        }

        VecPtr args = Environment.CommandLine();
        // command line always has at least program name
        if (Vec.IsEmpty(in args))
        {
            return 8;
        }

        return 0;
    }
}
"#;

    run_chic(program, "environment_vars.cl");
}
