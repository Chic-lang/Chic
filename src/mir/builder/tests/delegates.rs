use super::common::RequireExt;
use super::*;

#[test]
fn incompatible_delegate_conversion_emits_diagnostic() {
    let source = r#"
namespace Sample;

public delegate int Producer();
public delegate int Consumer(int value);

public int Demo(Producer producer)
{
    let bad = (Consumer)producer;
    return bad(1);
}
"#;

    let parsed = parse_module(source).require("parse delegate module");
    let lowering = lower_module(&parsed.module);

    let message = lowering
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("cannot convert `Sample::Producer` to delegate `Sample::Consumer`")
        })
        .map(|diag| diag.message.clone());
    assert!(
        message.is_some(),
        "expected delegate conversion diagnostic, got: {:#?}",
        lowering.diagnostics
    );
}
