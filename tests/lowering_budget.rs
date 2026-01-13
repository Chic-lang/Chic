use chic::frontend::parser::parse_module;
use chic::mir::lower_module;

#[test]
fn lowering_budget_fixture_runs() {
    let source = r#"
namespace Budget;

public struct Metrics
{
    public int Value;
}

public class Sample
{
    private readonly Metrics _metrics;

    public init()
    {
        _metrics = new Metrics { Value = 5 };
    }

    public int Compute(int delta)
    {
        return _metrics.Value + delta;
    }
}
"#;
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("budget fixture failed to parse: {:?}", err.diagnostics());
    });
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );
    assert!(
        !lowering.pass_metrics.is_empty(),
        "expected MIR pass metrics to be recorded"
    );
}
