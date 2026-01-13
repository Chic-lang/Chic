use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::parser::parse_module;
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "di_reports_missing_service_registration",
        r#"
@service
public class Consumer
{
    @inject
    public init(Dependency dep) { }
}

public class Dependency { }
"#,
        Expectation::contains(&["DI0001", "SPEC.md#dependency-injection-attributes"]),
    ),
    ArenaDiagnosticCase::custom(
        "doc_traits_sample_type_checks",
        doc_traits_sample_type_checks,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::parsed(
        "di_detects_singleton_scope_conflict",
        r#"
@service(lifetime: Singleton)
public class Root
{
    @inject
    public init(Worker worker) { }
}

@service(lifetime: Scoped)
public class Worker { }
"#,
        Expectation::contains(&["DI0002"]),
    ),
];

fn doc_traits_sample_type_checks(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let source = r#"
namespace Demo;

public interface Formatter
{
    int Render(ref this);
}

public struct Counter : Formatter
{
    int Render(ref this)
    {
        return 5;
    }
}

public static class Reports
{
    public static int ToValue<TFormatter>(ref TFormatter formatter)
        where TFormatter : Formatter
    {
        return formatter.Render();
    }
}

public int chic_main()
{
    var value = new Counter();
    let result = Reports.ToValue(ref value);
    if (result == 5)
    {
        return 0;
    }
    return 1;
}
"#;
    let parsed = parse_module(source).expect("parse doc sample");
    fixture.check_module(&parsed.module, &[], &TypeLayoutTable::default())
}

#[test]
fn di_diagnostics() {
    run_cases("di", CASES);
}
