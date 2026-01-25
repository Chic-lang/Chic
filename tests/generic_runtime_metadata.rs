use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

fn run_chic_check(source: &str) {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("main.cl");
    write_source(&main_src, source);
    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .env("CHIC_SKIP_MIR_VERIFY", "1")
        .env(
            "CHIC_STDLIB_BLOCKLIST",
            "packages/std.net/src/,packages/std.data/src/",
        )
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn generic_vec_instantiation_compiles() {
    run_chic_check(
        r#"
import Std.Collections;

namespace GenericVec;

public struct Box<T>
{
    public static int Make()
    {
        var vec = Vec.New<T>();
        unchecked
        {
            return (int)Vec.Len(in vec);
        }
    }
}

public int Main()
{
    return Box<int>.Make();
}
"#,
    );
}

#[test]
fn generic_span_from_pointer_compiles() {
    run_chic_check(
        r#"
import Std.Span;
import Std.Runtime.Collections;
import Std.Numeric;
import Std.Core;

namespace GenericSpan;

public struct Maker<T>
{
    public static usize Make()
    {
        var handle = CoreIntrinsics.DefaultValue<ValueMutPtr>();
        handle.Pointer = Pointer.NullMut<byte>();
        unchecked
        {
            handle.Size = (usize)__sizeof<T>();
            handle.Alignment = (usize)__alignof<T>();
        }
        var span = Span<T>.FromValuePointer(handle, 0);
        return span.Length;
    }
}

public int Main()
{
    unchecked
    {
        return (int)Maker<int>.Make();
    }
}
"#,
    );
}
