use super::common::RequireExt;
use super::*;
use crate::mir::data::StatementKind as MirStatementKind;

fn extract_return_constant(body: &MirBody) -> Option<&ConstValue> {
    for block in &body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                if place.local == LocalId(0) {
                    if let Rvalue::Use(Operand::Const(constant)) = value {
                        return Some(&constant.value);
                    }
                }
            }
        }
    }
    None
}

#[test]
fn lowers_sizeof_for_types_and_variables() {
    let source = r#"
namespace Sample;

public struct Point
{
    public int X;
}

public usize SizeOfPrimitive()
{
    return sizeof(int);
}

public usize SizeOfStruct()
{
    return sizeof(Point);
}

public usize SizeOfVariable(Point value)
{
    return sizeof value;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let primitive = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("SizeOfPrimitive"))
        .expect("missing SizeOfPrimitive function");
    let primitive_const =
        extract_return_constant(&primitive.body).expect("expected constant return for sizeof(int)");
    match primitive_const {
        ConstValue::UInt(value) => assert_eq!(*value, 4),
        other => panic!("expected uint constant, found {other:?}"),
    }

    let point = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("SizeOfStruct"))
        .expect("missing SizeOfStruct function");
    let point_const =
        extract_return_constant(&point.body).expect("expected constant return for sizeof(Point)");
    match point_const {
        ConstValue::UInt(value) => assert_eq!(*value, 4),
        other => panic!("expected uint constant, found {other:?}"),
    }

    let variable = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("SizeOfVariable"))
        .expect("missing SizeOfVariable function");
    let variable_const =
        extract_return_constant(&variable.body).expect("expected constant return for sizeof value");
    match variable_const {
        ConstValue::UInt(value) => assert_eq!(*value, 4),
        other => panic!("expected uint constant, found {other:?}"),
    }
}

#[test]
fn reports_diagnostic_for_unknown_type() {
    let source = r#"
namespace Sample;

public usize SizeOfUnknown()
{
    return sizeof(UnknownType);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("cannot determine size for type `UnknownType`"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn reports_diagnostic_for_non_variable_operand() {
    let source = r#"
namespace Sample;

public usize SizeOfExpression(int value)
{
    return sizeof(value + 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("expects a type or local variable"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn lowers_alignof_for_types_and_variables() {
    let source = r#"
namespace Sample;

public struct Pair
{
    public long A;
    public long B;
}

public usize AlignOfPrimitive()
{
    return alignof(int);
}

public usize AlignOfStruct()
{
    return alignof(Pair);
}

public usize AlignOfVariable(Pair value)
{
    return alignof value;
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let primitive = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("AlignOfPrimitive"))
        .expect("missing AlignOfPrimitive function");
    let primitive_const = extract_return_constant(&primitive.body)
        .expect("expected constant return for alignof(int)");
    match primitive_const {
        ConstValue::UInt(value) => assert_eq!(*value, 4),
        other => panic!("expected uint constant, found {other:?}"),
    }

    let pair = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("AlignOfStruct"))
        .expect("missing AlignOfStruct function");
    let pair_const =
        extract_return_constant(&pair.body).expect("expected constant return for alignof(Pair)");
    match pair_const {
        ConstValue::UInt(value) => assert_eq!(*value, 8),
        other => panic!("expected uint constant, found {other:?}"),
    }

    let variable = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("AlignOfVariable"))
        .expect("missing AlignOfVariable function");
    let variable_const = extract_return_constant(&variable.body)
        .expect("expected constant return for alignof value");
    match variable_const {
        ConstValue::UInt(value) => assert_eq!(*value, 8),
        other => panic!("expected uint constant, found {other:?}"),
    }
}

#[test]
fn reports_alignof_diagnostic_for_unknown_type() {
    let source = r#"
namespace Sample;

public usize AlignOfUnknown()
{
    return alignof(UnknownType);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("cannot determine alignment for type `UnknownType`"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn reports_alignof_diagnostic_for_non_variable_operand() {
    let source = r#"
namespace Sample;

public usize AlignOfExpression(int value)
{
    return alignof(value + 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("expects a type or local variable"),
        "unexpected diagnostic: {message}"
    );
}
