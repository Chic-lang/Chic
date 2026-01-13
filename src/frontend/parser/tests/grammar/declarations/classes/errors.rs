use super::*;

#[test]
fn parses_error_declaration_with_base() {
    let source = r"
public error TransientError : Exception
{
    public string Message;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let error_type = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected error declaration, found {other:?}"),
    };
    assert_eq!(error_type.kind, ClassKind::Error);
    assert_eq!(error_type.name, "TransientError");
    assert_eq!(error_type.bases.len(), 1, "expected explicit base recorded");
    assert_eq!(error_type.bases[0].name, "Exception");
    assert!(matches!(error_type.members[0], ClassMember::Field(_)));
}

#[test]
fn parses_error_declaration_without_explicit_base() {
    let source = r"
public error FatalError
{
    public string Reason;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let error_type = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected error declaration, found {other:?}"),
    };
    assert_eq!(error_type.kind, ClassKind::Error);
    assert!(
        error_type.bases.is_empty(),
        "error without base should not record parents"
    );
}
