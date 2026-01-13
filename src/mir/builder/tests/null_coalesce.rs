use super::common::RequireExt;
use super::*;

#[test]
fn lowers_null_coalesce_expression_into_branch() {
    let source = r"
namespace Demo;

public string Choose(string? input, string fallback)
{
    return input ?? fallback;
}
";
    let parsed = parse_module(source).require("parse null-coalescing expression module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Choose")
        .require("expected Demo::Choose function");

    let body = &function.body;
    assert!(
        body.blocks.len() >= 4,
        "expected at least entry/null/non-null/join blocks, found {}",
        body.blocks.len()
    );

    let entry = &body.blocks[0];
    let switch = entry
        .terminator
        .as_ref()
        .require("entry block should terminate in switch");
    let (null_id, non_null_id) = match switch {
        Terminator::SwitchInt {
            discr,
            targets,
            otherwise,
        } => {
            assert_eq!(targets.len(), 1, "expected single null target");
            let (pattern, null_block) = targets[0];
            assert_eq!(pattern, 0, "null branch should trigger on HasValue == 0");
            if let Operand::Copy(place) = discr {
                assert!(
                    place
                        .projection
                        .iter()
                        .any(|elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "HasValue")
                            || matches!(elem, ProjectionElem::Field(index) if *index == 0)),
                    "switch discriminant should access HasValue flag, found {:?}",
                    place.projection
                );
            } else {
                panic!("expected switch discriminant to be a place copy, found {discr:?}");
            }
            (null_block, *otherwise)
        }
        other => panic!("expected switch terminator, found {other:?}"),
    };

    let null_block = &body.blocks[null_id.0];
    let non_null_block = &body.blocks[non_null_id.0];
    assert_eq!(
        non_null_block.statements.len(),
        1,
        "non-null branch should assign result directly"
    );
    assert_eq!(
        null_block.statements.len(),
        1,
        "null branch should contain fallback assignment"
    );

    match &non_null_block.statements[0].kind {
        MirStatementKind::Assign { value, .. } => match value {
            Rvalue::Use(Operand::Copy(place)) => assert!(
                place.projection.iter().any(
                    |elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "Value")
                        || matches!(elem, ProjectionElem::Field(index) if *index == 1)
                ),
                "non-null path should copy payload value, found projection {:?}",
                place.projection
            ),
            other => panic!("expected payload copy in non-null branch, found {other:?}"),
        },
        other => panic!("expected assignment in non-null branch, found {other:?}"),
    }

    match &null_block.statements[0].kind {
        MirStatementKind::Assign { value, .. } => match value {
            Rvalue::Use(Operand::Copy(place)) => assert!(
                place.projection.is_empty(),
                "null branch should copy fallback local, found projection {:?}",
                place.projection
            ),
            other => panic!("expected fallback copy in null branch, found {other:?}"),
        },
        other => panic!("expected assignment in null branch, found {other:?}"),
    }
}

#[test]
fn null_coalesce_assignment_updates_nullable_slot() {
    let source = r"
namespace Demo;

public string? Ensure(string? current, string fallback)
{
    var value = current;
    value ??= fallback;
    return value;
}
";
    let parsed = parse_module(source).require("parse null-coalescing assignment module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Ensure")
        .require("expected Demo::Ensure function");

    let body = &function.body;
    let switch_block = body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .require("expected switch block for null-coalescing assignment");
    let (null_block_id, non_null_block_id) = match switch_block.terminator.as_ref().unwrap() {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            assert_eq!(targets.len(), 1);
            (targets[0].1, *otherwise)
        }
        _ => unreachable!("filtered by find"),
    };

    let null_block = &body.blocks[null_block_id.0];
    let non_null_block = &body.blocks[non_null_block_id.0];
    assert!(
        non_null_block.statements.is_empty(),
        "non-null branch should not mutate the slot"
    );
    assert_eq!(
        null_block.statements.len(),
        2,
        "null branch should assign payload and flag"
    );

    if let MirStatementKind::Assign { place, value } = &null_block.statements[0].kind {
        assert!(
            place.projection.iter().any(
                |elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "Value")
                    || matches!(elem, ProjectionElem::Field(index) if *index == 1)
            ),
            "first null-branch assignment should target payload field, found {:?}",
            place.projection
        );
        assert!(
            matches!(value, Rvalue::Use(Operand::Copy(_))),
            "payload assignment should copy fallback operand"
        );
    } else {
        panic!("expected payload assignment in null branch");
    }

    if let MirStatementKind::Assign { place, value } = &null_block.statements[1].kind {
        assert!(
            place.projection.iter().any(
                |elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "HasValue")
                    || matches!(elem, ProjectionElem::Field(index) if *index == 0)
            ),
            "second null-branch assignment should set HasValue flag, found {:?}",
            place.projection
        );
        assert!(
            matches!(
                value,
                Rvalue::Use(Operand::Const(ConstOperand {
                    value: ConstValue::Bool(true),
                    ..
                }))
            ),
            "HasValue assignment should store `true`"
        );
    } else {
        panic!("expected HasValue flag assignment in null branch");
    }
}

#[test]
fn property_null_coalesce_invokes_setter_only_when_null() {
    let source = r"
namespace Demo;

public class Person
{
    public string? Name { get; set; }

    public void EnsureName(ref this, string fallback)
    {
        this.Name ??= fallback;
    }
}
";
    let parsed = parse_module(source).require("parse property null-coalescing module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let method = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Person::EnsureName")
        .require("expected EnsureName method");

    let body = &method.body;
    let switch_block = body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .require("expected switch block stemming from null-coalescing");

    let (null_block_id, non_null_block_id) = match switch_block.terminator.as_ref().unwrap() {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            assert_eq!(targets.len(), 1);
            (targets[0].1, *otherwise)
        }
        _ => unreachable!(),
    };

    let null_block = &body.blocks[null_block_id.0];
    let non_null_block = &body.blocks[non_null_block_id.0];
    assert!(
        non_null_block
            .terminator
            .as_ref()
            .is_some_and(|term| matches!(term, Terminator::Goto { .. })),
        "non-null branch should immediately jump to join"
    );

    match null_block.terminator.as_ref() {
        Some(Terminator::Call { func, .. }) => match func {
            Operand::Pending(PendingOperand { repr, .. }) => {
                assert_eq!(
                    repr, "Demo::Person::set_Name",
                    "null branch should invoke property setter"
                );
            }
            other => panic!("expected pending operand for setter call, found {other:?}"),
        },
        other => panic!("expected setter call terminator in null branch, found {other:?}"),
    }

    assert!(
        body.blocks.iter().any(|block| {
            if let Some(Terminator::Call {
                func: Operand::Pending(pending),
                ..
            }) = block.terminator.as_ref()
            {
                pending.repr == "Demo::Person::get_Name"
            } else {
                false
            }
        }),
        "getter call should be emitted before null-coalescing branch"
    );
}

#[test]
fn async_null_coalesce_only_awaits_in_null_branch() {
    let source = r"
namespace Demo;

public async Task<string> Compose(string? prefix, Future future)
{
    var value = prefix ?? await future;
    return value;
}
";
    let parsed = parse_module(source).require("parse async null-coalescing module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let method = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Compose")
        .require("expected Compose function");
    assert!(method.is_async, "function should be marked async");

    let body = &method.body;
    let switch_block = body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .require("expected switch block created by null-coalescing");
    let (null_block_id, non_null_block_id) = match switch_block.terminator.as_ref().unwrap() {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => (targets[0].1, *otherwise),
        _ => unreachable!(),
    };

    let null_block = &body.blocks[null_block_id.0];
    let non_null_block = &body.blocks[non_null_block_id.0];

    match null_block.terminator.as_ref() {
        Some(Terminator::Await { .. }) => {}
        other => panic!("null branch should await future, found {other:?}"),
    }
    assert!(
        non_null_block
            .terminator
            .as_ref()
            .is_some_and(|term| !matches!(term, Terminator::Await { .. })),
        "non-null branch must not await the fallback future"
    );

    let machine = method
        .body
        .async_machine
        .as_ref()
        .require("async metadata should be recorded");
    assert_eq!(
        machine.suspend_points.len(),
        1,
        "expected single suspend point"
    );
    let point = &machine.suspend_points[0];
    assert_eq!(
        point.await_block, null_block.id,
        "await suspend point should match null branch"
    );
}
