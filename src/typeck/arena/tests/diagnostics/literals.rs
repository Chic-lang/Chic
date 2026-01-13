use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "literal_suffix_mismatch_reports_error",
        r#"
namespace Demo;

public class Sample
{
    public int Value = 1u8;
}
"#,
        Expectation::contains(&["[TCK120]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "literal_suffix_overflow_reports_error",
        r#"
namespace Demo;

public class Sample
{
    public byte Value = 300u8;
}
"#,
        Expectation::contains(&["[TCK121]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "default_literal_without_type_reports_error",
        r#"
namespace Demo;

public class Sample
{
    public void Use()
    {
        let value = default;
    }
}
"#,
        Expectation::contains(&["[TCK240]"]),
    ),
    ArenaDiagnosticCase::parsed(
        "default_literal_rejected_for_non_nullable_reference",
        r#"
namespace Demo;

public class Sample
{
    public void Use()
    {
        let text: string = default;
    }
}
"#,
        Expectation::contains(&["[TCK241]"]),
    ),
];

#[test]
fn literal_diagnostics() {
    run_cases("literals", CASES);
}
