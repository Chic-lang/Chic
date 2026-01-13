use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[ArenaDiagnosticCase::parsed(
    "duplicate_record_pattern_field_reports_pat0002",
    r#"
namespace Records;

public struct Pair { public int A; public int B; }

public int Demo(Pair pair)
{
    switch (pair)
    {
        case Pair { A: 1, A: 2 }:
            return 0;
        default:
            return 1;
    }
}
"#,
    Expectation::contains(&["[PAT0002]"]),
)];

#[test]
fn pattern_diagnostics() {
    run_cases("patterns", CASES);
}
