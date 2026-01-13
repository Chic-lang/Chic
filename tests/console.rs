use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use tempfile::tempdir;

mod common;
use common::write_source;

fn stdlib_enabled() -> bool {
    env::var("CHIC_ENABLE_STDLIB_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn run_console(
    source: &str,
    name: &str,
    backend: &str,
    stdin: Option<&str>,
) -> assert_cmd::assert::Assert {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join(name);
    write_source(&path, source);

    let mut cmd = Command::cargo_bin("chic").expect("binary");
    cmd.arg("run")
        .arg(path.to_str().unwrap())
        .args(["--backend", backend])
        .env("CHIC_SKIP_STDLIB", "0");
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    cmd.assert()
}

#[test]
fn console_newline_override() {
    if !stdlib_enabled() {
        eprintln!("skipping console_newline_override (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)");
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;

public class Program
{
    public int Main()
    {
        Console.NewLine = "\r\n";
        Console.Write("a");
        Console.WriteLine("b");
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_console(program, "console_newline.cl", backend, None)
            .success()
            .stdout(predicate::eq("ab\r\n"))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn console_readline_returns_null_on_eof() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping console_readline_returns_null_on_eof (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;

public class Program
{
    public int Main()
    {
        var line = Console.ReadLine();
        if (line != null)
        {
            return 1;
        }
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_console(program, "console_readline_eof.cl", backend, Some(""))
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn console_redirection_succeeds() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping console_redirection_succeeds (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;

public class Program
{
    public int Main()
    {
        var original = Console.Out;
        var sink = new StringWriter();
        Console.SetOut(sink);
        Console.Write("hello");
        Console.WriteLine(" world");
        Console.SetOut(original);
        var captured = sink.ToString();
        Console.WriteLine(captured);

        var source = new StringReader("next-line\n");
        Console.SetIn(source);
        var read = Console.ReadLine();
        if (read != "next-line")
        {
            return 2;
        }
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_console(program, "console_redirection.cl", backend, None)
            .success()
            .stdout(predicate::eq("hello world\n\n"))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn console_formatting_and_escapes() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping console_formatting_and_escapes (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;

public class Program
{
    public int Main()
    {
        var args = new object[2];
        args[0] = "fmt";
        args[1] = 5;
        Console.WriteLine("{0} {1}", args);
        Console.WriteLine("{{braces}}");
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_console(program, "console_formatting.cl", backend, None)
            .success()
            .stdout(predicate::eq("fmt 5\n{braces}\n"))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn console_capabilities_throw_when_redirected() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping console_capabilities_throw_when_redirected (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;

public class Program
{
    public int Main()
    {
        var redirected = Console.IsOutputRedirected;
        try
        {
            var _ = Console.ForegroundColor;
            if (redirected)
            {
                return 1;
            }
        }
        catch (NotSupportedException)
        {
            if (!redirected)
            {
                return 2;
            }
        }

        try
        {
            Console.Clear();
            if (redirected)
            {
                return 3;
            }
        }
        catch (NotSupportedException)
        {
            if (!redirected)
            {
                return 4;
            }
        }
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_console(program, "console_caps.cl", backend, None)
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn console_thread_safe_write_lines() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping console_thread_safe_write_lines (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace ConsoleTests;

using Std;
import Std.Platform.Thread;

public class Program
{
    private static void WriteLinesA()
    {
        var args = new object[2];
        args[0] = "A";
        for (var i = 0; i < 20; i += 1)
        {
            args[1] = i;
            Console.WriteLine("{0}{1}", args);
        }
    }

    private static void WriteLinesB()
    {
        var args = new object[2];
        args[0] = "B";
        for (var i = 0; i < 20; i += 1)
        {
            args[1] = i;
            Console.WriteLine("{0}{1}", args);
        }
    }

    public int Main()
    {
        var original = Console.Out;
        var sink = new StringWriter();
        Console.SetOut(sink);

        var t1 = Thread.Spawn(ThreadStartFactory.Function(WriteLinesA));
        var t2 = Thread.Spawn(ThreadStartFactory.Function(WriteLinesB));

        if (t1.Join() != ThreadStatus.Success)
        {
            return 10;
        }
        if (t2.Join() != ThreadStatus.Success)
        {
            return 11;
        }

        Console.SetOut(original);
        Console.WriteLine(sink.ToString());
        return 0;
    }
}
"#;

    let assert = run_console(program, "console_threads.cl", "llvm", None).success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let lines: Vec<&str> = stdout.split('\n').filter(|line| !line.is_empty()).collect();
    assert_eq!(40, lines.len(), "expected 40 lines from two writers");
    for line in lines {
        let (prefix, digits) = line.split_at(1);
        assert!(
            prefix == "A" || prefix == "B",
            "unexpected prefix in line `{}`",
            line
        );
        assert!(
            digits.chars().all(|ch| ch.is_ascii_digit()),
            "line `{}` should only contain digits after prefix",
            line
        );
    }
}
