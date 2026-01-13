use super::*;

#[test]
fn parses_simple_type_alias() {
    let source = r#"
public typealias AudioSample = UInt16;
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let alias = match &parse.module.items[0] {
        Item::TypeAlias(alias) => alias,
        other => panic!("expected type alias, found {other:?}"),
    };
    assert_eq!(alias.name, "AudioSample");
    assert_eq!(alias.target.name, "UInt16");
}

#[test]
fn rejects_async_type_alias() {
    let source = r#"
async typealias Later = int;
"#;

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`async`") && diag.message.contains("type alias")),
        "expected async type alias diagnostic, found {diagnostics:?}"
    );
}
