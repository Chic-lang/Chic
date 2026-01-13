use super::super::common::FunctionFixture;
use crate::frontend::ast::StatementKind;
use crate::frontend::parser::tests::fixtures::parse_fail;

#[test]
fn parses_try_with_filter_and_finally() {
    let source = r"
public void Handle()
{
    try
    {
        Work();
    }
catch (Exception err) when (err.ShouldRetry())
    {
        return;
    }
    finally
    {
        Cleanup();
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Try(try_stmt) => {
            assert_eq!(try_stmt.catches.len(), 1);
            let catch = &try_stmt.catches[0];
            assert!(
                catch
                    .type_annotation
                    .as_ref()
                    .is_some_and(|ty| ty.name == "Exception")
            );
            assert!(catch.identifier.as_ref().is_some_and(|name| name == "err"));
            assert!(
                catch
                    .filter
                    .as_ref()
                    .is_some_and(|expr| expr.text.contains("ShouldRetry"))
            );
            assert!(try_stmt.finally.is_some(), "missing finally block");
        }
        other => panic!("expected try statement, found {other:?}"),
    }
}

#[test]
fn parses_try_with_multiple_catches() {
    let source = r"
public void Recover()
{
    try
    {
        Work();
    }
    catch (IOException io)
    {
        return;
    }
    catch
    {
        throw;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Try(try_stmt) => {
            assert_eq!(try_stmt.catches.len(), 2);
            let first = &try_stmt.catches[0];
            assert!(
                first
                    .type_annotation
                    .as_ref()
                    .is_some_and(|ty| ty.name == "IOException")
            );
            assert!(first.identifier.as_deref() == Some("io"));
            let second = &try_stmt.catches[1];
            assert!(second.type_annotation.is_none());
            assert!(second.identifier.is_none());
        }
        other => panic!("expected try statement, found {other:?}"),
    }
}

#[test]
fn parses_try_with_multiple_filtered_catches() {
    let source = r"
public void Recover()
{
    try
    {
        Work();
    }
    catch (TransientError err) when (err.ShouldRetry() && err.Attempts < 3)
    {
        err.Retry();
    }
    catch (CriticalError fatal) when (fatal.IsTerminal())
    {
        throw;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Try(try_stmt) => {
            assert_eq!(try_stmt.catches.len(), 2);
            let first = &try_stmt.catches[0];
            assert!(
                first
                    .filter
                    .as_ref()
                    .is_some_and(|expr| expr.text.contains("ShouldRetry() && err.Attempts < 3"))
            );
            let second = &try_stmt.catches[1];
            assert!(
                second
                    .filter
                    .as_ref()
                    .is_some_and(|expr| expr.text.contains("IsTerminal"))
            );
        }
        other => panic!("expected try statement, found {other:?}"),
    }
}

#[test]
fn catch_filter_missing_paren_reports_error() {
    let source = r"
public void Recover()
{
    try
    {
        Work();
    }
    catch (TransientError err) when (err.ShouldRetry()
    {
        err.Retry();
    }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("unterminated catch filter expression")),
        "expected catch filter diagnostic, found {:#?}",
        diagnostics
    );
}
