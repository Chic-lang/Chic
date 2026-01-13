use super::helper::*;
use super::*;
use crate::mir::AggregateKind;

fn find_assignment<'a, F>(func: &'a MirFunction, predicate: F) -> Option<&'a MirStatementKind>
where
    F: Fn(&MirStatementKind) -> bool,
{
    func.body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .map(|stmt| &stmt.kind)
        .find(|kind| predicate(kind))
}

#[test]
fn lowers_inclusive_range_expression_into_aggregate() {
    let source = r#"
namespace Sample;
import Std.Range;

public RangeInclusive SliceBounds(int start, int end)
{
    return start..=end;
}
"#;

    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "SliceBounds");
    let aggregate = find_assignment(func, |kind| {
        matches!(
            kind,
            MirStatementKind::Assign {
                value:
                    Rvalue::Aggregate {
                        kind: AggregateKind::Adt { name, variant: None },
                        fields,
                    },
                ..
            } if name == "Std::Range::RangeInclusive" && fields.len() == 2
        )
    })
    .expect("expected aggregate construction for range expression");

    match aggregate {
        MirStatementKind::Assign {
            value: Rvalue::Aggregate { fields, .. },
            ..
        } => {
            assert!(matches!(fields[0], Operand::Copy(_)));
            assert!(matches!(fields[1], Operand::Copy(_)));
        }
        _ => panic!("unexpected statement shape for range aggregate"),
    }
}

#[test]
fn lowers_index_from_end_into_length_and_subtraction() {
    let source = r#"
namespace Sample;
import Std.Span;

public int Last(Span<int> values)
{
    return values[^1];
}
"#;

    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Last");
    let has_len = find_assignment(func, |kind| {
        matches!(
            kind,
            MirStatementKind::Assign {
                value: Rvalue::Len(_),
                ..
            }
        )
    });
    assert!(
        has_len.is_some(),
        "expected length computation for index-from-end"
    );

    let has_sub = find_assignment(func, |kind| {
        matches!(
            kind,
            MirStatementKind::Assign {
                value: Rvalue::Binary { op: BinOp::Sub, .. },
                ..
            }
        )
    });
    assert!(
        has_sub.is_some(),
        "expected subtraction to translate from-end index"
    );
}

#[test]
fn lowers_foreach_over_range_into_counted_loop() {
    let source = r#"
namespace Sample;
import Std.Range;

public int SumRange()
{
    var total = 0;
    foreach (var i in 1 .. 3)
    {
        total = total + i;
    }
    return total;
}
"#;

    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "SumRange");
    let has_idx_local = func.body.locals.iter().any(|decl| {
        decl.name
            .as_deref()
            .is_some_and(|name| name.contains("__foreach_idx_local_"))
    });
    assert!(has_idx_local, "expected foreach index local to be created");

    let increment = find_assignment(func, |kind| {
        matches!(
            kind,
            MirStatementKind::Assign {
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(Place { .. }),
                    rhs: Operand::Const(ConstOperand {
                        value: ConstValue::UInt(1),
                        ..
                    }),
                    ..
                },
                ..
            }
        )
    });
    assert!(
        increment.is_some(),
        "expected foreach lowering to increment the loop index"
    );
}
