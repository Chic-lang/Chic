use super::common::RequireExt;
use super::*;

#[test]
fn lowers_switch_with_guard_into_branch_chain() {
    let source = r"
namespace Spec;

public int Select(int value, bool flag)
{
var result = 0;
switch (value)
{
    case 0:
        result = 1;
        break;
    case 1 when (flag):
        result = 2;
        break;
    default:
        result = 3;
        break;
}

return result;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        assert!(
            lowering
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("does not support list patterns")),
            "unexpected diagnostics: {0:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = &lowering.module.functions[0];
    for block in &func.body.blocks {
        for stmt in &block.statements {
            assert!(
                !matches!(stmt.kind, MirStatementKind::Pending(_)),
                "found pending statement in block {}",
                block.id
            );
        }
        if let Some(Terminator::Pending(_)) = &block.terminator {
            panic!("found pending terminator in block {}", block.id);
        }
    }

    let switch_int_count = func
        .body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .count();
    assert!(
        switch_int_count >= 2,
        "expected at least two SwitchInt terminators, found {switch_int_count}"
    );
}

#[test]
fn lowers_switch_over_char_values() {
    let source = r"
namespace Sample;

public int Evaluate(char value)
{
switch (value)
{
    case 'A':
        return 1;
    case '\uD83D':
        return 2;
    default:
        return 0;
}
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Evaluate")
        .require("missing Evaluate function");

    let switch_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::SwitchInt { .. })))
        .require("expected switch terminator for char dispatch");

    let Terminator::SwitchInt {
        discr,
        targets,
        otherwise,
    } = switch_block.terminator.as_ref().unwrap()
    else {
        unreachable!();
    };

    let discr_local = match discr {
        Operand::Copy(place) | Operand::Move(place) => place.local,
        other => panic!("unexpected discriminant operand: {other:?}"),
    };
    let discr_decl = func
        .body
        .locals
        .get(discr_local.0)
        .expect("discriminant local should exist");
    assert_eq!(discr_decl.ty.canonical_name(), "bool");

    assert_eq!(targets.len(), 1, "literal case dispatch uses boolean guard");
    assert_eq!(targets[0].0, 1, "true branch should route to case body");
    assert_ne!(targets[0].1, *otherwise, "otherwise branch should differ");

    let mut compared_chars = Vec::new();
    for block in &func.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Assign {
                value: Rvalue::Binary {
                    op: BinOp::Eq, rhs, ..
                },
                ..
            } = &stmt.kind
            {
                if let Operand::Const(ConstOperand {
                    value: ConstValue::Char(ch),
                    ..
                }) = rhs
                {
                    compared_chars.push(*ch);
                }
            }
        }
    }
    compared_chars.sort_unstable();
    assert_eq!(compared_chars, vec!['A' as u16, 0xD83D]);
}

#[test]
fn switch_literal_guards_assign_discriminant_temp() {
    let source = r"
namespace N;

public enum E
{
    A = 0,
    B = 1,
}

public static class C
{
    public static int Foo(E e)
    {
        switch (e)
        {
            case E.A:
                return 1;
            case E.B:
                return 2;
            default:
                return 3;
        }
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

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "N::C::Foo")
        .require("missing Foo function");

    for block in &func.body.blocks {
        let Some(Terminator::SwitchInt { discr, .. }) = &block.terminator else {
            continue;
        };
        let discr_local = match discr {
            Operand::Copy(place) | Operand::Move(place) => place.local,
            _ => continue,
        };
        let Some(decl) = func.body.locals.get(discr_local.0) else {
            continue;
        };
        let Some(name) = decl.name.as_deref() else {
            continue;
        };
        if !name.starts_with("$t") {
            continue;
        }

        let killed_in_block = block.statements.iter().any(|stmt| {
            matches!(stmt.kind, MirStatementKind::StorageDead(local) if local == discr_local)
        });
        assert!(
            !killed_in_block,
            "SwitchInt discriminant temp `{name}` must not be StorageDead in the same block; block={:?} stmts={:?} term={:?}",
            block.id, block.statements, block.terminator
        );

        let assigned_somewhere = func.body.blocks.iter().any(|candidate| {
            candidate.statements.iter().any(|stmt| match &stmt.kind {
                MirStatementKind::Assign { place, .. } => {
                    place.local == discr_local && place.projection.is_empty()
                }
                MirStatementKind::ZeroInit { place } | MirStatementKind::Deinit(place) => {
                    place.local == discr_local && place.projection.is_empty()
                }
                _ => false,
            })
        });
        assert!(
            assigned_somewhere,
            "SwitchInt discriminant temp `{name}` must be assigned by at least one predecessor; local={:?}",
            discr_local
        );
    }
}

#[test]
fn lowers_goto_case_and_default_targets() {
    let source = r"
namespace Spec;

public int Route(int value)
{
var result = 0;
switch (value)
{
    case 0:
        goto case 1;
    case 1:
        result = 42;
        break;
    default:
        result = -1;
        break;
}

return result;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        return;
    }

    let func = &lowering.module.functions[0];
    for block in &func.body.blocks {
        for stmt in &block.statements {
            assert!(
                !matches!(stmt.kind, MirStatementKind::Pending(_)),
                "found pending statement in block {}",
                block.id
            );
        }
        if let Some(Terminator::Pending(_)) = &block.terminator {
            panic!("found pending terminator in block {}", block.id);
        }
    }

    let goto_count = func
        .body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Some(Terminator::Goto { .. })))
        .count();
    assert!(
        goto_count >= 3,
        "expected multiple goto terminators (for goto, breaks, and dispatch); found {goto_count}"
    );
}

#[test]
fn records_pattern_bindings_for_switch_match_arms() {
    let source = r"
namespace Patterns;

public struct Point { public int X; public int Y; }

public int SumWhenGreater(Point pt)
{
switch (pt)
{
    case Point { X: var x, Y: var y } when x > y:
        return x + y;
    default:
        return 0;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        assert!(
            lowering
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("does not support list patterns")),
            "unexpected diagnostics: {0:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Patterns::SumWhenGreater")
        .require("missing lowered function");
    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .require("expected match terminator block");

    let Terminator::Match { arms, .. } = match_block
        .terminator
        .as_ref()
        .expect("match block should have a terminator")
    else {
        unreachable!();
    };
    assert_eq!(arms.len(), 1);
    let arm = &arms[0];
    if arm.bindings.is_empty() {
        return;
    }
    assert_eq!(arm.bindings.len(), 2);

    let binding_names: Vec<_> = arm.bindings.iter().map(|b| b.name.as_str()).collect();
    assert!(binding_names.contains(&"x"));
    assert!(binding_names.contains(&"y"));

    for binding in &arm.bindings {
        let local = func
            .body
            .local(binding.local)
            .require("binding local should exist");
        assert_eq!(local.name.as_deref(), Some(binding.name.as_str()));
        assert!(matches!(local.kind, LocalKind::Local));
        assert_eq!(binding.projection.len(), 1);
        match &binding.projection[0] {
            PatternProjectionElem::FieldNamed(field) => {
                assert!(field == "X" || field == "Y");
            }
            other => panic!("unexpected projection element: {other:?}"),
        }
    }

    let guard = arm.guard.as_ref().require("expected guard metadata");
    assert!(guard.parsed, "guard expression should parse successfully");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "control-flow guard lowering requires a realistic program fixture"
)]
fn lowers_match_guards_into_control_flow() {
    let source = r"
namespace Guards;

public struct Pair { public int A; public int B; }

public int Select(Pair pair)
{
switch (pair)
{
    case Pair { A: var a, B: var b } when a > b:
        return a;
    default:
        return 0;
}
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        assert!(
            lowering
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("does not support list patterns")),
            "unexpected diagnostics: {0:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Guards::Select")
        .require("missing lowered function");

    let entry = &func.body.blocks[0];
    let first_target = match &entry.terminator {
        Some(Terminator::Goto { target }) => *target,
        other => panic!("expected goto into match chain, found {other:?}"),
    };

    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| block.id == first_target)
        .require("match dispatch block not found");
    let match_term = match &match_block.terminator {
        Some(Terminator::Match {
            arms, otherwise, ..
        }) => {
            assert_eq!(arms.len(), 1);
            (*otherwise, arms[0].target)
        }
        _ => return,
    };

    let fallback_block = match_term.0;
    let binding_block = match_term.1;
    let binding_block_data = func
        .body
        .blocks
        .iter()
        .find(|block| block.id == binding_block)
        .require("binding block not found");
    assert!(
        !binding_block_data.statements.is_empty(),
        "binding block should initialize locals"
    );
    let mut saw_assign = false;
    for stmt in &binding_block_data.statements {
        match &stmt.kind {
            MirStatementKind::Assign { .. } => saw_assign = true,
            MirStatementKind::StorageLive(_) => {}
            other => panic!("unexpected statement in binding block: {other:?}"),
        }
    }
    assert!(
        saw_assign,
        "expected at least one assignment in binding block"
    );
    let next_target = match binding_block_data
        .terminator
        .as_ref()
        .require("binding block should terminate")
    {
        Terminator::Goto { target } => *target,
        other => panic!("expected binding goto, found {other:?}"),
    };

    let guard_block = next_target;
    let guard_block_data = func
        .body
        .blocks
        .iter()
        .find(|block| block.id == guard_block)
        .require("guard block not found");
    let guard_term = guard_block_data
        .terminator
        .as_ref()
        .require("guard block should terminate");
    match guard_term {
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            assert_eq!(targets.len(), 1);
            let (_, body_target) = targets[0];
            assert_ne!(
                body_target, *otherwise,
                "guard true should diverge from false branch"
            );
            assert_eq!(
                *otherwise, fallback_block,
                "guard false should fall through to fallback block"
            );
        }
        other => panic!("expected guard SwitchInt terminator, found {other:?}"),
    }
}

#[test]
fn goto_case_resolves_complex_patterns() {
    let source = r"
namespace Geometry;

public int Process(Shape shape)
{
switch (shape)
{
    case Shape.Circle { Radius: 1 }:
        goto case Shape.Circle { Radius: 2 };
    case Shape.Circle { Radius: 2 }:
        return 2;
    default:
        return 0;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    if !lowering.diagnostics.is_empty() {
        assert!(
            lowering
                .diagnostics
                .iter()
                .all(|diag| diag.message.contains("does not support list patterns")),
            "unexpected diagnostics: {0:?}",
            lowering.diagnostics
        );
        return;
    }

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Process"))
        .require("Process function");
    for block in &func.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Pending(pending) = &stmt.kind {
                assert_ne!(
                    pending.kind,
                    PendingStatementKind::Goto,
                    "goto case should not remain pending"
                );
            }
        }
    }
}

#[test]
fn goto_case_expression_guard_reports_error() {
    let source = r"
namespace Spec;

public int Jump(int value)
{
switch (value)
{
    case 0:
        goto case 1 when (value > 0);
    case 1:
        return 1;
    default:
        return -1;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("`goto case` cannot include a `when` guard")),
        "expected diagnostic about inline guard on goto case, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn goto_case_cannot_target_guarded_label() {
    let source = r"
namespace Spec;

public int Jump(int value)
{
switch (value)
{
    case 0 when (value == 0):
        goto case 0;
    default:
        return -1;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("`goto case` cannot target a pattern with a `when` guard")),
        "expected diagnostic about targeting guarded case, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn switch_pattern_bindings_emit_borrows_and_moves() {
    let source = r"
namespace Spec;

public int Consume((int X, int Y) pair)
{
    switch (pair)
    {
        case ref var unique:
            unique.X = 1;
            return unique.X;
        case in var snapshot:
            return snapshot.Y;
        case var mover move:
            return mover.X + mover.Y;
        default:
            return -1;
    }
}
";
    let parsed = parse_module(source).require("parse");
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
        .find(|f| f.name == "Spec::Consume")
        .require("missing Consume function");

    let mover_local = func
        .body
        .locals
        .iter()
        .enumerate()
        .find_map(|(idx, decl)| {
            if decl.name.as_deref() == Some("mover") {
                Some(LocalId(idx))
            } else {
                None
            }
        })
        .expect("expected mover binding local");

    let mut saw_shared_borrow = false;
    let mut saw_unique_borrow = false;
    let mut saw_move_assign = false;

    for block in &func.body.blocks {
        for stmt in &block.statements {
            match &stmt.kind {
                MirStatementKind::Borrow { kind, .. } => {
                    if *kind == BorrowKind::Shared {
                        saw_shared_borrow = true;
                    } else if *kind == BorrowKind::Unique {
                        saw_unique_borrow = true;
                    }
                }
                MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Move(_)),
                } if place.local == mover_local => {
                    saw_move_assign = true;
                }
                _ => {}
            }
        }
    }

    assert!(saw_shared_borrow, "expected shared borrow for `in` binding");
    assert!(
        saw_unique_borrow,
        "expected unique borrow for `ref` binding"
    );
    assert!(
        saw_move_assign,
        "expected move assignment for `move` binding"
    );
}
