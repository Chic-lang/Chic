use super::common::RequireExt;
use super::*;

#[test]
fn lowers_ref_argument_with_unique_borrow() {
    let source = r#"
namespace Sample;

public void Mirror(ref int value) { }

public void Use()
{
    var data = 1;
    Mirror(ref data);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let entry = &use_func.body.blocks[0];

    let borrow = entry
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Borrow { kind, .. } => Some(kind),
            _ => None,
        })
        .expect("expected borrow statement for ref argument");
    assert!(matches!(borrow, BorrowKind::Unique));

    let Terminator::Call { arg_modes, .. } = entry
        .terminator
        .as_ref()
        .expect("expected call terminator in entry block")
    else {
        unreachable!();
    };
    assert_eq!(arg_modes, &[ParamMode::Ref]);
}

#[test]
fn lowers_out_argument_with_unique_borrow() {
    let source = r#"
namespace Sample;

public void Set(out int value)
{
    value = 42;
}

public void Use()
{
    var data = 0;
    Set(out data);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let entry = &use_func.body.blocks[0];

    let borrow = entry
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Borrow { kind, .. } => Some(kind),
            _ => None,
        })
        .expect("expected borrow statement for out argument");
    assert!(matches!(borrow, BorrowKind::Unique));

    let Terminator::Call { arg_modes, .. } =
        entry.terminator.as_ref().expect("expected call terminator")
    else {
        unreachable!();
    };
    assert_eq!(arg_modes, &[ParamMode::Out]);
}

#[test]
fn reports_missing_ref_modifier_on_call() {
    let source = r#"
namespace Sample;

public void Mirror(ref int value) { }

public void Use()
{
    var data = 0;
    Mirror(data);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("ref")),
        "expected diagnostic referencing missing ref modifier, found {:?}",
        lowering.diagnostics
    );
}
