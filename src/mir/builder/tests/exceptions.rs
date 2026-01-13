use super::common::RequireExt;
use super::*;
use crate::mir::ConstOperand;

const EXCEPTIONS_PREAMBLE: &str = r#"
public class Resource
{
    public void Dispose()
    {
    }
}

public void Process(int value)
{
}

public void Use(Resource resource)
{
}

public void Inner()
{
}

public void CleanupInner()
{
}

public void CleanupOuter()
{
}

public void Work()
{
}

public void Cleanup()
{
}

public class Exception
{
    public bool IsTransient()
    {
        return true;
    }

    public void Log()
    {
    }
}
"#;

fn with_exceptions_preamble(body: &str) -> String {
    format!(
        "namespace Recovery;\n\n{preamble}\n{body}",
        preamble = EXCEPTIONS_PREAMBLE,
        body = body
    )
}

#[test]
fn lowers_try_catch_with_filter_into_exception_region() {
    let source = with_exceptions_preamble(
        r#"
public int Attempt(int value)
{
try
{
    Process(value);
}
catch (Exception err) when (err.IsTransient())
{
    return 1;
}

return 0;
}
"#,
    );
    let parsed = parse_module(&source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Attempt"))
        .require("Attempt function");
    let body = &func.body;

    assert_eq!(
        body.exception_regions.len(),
        1,
        "expected single exception region"
    );
    let region = &body.exception_regions[0];
    assert_eq!(region.catches.len(), 1, "expected single catch clause");
    let catch = &region.catches[0];
    let catch_ty = catch
        .ty
        .as_ref()
        .map(|ty| ty.canonical_name())
        .unwrap_or_else(|| "<missing>".into());
    assert_eq!(catch_ty, "Exception", "expected catch type metadata");
    let binding = catch.binding.require("binding local");
    assert_eq!(
        body.locals[binding.0].name.as_deref(),
        Some("err"),
        "expected binding name recorded on local"
    );

    let dispatch = region.dispatch.require("dispatch block");
    match body.blocks[dispatch.0]
        .terminator
        .as_ref()
        .require("dispatch terminator")
    {
        Terminator::Goto { target } => assert_eq!(
            *target, catch.entry,
            "catch dispatch should jump to first catch entry"
        ),
        other => panic!("expected goto from dispatch, found {other:?}"),
    }

    let entry_block = &body.blocks[catch.entry.0];
    assert!(
        matches!(
            entry_block.terminator,
            Some(Terminator::Call { .. } | Terminator::Goto { .. })
        ),
        "catch entry should evaluate binding/filter before dispatch"
    );

    let filter = catch.filter.as_ref().require("filter metadata");
    let filter_block = &body.blocks[filter.block.0];
    match filter_block
        .terminator
        .as_ref()
        .require("catch filter terminator")
    {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            assert!(
                targets.iter().any(|(_, target)| *target == catch.body),
                "filter switch should branch into catch body"
            );
            let otherwise_block = &body.blocks[otherwise.0];
            match otherwise_block
                .terminator
                .as_ref()
                .require("otherwise terminator")
            {
                Terminator::Throw { .. } => {}
                other => {
                    panic!("expected throw terminator on failed filter, found {other:?}")
                }
            }
        }
        other => panic!("expected switch on catch filter guard, found {other:?}"),
    }

    assert!(filter.parsed, "catch filter should parse successfully");

    let pending_try = body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .any(|stmt| {
            matches!(
                &stmt.kind,
                MirStatementKind::Pending(p) if p.kind == PendingStatementKind::Try
            )
        });
    assert!(
        !pending_try,
        "try/catch lowering should not leave pending statements"
    );
}

#[test]
fn lowers_try_finally_with_unhandled_path() {
    let source = with_exceptions_preamble(
        r#"
public void Cleanup(Resource resource)
{
try
{
    Use(resource);
}
finally
{
    resource.Dispose();
}
}
"#,
    );
    let parsed = parse_module(&source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.starts_with("Recovery::Cleanup") && f.signature.params.len() == 1)
        .require("Cleanup function");
    let body = &func.body;

    assert_eq!(
        body.exception_regions.len(),
        1,
        "expected try/finally region"
    );
    let region = &body.exception_regions[0];
    let finally = region.finally.as_ref().require("finally metadata");
    let exit_block = &body.blocks[finally.exit.0];
    match exit_block
        .terminator
        .as_ref()
        .require("finally exit terminator")
    {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            assert_eq!(
                *otherwise, region.after_block,
                "finally should fall through to after-block"
            );
            assert!(
                targets.iter().any(|(_, target)| {
                    matches!(
                        body.blocks[target.0].terminator.as_ref(),
                        Some(Terminator::Throw { .. })
                    )
                }),
                "finally exit should branch into throw path"
            );
        }
        other => panic!("expected switch terminator for finally exit, found {other:?}"),
    }
}

#[test]
fn lowers_nested_try_finally_chains() {
    let source = with_exceptions_preamble(
        r#"
public void Sequence()
{
try
{
    try
    {
        Inner();
    }
    finally
    {
        CleanupInner();
    }
}
finally
{
    CleanupOuter();
}
}
"#,
    );
    let parsed = parse_module(&source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Sequence"))
        .require("Sequence function");
    let body = &func.body;

    let final_regions: Vec<_> = body
        .exception_regions
        .iter()
        .filter_map(|region| {
            region.finally.as_ref().map(|finally| {
                (
                    region.try_entry,
                    region.after_block,
                    finally.entry,
                    finally.exit,
                )
            })
        })
        .collect();
    assert_eq!(
        final_regions.len(),
        2,
        "expected nested try/finally regions"
    );

    let pending_flag_count = body
        .locals
        .iter()
        .filter(|decl| {
            decl.name
                .as_deref()
                .is_some_and(|name| name.starts_with("__pending_exception"))
        })
        .count();
    assert_eq!(
        pending_flag_count, 2,
        "each finally should allocate its own pending-exception flag"
    );

    for (try_entry, after_block, _entry, exit) in final_regions {
        let exit_block = &body.blocks[exit.0];
        let terminator = exit_block
            .terminator
            .as_ref()
            .require("finally exit terminator");
        match terminator {
            Terminator::SwitchInt {
                discr,
                targets,
                otherwise,
            } => {
                assert_eq!(
                    *otherwise, after_block,
                    "finally region at {try_entry:?} should fall through to its after block"
                );
                assert!(
                    targets.iter().any(|(_, block)| {
                        matches!(
                            body.blocks[block.0].terminator.as_ref(),
                            Some(Terminator::Throw { .. })
                        )
                    }),
                    "finally region at {try_entry:?} should branch into the throw path"
                );
                if let Operand::Copy(place) = discr {
                    let flag_name = body.locals[place.local.0]
                        .name
                        .as_deref()
                        .unwrap_or_default();
                    assert!(
                        flag_name.starts_with("__pending_exception"),
                        "finally region at {try_entry:?} should test the pending-exception flag, found {flag_name}"
                    );
                } else {
                    panic!(
                        "finally region at {try_entry:?} should branch on the pending-exception flag"
                    );
                }
            }
            other => panic!(
                "expected switch terminator for finally region at {try_entry:?}, found {other:?}"
            ),
        }
    }
}

#[test]
fn catch_cleanup_resets_exception_flag_before_finally() {
    let source = with_exceptions_preamble(
        r#"
public void Recover()
{
try
{
    Work();
}
catch (Exception err)
{
    err.Log();
}
finally
{
    Cleanup();
}
}
"#,
    );
    let parsed = parse_module(&source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Recover"))
        .require("Recover function");
    let region = func
        .body
        .exception_regions
        .first()
        .require("expected exception region");

    let catch = region.catches.first().require("catch metadata");
    let cleanup_block = &func.body.blocks[catch.cleanup.0];
    let resets_flag = cleanup_block.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            MirStatementKind::Assign {
                value: Rvalue::Use(Operand::Const(ConstOperand {
                    value: ConstValue::Bool(false),
                    ..
                })),
                ..
            }
        )
    });
    assert!(
        resets_flag,
        "catch cleanup should reset the pending-exception flag before finally"
    );
}

#[test]
fn throw_nullable_exception_reports_diagnostic() {
    let source = r"
namespace Recovery;

public class Thrower
{
    public void Raise(Exception? pending)
    {
        throw pending;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("may be `null`")),
        "expected nullable throw diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn throw_non_nullable_exception_succeeds() {
    let source = r"
namespace Recovery;

public class Thrower
{
    public void Raise(Exception pending)
    {
        throw pending;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn catch_nullable_exception_reports_diagnostic() {
    let source = r"
namespace Recovery;

public void Handle()
{
    try
    {
        return;
    }
    catch (Exception? pending)
    {
        return;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("catch clause type `Exception?`")),
        "expected nullable catch diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowers_throw_without_try_into_throw_terminator() {
    let source = r"
namespace Recovery;

public void Fail(Exception err)
{
    throw err;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Fail"))
        .require("Fail function");
    let entry = func
        .body
        .blocks
        .first()
        .expect("throw function should have an entry block");
    match entry.terminator.as_ref().require("throw terminator") {
        Terminator::Throw { exception, ty } => {
            assert!(
                exception.is_some(),
                "throw should carry an exception operand"
            );
            let ty_name = ty
                .as_ref()
                .map(|ty| ty.canonical_name())
                .unwrap_or_else(|| "<missing>".into());
            assert_eq!(
                ty_name, "Exception",
                "throw terminator should record exception type"
            );
        }
        other => panic!("expected throw terminator, found {other:?}"),
    }
}

#[test]
fn return_throw_omits_return_terminator() {
    let source = r"
namespace Recovery;

public int Fail(Exception err)
{
    return throw err;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Fail"))
        .require("Fail function");
    let entry = func
        .body
        .blocks
        .first()
        .expect("throw function should have an entry block");
    assert!(
        matches!(entry.terminator, Some(Terminator::Throw { .. })),
        "return throw should end with a throw terminator"
    );
}

#[test]
fn throw_requires_error_operand_type() {
    let source = r"
namespace Recovery;

public void Fail(int value)
{
    throw value;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("does not derive from `Exception`")),
        "expected diagnostic when throwing non-error operand: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn rethrow_outside_try_reports_diagnostic() {
    let source = r"
namespace Recovery;

public void Fail()
{
    throw;
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("cannot rethrow outside a catch block")),
        "expected rethrow diagnostic: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn throw_only_function_skips_synthesised_return() {
    let source = r#"
namespace Throwing;

public class Exception { }
public class FormatException : Exception { }

public int Parse()
{
    throw new FormatException();
}
"#;

    let parsed = parse_module(source).require("parse throw-only module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Parse"))
        .require("Parse function");
    let body = &func.body;

    let throw_term = body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Throw { ty, .. }) => Some(ty.clone()),
            _ => None,
        })
        .require("throw terminator");

    let throw_ty = throw_term
        .as_ref()
        .map(|ty| ty.canonical_name())
        .unwrap_or_else(|| "<missing>".into());
    assert_eq!(
        throw_ty, "FormatException",
        "throw should record exception type"
    );

    let has_return = body
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Some(Terminator::Return)));
    assert!(
        !has_return,
        "throw-only function should not synthesise a return terminator"
    );
}

#[test]
fn catch_requires_error_type_annotation() {
    let source = r"
namespace Recovery;

public void Recover()
{
    try
    {
    }
    catch (int value)
    {
        value = 0;
    }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("does not derive from `Exception`")),
        "expected diagnostic when catch type is not an error: {:?}",
        lowering.diagnostics
    );
}
