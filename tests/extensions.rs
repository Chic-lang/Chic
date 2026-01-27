use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

mod common;
use common::write_source;

fn extensions_enabled() -> bool {
    matches!(
        std::env::var("CHIC_ENABLE_EXTENSIONS"),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}

#[test]
fn extension_methods_bind_and_mutate() {
    if !extensions_enabled() {
        eprintln!("skipping extensions test: CHIC_ENABLE_EXTENSIONS not set");
        return;
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("extension_usage.ch");

    write_source(
        &main_src,
        r#"
import Std.Collections;

namespace Std.Helpers
{
    import Std.Collections;

    public extension VecPtr
    {
        public bool IsEmpty(in this)
        {
            return Vec.Len(in this) == 0;
        }
    }
}

namespace Samples
{
    import Std.Collections;
    import Std.Helpers;

    public struct Holder<T> { }

    public extension<T> Holder<T>
    {
        public int Count(in this)
        {
            return 0;
        }
    }

    public int Main()
    {
        var vec = Vec.New<int>();
        if (!vec.IsEmpty()) { return 1; }

        Holder<int> holder;
        var total = holder.Count();
        if (total != 0) { return 2; }

        return 0;
    }
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_TRACE_PIPELINE", "0")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("check passed for")
                .or(predicate::str::contains("check completed with diagnostics")),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn extension_method_requires_this_receiver() {
    let dir = tempfile::tempdir().expect("temp dir");
    let main_src = dir.path().join("extension_error.ch");

    write_source(
        &main_src,
        r#"
namespace Demo;

public struct Value
{
    public int Data;
}

public extension Value
{
    public void Bad(Value other) { }
}
"#,
    );

    cargo_bin_cmd!("chic")
        .arg("check")
        .arg(&main_src)
        .env("CHIC_SKIP_STDLIB", "1")
        .env("CHIC_TRACE_PIPELINE", "0")
        .env("CHIC_LOG_LEVEL", "error")
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "extension method `Demo::Value::Bad` must declare a leading `this` receiver parameter",
        ));
}
