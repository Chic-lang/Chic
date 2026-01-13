use super::common::RequireExt;
use super::*;

#[test]
fn vectorize_hint_marks_mir_body() {
    let source = r#"
@vectorize(decimal)
public decimal Dot(decimal lhs, decimal rhs)
{
    return lhs;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("DM0001")),
        "expected DM0001 diagnostic, found {:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("Dot"))
        .expect("missing Dot function");
    assert!(
        function.body.vectorize_decimal,
        "expected MIR body to capture decimal vectorization hint"
    );
}
