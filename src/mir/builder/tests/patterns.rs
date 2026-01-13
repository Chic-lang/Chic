use super::common::RequireExt;
use super::*;
use crate::mir::AggregateKind;

#[test]
fn lowers_struct_pattern_switch_into_match() {
    let source = r"
namespace Geometry;

public int Process(Shape shape)
{
switch (shape)
{
    case Shape.Circle { Radius: 10 }:
        return 1;
    default:
        return 0;
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
        .find(|f| f.name.ends_with("::Process"))
        .require("missing Process function");
    let entry = &func.body.blocks[0];
    let mut dispatch = match entry.terminator.as_ref().require("terminator") {
        Terminator::Goto { target } => *target,
        other => panic!("expected goto into match chain, found {other:?}"),
    };

    loop {
        let block = func
            .body
            .blocks
            .iter()
            .find(|block| block.id == dispatch)
            .require("match dispatch block");
        match block.terminator.as_ref().require("match terminator") {
            Terminator::Goto { target } => dispatch = *target,
            Terminator::Match {
                arms, otherwise, ..
            } => {
                assert_eq!(arms.len(), 1, "expected single match arm");
                match &arms[0].pattern {
                    Pattern::Enum { variant, .. } => assert_eq!(variant, "Circle"),
                    other => panic!("expected enum pattern, found {other:?}"),
                }
                let _ = func
                    .body
                    .blocks
                    .iter()
                    .find(|block| block.id == *otherwise)
                    .require("default block present");
                break;
            }
            other => panic!("expected Match terminator, found {other:?}"),
        }
    }
}

#[test]
fn lowers_tuple_patterns_into_match() {
    let source = r"
namespace Tuples;

public int Sum((int, int) pair)
{
    switch (pair)
    {
        case (var a, var b):
            return a + b;
        default:
            return 0;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .require("expected match terminator");
    match match_block.terminator.as_ref().require("match terminator") {
        Terminator::Match { arms, .. } => {
            assert!(
                matches!(&arms[0].pattern, Pattern::Tuple(elements) if elements.len() == 2),
                "expected tuple pattern in first arm, found {:?}",
                arms[0].pattern
            );
        }
        other => panic!("unexpected terminator: {other:?}"),
    }
}

#[test]
fn lowers_positional_struct_pattern() {
    let source = r"
namespace Data;

public struct Pair { public int A; public int B; }

public int Process(Pair pair)
{
switch (pair)
{
    case Pair(1, 2):
        return 1;
    default:
        return 0;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .unwrap_or_else(|| {
            panic!(
                "expected Match terminator, found {:?}",
                func.body
                    .blocks
                    .iter()
                    .map(|block| &block.terminator)
                    .collect::<Vec<_>>()
            )
        });
    match match_block.terminator.as_ref().require("match terminator") {
        Terminator::Match { arms, .. } => {
            assert_eq!(arms.len(), 1);
            let pattern = &arms[0].pattern;
            match pattern {
                Pattern::Struct { path, fields } => {
                    assert_eq!(path.last().map(String::as_str), Some("Pair"));
                    assert_eq!(fields.len(), 2);
                }
                other => panic!("expected struct pattern, found {other:?}"),
            }
        }
        other => panic!("unexpected terminator: {other:?}"),
    }
}

#[test]
fn record_positional_pattern_uses_primary_constructor_fields() {
    let source = r"
namespace Data;

public record struct Point(int X, int Y)
{
    public int Z;
}

public int Process(Point value)
{
switch (value)
{
    case Point(var x, var y):
        return x + y;
    default:
        return 0;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Process"))
        .require("missing Process function");
    let match_block = func
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Match { .. })))
        .unwrap_or_else(|| {
            panic!(
                "expected Match terminator, found {:?}",
                func.body
                    .blocks
                    .iter()
                    .map(|block| &block.terminator)
                    .collect::<Vec<_>>()
            )
        });
    match match_block.terminator.as_ref().require("match terminator") {
        Terminator::Match { arms, .. } => {
            assert_eq!(arms.len(), 1);
            let Pattern::Struct { fields, .. } = &arms[0].pattern else {
                panic!("expected struct pattern, found {:?}", arms[0].pattern);
            };
            let field_names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
            assert_eq!(
                field_names,
                ["X", "Y"],
                "record positional patterns should use primary constructor fields"
            );
        }
        other => panic!("unexpected terminator: {other:?}"),
    }
}

#[test]
fn lowers_is_expression_into_match() {
    let source = r"
namespace Geometry;

public bool IsCircle(Shape shape)
{
return shape is Shape.Circle { Radius: 10 };
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let has_match = func
        .body
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Some(Terminator::Match { .. })));
    assert!(
        has_match,
        "expected `is` expression to lower into Match terminator"
    );
}

#[test]
fn list_pattern_generates_guard() {
    let source = r"
public int Check(int[] values)
{
switch (values)
{
    case [1, 2]:
        return 1;
    default:
        return 0;
}
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let mut has_index_check = false;
    for block in &func.body.blocks {
        for stmt in &block.statements {
            if let MirStatementKind::Assign {
                value:
                    Rvalue::Binary {
                        lhs: left,
                        rhs: right,
                        ..
                    },
                ..
            } = &stmt.kind
            {
                let index_operand = match (left, right) {
                    (
                        Operand::Copy(place),
                        Operand::Const(ConstOperand {
                            value: ConstValue::Int(_),
                            ..
                        }),
                    )
                    | (
                        Operand::Const(ConstOperand {
                            value: ConstValue::Int(_),
                            ..
                        }),
                        Operand::Copy(place),
                    ) => Some(place),
                    _ => None,
                };
                if let Some(place) = index_operand
                    && place
                        .projection
                        .iter()
                        .any(|elem| matches!(elem, ProjectionElem::Index(_)))
                {
                    has_index_check = true;
                    break;
                }
            }
        }
    }
    assert!(
        has_index_check,
        "expected guard lowering to introduce index checks"
    );
}

#[test]
fn list_pattern_tail_binding_creates_readonly_span() {
    let source = r"
public int Tail(int[] values)
{
    switch (values)
    {
        case [var head, ..tail]:
            return tail.Length;
        default:
            return 0;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "{0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let (local, name, field_count) = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign {
                place,
                value:
                    Rvalue::Aggregate {
                        kind: AggregateKind::Adt { name, .. },
                        fields,
                    },
            } => Some((place.local, name.clone(), fields.len())),
            _ => None,
        })
        .expect("expected aggregate assignment for tail binding");
    assert!(
        name.contains("ReadOnlySpan"),
        "expected readonly span aggregate name, found {}",
        name
    );
    assert_eq!(
        field_count, 4,
        "expected ptr/len/elem_size/elem_align fields"
    );
    assert!(
        matches!(func.body.locals[local.0].ty, Ty::ReadOnlySpan(_)),
        "tail binding local should be readonly span, found {:?}",
        func.body.locals[local.0].ty
    );
}
