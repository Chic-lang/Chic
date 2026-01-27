use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

#[test]
fn span_stackalloc_uses_typed_handles() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("span_stackalloc.ch");

    write_source(
        &main_src,
        r#"
import Std.Memory;
import Std.Span;

namespace SpanStackalloc;

public int Main()
{
    let len = (usize)4;
    var buffer = Std.Memory.StackAlloc.Buffer<int>(len);
    var span = Span<int>.FromValuePointer(buffer, len);
    var readonlySpan = span.AsReadOnly();
    if (span.Length != len) { return 1; }
    if (readonlySpan.Length != len) { return 2; }
    return 0;
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}
