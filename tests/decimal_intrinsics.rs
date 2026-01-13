use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::error::Error;

mod common;
use common::write_source;

#[test]
#[ignore = "stdlib region telemetry identifiers (Profile/Generation) currently fail codegen; tracked separately"]
fn decimal_intrinsics_execute_via_runtime_wrappers() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let main_src = dir.path().join("decimal_intrinsics_exec.cl");

    write_source(
        &main_src,
        r#"
import Std.Numeric.Decimal;

namespace DecimalExec;

@vectorize(decimal)
public int Main()
{
    let lhs = 1.25m;
    let rhs = 2.75m;

    Std.Numeric.Decimal.Intrinsics.Add(lhs, rhs);
    Std.Numeric.Decimal.Intrinsics.AddVectorized(lhs, rhs);

    Std.Numeric.Decimal.Intrinsics.DivWithOptions(
        rhs,
        lhs,
        Std.Numeric.Decimal.DecimalRoundingMode.TiesToEven,
        Std.Numeric.Decimal.DecimalVectorizeHint.None
    );

    Std.Numeric.Decimal.Intrinsics.DivWithOptions(
        rhs,
        lhs,
        Std.Numeric.Decimal.DecimalRoundingMode.TiesToEven,
        Std.Numeric.Decimal.DecimalVectorizeHint.Decimal
    );

    Std.Numeric.Decimal.Intrinsics.Fma(lhs, rhs, 1.0000m);
    Std.Numeric.Decimal.Intrinsics.FmaWithOptions(
        lhs,
        rhs,
        1.0000m,
        Std.Numeric.Decimal.DecimalRoundingMode.TowardZero,
        Std.Numeric.Decimal.DecimalVectorizeHint.Decimal
    );

    Std.Numeric.Decimal.Intrinsics.Div(lhs, 0m);

    return 0;
}
"#,
    );

    for backend in ["llvm", "wasm"] {
        cargo_bin_cmd!("chic")
            .env(
                "CHIC_ASYNC_STDLIB_OVERRIDE",
                "tests/testdate/stdlib_async_stub.cl",
            )
            .arg("run")
            .arg(&main_src)
            .args(["--backend", backend])
            // Keep stderr deterministic for assertions; pipeline info logs are opt-in via CHIC_LOG_LEVEL.
            .env("CHIC_LOG_LEVEL", "error")
            .assert()
            .success()
            // Some pipelines surface non-fatal type diagnostics on stdout; permit them as long as
            // the run succeeds.
            .stdout(predicate::str::is_empty().or(predicate::str::starts_with("type diagnostics:")))
            .stderr(predicate::str::is_empty().or(predicate::str::contains(
                "warning: overriding the module target triple",
            )));
    }

    Ok(())
}
