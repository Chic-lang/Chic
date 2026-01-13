use super::*;

#[test]
fn trait_keyword_is_rejected() {
    let source = r"
public trait Sample
{
    void Run();
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`trait` is no longer supported")),
        "expected trait removal diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn impl_keyword_is_rejected() {
    let source = r"
public impl Sample
{
    void Run();
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`impl` is no longer supported")),
        "expected impl removal diagnostic, found {:?}",
        diagnostics
    );
}
