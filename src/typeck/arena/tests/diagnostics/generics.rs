use super::{ArenaDiagnosticCase, Expectation, run_cases};

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::parsed(
        "new_expression_missing_required_generic_arguments",
        r#"
namespace Demo;

public struct Vec<T>
{
    public init(int capacity) { }
}

public class Usage
{
    public Vec<int> Build()
    {
        return new Vec(capacity: 4);
    }
}
"#,
        Expectation::contains(&["type `Demo::Vec`", "requires 1 type argument"]),
    ),
    ArenaDiagnosticCase::parsed(
        "type_annotation_missing_generic_arguments",
        r#"
namespace Demo;

public struct Vec<T> { }

public struct Holder
{
    public Vec Field;
}
"#,
        Expectation::contains(&["type `Demo::Vec`", "requires 1 type argument"]),
    ),
];

#[test]
fn generic_argument_diagnostics() {
    run_cases("generics", CASES);
}
