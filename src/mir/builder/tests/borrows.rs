use super::*;
use crate::mir::builder::tests::common::RequireExt;
use crate::typeck::BorrowEscapeCategory;

fn borrow_constraints<'a>(
    constraints: &'a [TypeConstraint],
    name: &str,
) -> Vec<&'a TypeConstraint> {
    constraints
        .iter()
        .filter(|constraint| {
            matches!(
                &constraint.kind,
                ConstraintKind::BorrowEscape { function, .. } if function.ends_with(name)
            )
        })
        .collect()
}

#[test]
fn returning_ref_parameter_produces_borrow_escape_constraint() {
    let source = r"
namespace Borrow;

public class Samples
{
    public string Return(ref string value)
    {
        return value;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);

    let constraints = borrow_constraints(&lowering.constraints, "::Return");
    assert_eq!(constraints.len(), 1, "expected single borrow escape");

    let constraint = constraints[0];
    let ConstraintKind::BorrowEscape {
        parameter,
        parameter_mode,
        escape,
        ..
    } = &constraint.kind
    else {
        unreachable!("filtered constraint must be borrow escape");
    };

    assert_eq!(parameter, "value");
    assert_eq!(parameter_mode, &ParamMode::Ref);
    assert!(matches!(escape, BorrowEscapeCategory::Return));
}

#[test]
fn storing_in_parameter_records_store_escape() {
    let source = r"
namespace Borrow;

public class Cache
{
    private string _cached;

    public void Remember(in string data)
    {
        _cached = data;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);

    let constraints = borrow_constraints(&lowering.constraints, "::Remember");
    assert_eq!(constraints.len(), 1, "expected single borrow escape");

    let ConstraintKind::BorrowEscape {
        parameter_mode,
        escape,
        ..
    } = &constraints[0].kind
    else {
        unreachable!();
    };

    assert_eq!(parameter_mode, &ParamMode::In);
    match escape {
        BorrowEscapeCategory::Store { target } => {
            assert!(
                target.contains("_cached"),
                "expected target to reference backing field: {target}"
            );
        }
        other => panic!("expected store escape, found {other:?}"),
    }
}

#[test]
fn capturing_ref_parameter_creates_capture_escape() {
    let source = r"
namespace Borrow;

public delegate string Producer();

public class Closures
{
    public Producer Capture(ref string value)
    {
        return () => value;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);

    let constraints = borrow_constraints(&lowering.constraints, "::Capture");
    assert_eq!(constraints.len(), 1, "expected borrow escape for capture");

    let ConstraintKind::BorrowEscape { escape, .. } = &constraints[0].kind else {
        unreachable!();
    };

    match escape {
        BorrowEscapeCategory::Capture { closure } => {
            assert!(
                closure.contains("Closures::Capture"),
                "unexpected closure name `{closure}`"
            );
        }
        other => panic!("expected capture escape, found {other:?}"),
    }
}
