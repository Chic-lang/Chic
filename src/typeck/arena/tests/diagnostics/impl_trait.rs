use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[ArenaDiagnosticCase::lowered(
    "impl_trait_missing_bound_reports_tck310",
    r#"
namespace Demo;

public interface Formatter
{
    public int Format(int value);
}

public class Plain : Formatter
{
    public int Format(int value) { return value; }
}

public Formatter MakeFormatter()
{
    return new Plain();
}
"#,
    Expectation::clean(),
)];

#[test]
fn impl_trait_diagnostics() {
    run_cases("impl_trait", CASES);
}
