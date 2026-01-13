use super::*;
use crate::mir::{LoweringResult, lower_module};

fn lower_source(source: &str) -> LoweringResult {
    let parsed = parse_module(source).expect("module should parse");
    lower_module(&parsed.module)
}

#[test]
fn collects_testcase_metadata_with_attributes() {
    let source = r#"
namespace Suite;

@category(smoke)
@tag(integration)
@id(custom-id)
testcase Sample(int count, string name = "demo")
{
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let cases = lowering.module.test_cases;
    assert_eq!(cases.len(), 1, "expected one testcase");
    let meta = &cases[0];
    assert_eq!(meta.id, "custom-id");
    assert_eq!(meta.name, "Sample");
    assert_eq!(meta.namespace.as_deref(), Some("Suite"));
    assert_eq!(meta.categories, vec!["integration", "smoke"]);
    assert_eq!(meta.parameters.len(), 2);
    assert_eq!(meta.parameters[0].name, "count");
    assert_eq!(meta.parameters[0].ty.as_deref(), Some("int"));
    assert!(!meta.parameters[0].has_default);
    assert_eq!(meta.parameters[1].name, "name");
    assert_eq!(meta.parameters[1].ty.as_deref(), Some("string"));
    assert!(meta.parameters[1].has_default);
}

#[test]
fn reports_unknown_testcase_attributes() {
    let source = r#"
@unknown_attr
testcase UsesUnknownAttribute()
{
}
"#;

    let lowering = lower_source(source);
    let messages: Vec<_> = lowering
        .diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("not supported on testcase declarations")),
        "expected testcase attribute diagnostic, got {messages:?}"
    );
}
