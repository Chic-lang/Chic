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

fn run_program(
    source: &str,
    name: &str,
    backend: &str,
    extra_args: &[&str],
) -> assert_cmd::assert::Assert {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join(name);
    write_source(&path, source);

    let mut cmd = Command::cargo_bin("chic").expect("binary");
    cmd.arg("run")
        .arg(path.to_str().unwrap())
        .args(["--backend", backend])
        .args(extra_args)
        .env("CHIC_SKIP_STDLIB", "0");
    cmd.assert()
}

#[test]
fn diagnostics_indentation_and_categories() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping diagnostics_indentation_and_categories (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace DiagnosticsTests;

using Std;
import Std.Diagnostics;

public class RecordingListener : TraceListener
{
    public string Buffer;

    public init()
    {
        Buffer = Std.Runtime.StringRuntime.Create();
    }

    public override void Write(string? message)
    {
        if (message == null)
        {
            return;
        }
        Buffer = Buffer + message;
    }

    public override void WriteLine(string? message)
    {
        if (message != null)
        {
            Buffer = Buffer + message;
        }
        Buffer = Buffer + "\n";
    }
}

public class Program
{
    public int Main()
    {
        Trace.Listeners.Clear();
        var listener = new RecordingListener();
        Trace.Listeners.Add(listener);
        Trace.IndentSize = 2;
        Trace.Indent();
        Trace.WriteLine("first");
        Trace.WriteLine("second", "cat");
        Trace.Unindent();
        Trace.Write("tail");
        Debug.WriteLine("debug-line");
        Console.Write(listener.Buffer);
        return 0;
    }
}
"#;

    let expected = "  first\n  [cat] second\ntaildebug-line\n";
    for backend in ["llvm", "wasm"] {
        run_program(program, "diagnostics_indent.cl", backend, &[])
            .success()
            .stdout(predicate::eq(expected))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn diagnostics_autoflush_counts() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping diagnostics_autoflush_counts (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace DiagnosticsTests;

using Std;
import Std.Diagnostics;

public class CountingListener : TraceListener
{
    public int Flushes;

    public override void Flush()
    {
        Flushes += 1;
    }
}

public class Program
{
    public int Main()
    {
        Trace.Listeners.Clear();
        var listener = new CountingListener();
        Trace.Listeners.Add(listener);
        Trace.AutoFlush = true;
        Trace.Write("a");
        Trace.WriteLine("b");
        Console.Write(listener.Flushes);
        return 0;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_program(program, "diagnostics_autoflush.cl", backend, &[])
            .success()
            .stdout(predicate::eq("2"))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn diagnostics_asserts_throw_and_format() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping diagnostics_asserts_throw_and_format (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace DiagnosticsTests;

using Std;
import Std.Diagnostics;

public class Program
{
    public int Main()
    {
        Trace.Listeners.Clear();
        try
        {
            Debug.Assert(false, "boom", "detail");
        }
        catch (AssertFailedException ex)
        {
            if (ex.DetailMessage == null || !ex.DetailMessage.Contains("detail"))
            {
                return 3;
            }
            var message = ex.Message;
            Console.WriteLine(message);
            if (!message.Contains("Assertion failed"))
            {
                return 1;
            }
            if (!message.Contains("detail"))
            {
                return 2;
            }
            if (!message.Contains("stack trace unavailable"))
            {
                return 4;
            }
            return 0;
        }
        return 5;
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_program(program, "diagnostics_asserts.cl", backend, &[])
            .success()
            .stdout(predicate::str::contains("boom").and(predicate::str::contains("detail")))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }
}

#[test]
fn conditional_calls_skip_side_effects() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping conditional_calls_skip_side_effects (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let debug_program = r#"
namespace DiagnosticsTests;

using Std;
import Std.Diagnostics;

public static class Counter
{
    private static int Value;

    public static int Tick()
    {
        Value += 1;
        return Value;
    }

    public static int Current() => Value;
}

public class Program
{
    public int Main()
    {
        Debug.WriteLine(Counter.Tick());
        return Counter.Current();
    }
}
"#;

    let trace_program = r#"
namespace DiagnosticsTests;

using Std;
import Std.Diagnostics;

public static class Counter
{
    private static int Value;

    public static int Tick()
    {
        Value += 1;
        return Value;
    }

    public static int Current() => Value;
}

public class Program
{
    public int Main()
    {
        Trace.WriteLine(Counter.Tick());
        return Counter.Current();
    }
}
"#;

    for backend in ["llvm", "wasm"] {
        run_program(
            debug_program,
            "conditional_debug.cl",
            backend,
            &["--define", "DEBUG=false"],
        )
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().or(predicate::str::contains(
            "warning: overriding the module target triple",
        )));
        run_program(
            trace_program,
            "conditional_trace.cl",
            backend,
            &["--define", "TRACE=false"],
        )
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty().or(predicate::str::contains(
            "warning: overriding the module target triple",
        )));
    }
}

#[test]
fn conditional_attribute_rejects_non_void() {
    if !stdlib_enabled() {
        eprintln!(
            "skipping conditional_attribute_rejects_non_void (set CHIC_ENABLE_STDLIB_TESTS=1 to enable)"
        );
        return;
    }

    let program = r#"
namespace DiagnosticsTests;

import Std.Diagnostics;

public class Program
{
    @conditional("DEBUG")
    public int BadConditional()
    {
        return 1;
    }

    public int Main()
    {
        return BadConditional();
    }
}
"#;

    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("conditional_invalid.cl");
    write_source(&path, program);

    let mut cmd = Command::cargo_bin("chic").expect("binary");
    cmd.arg("check")
        .arg(path.to_str().unwrap())
        .env("CHIC_SKIP_STDLIB", "0");
    cmd.assert().failure().stderr(predicate::str::contains(
        "[MIRL0330] `@conditional` requires a void return type",
    ));
}
