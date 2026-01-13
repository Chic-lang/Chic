use super::*;

#[test]
fn deinit_destructor_hook_is_forbidden_with_fixit() {
    let source = r"
public struct Widget
{
    public void deinit(ref this) { }
}
";
    let diagnostics = parse_fail(source);
    let diag = diagnostics
        .iter()
        .find(|diag| {
            diag.code
                .as_ref()
                .is_some_and(|code| code.code == "DISPOSE0001")
        })
        .expect("expected DISPOSE0001");

    assert!(
        diag.message.contains("forbidden") && diag.message.contains("dispose"),
        "unexpected message: {diag:?}"
    );

    let label = diag.primary_label.as_ref().expect("expected primary label");
    assert_eq!(
        &source[label.span.start..label.span.end],
        "deinit",
        "expected primary span to cover `deinit`"
    );
    assert!(
        diag.suggestions
            .iter()
            .any(|s| s.replacement.as_deref() == Some("dispose")),
        "expected a replacement suggestion to `dispose`, got {diag:?}"
    );
}

#[test]
fn dispose_rejected_with_wrong_signature() {
    let source = r"
public struct Widget
{
    public void dispose() { }
}
";
    let diagnostics = parse_fail(source);
    let diag = diagnostics
        .iter()
        .find(|diag| {
            diag.code
                .as_ref()
                .is_some_and(|code| code.code == "DISPOSE0002")
        })
        .expect("expected DISPOSE0002");

    let label = diag.primary_label.as_ref().expect("expected primary label");
    assert_eq!(
        &source[label.span.start..label.span.end],
        "dispose",
        "expected primary span to cover `dispose`"
    );
    assert!(
        !diag.suggestions.is_empty(),
        "expected at least one suggestion, got {diag:?}"
    );
}
