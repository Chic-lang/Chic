use super::*;

#[test]
fn parses_class_and_extension_operator_overloads() {
    let source = r#"
namespace Numbers;

public struct MyNumber { }

public class MathOps
{
    public static MyNumber operator +(MyNumber lhs, MyNumber rhs);
    public static MyNumber operator -(MyNumber value);
    public static implicit operator MyNumber(int value);
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    assert_eq!(module.items.len(), 2);
    let number_class = match &module.items[1] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };

    let add_overload = number_class
        .members
        .iter()
        .filter_map(|member| {
            if let ClassMember::Method(func) = member {
                (func.name == "op_Addition").then_some(func)
            } else {
                None
            }
        })
        .next()
        .expect("expected binary operator overload");
    let add_meta = add_overload
        .operator
        .as_ref()
        .expect("operator metadata missing for binary overload");
    assert!(matches!(
        add_meta.kind,
        OperatorKind::Binary(BinaryOperator::Add)
    ));
    assert_eq!(add_overload.signature.parameters.len(), 2);

    let negate_overload = number_class
        .members
        .iter()
        .filter_map(|member| {
            if let ClassMember::Method(func) = member {
                (func.name == "op_UnaryNegation").then_some(func)
            } else {
                None
            }
        })
        .next()
        .expect("expected unary operator overload");
    let negate_meta = negate_overload
        .operator
        .as_ref()
        .expect("operator metadata missing for unary overload");
    assert!(matches!(
        negate_meta.kind,
        OperatorKind::Unary(UnaryOperator::Negate)
    ));
    assert_eq!(negate_overload.signature.parameters.len(), 1);

    let conversion_overload = number_class
        .members
        .iter()
        .filter_map(|member| {
            if let ClassMember::Method(func) = member {
                (func.name == "op_Implicit_MyNumber").then_some(func)
            } else {
                None
            }
        })
        .next()
        .expect("expected conversion operator overload");
    let conversion_meta = conversion_overload
        .operator
        .as_ref()
        .expect("operator metadata missing for conversion overload");
    assert!(matches!(
        conversion_meta.kind,
        OperatorKind::Conversion(ConversionKind::Implicit)
    ));
    assert_eq!(conversion_overload.signature.parameters.len(), 1);
}

#[test]
fn rejects_operator_without_static_modifier() {
    let source = r#"
namespace Sample;

public struct MyNumber { }

public class Broken
{
    public MyNumber operator +(MyNumber lhs, MyNumber rhs);
}
"#;

    let err = parse_module(source).expect_err("expected static modifier diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("must be declared `static`")),
        "expected static modifier diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_additional_unary_operator_symbols() {
    let source = r#"
namespace Numbers;

public struct MyNumber { }

public class MoreOps
{
    public static MyNumber operator ~(MyNumber value);
    public static MyNumber operator ++(MyNumber value);
    public static MyNumber operator +(MyNumber value);
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[1] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let mut seen = 0usize;
    for member in &class.members {
        if let ClassMember::Method(func) = member {
            if let Some(op) = &func.operator {
                match op.kind {
                    OperatorKind::Unary(UnaryOperator::OnesComplement) => seen |= 1,
                    OperatorKind::Unary(UnaryOperator::Increment) => seen |= 2,
                    OperatorKind::Unary(UnaryOperator::UnaryPlus) => seen |= 4,
                    _ => {}
                }
            }
        }
    }
    assert_eq!(
        seen, 0b111,
        "expected all unary operator kinds to be parsed"
    );
}

#[test]
fn rejects_non_overloadable_operator_symbols() {
    let source = r#"
namespace Numbers;

public struct MyNumber { }

public class Broken
{
    public static MyNumber operator &&(MyNumber lhs, MyNumber rhs);
}
"#;

    let err = parse_module(source).expect_err("expected non-overloadable operator diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("not a supported overloadable operator")),
        "expected non-overloadable diagnostic, found {:?}",
        err.diagnostics()
    );
}
