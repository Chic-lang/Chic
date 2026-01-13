use super::common::RequireExt;
use super::*;

fn extract_string_constant(body: &MirBody) -> Option<String> {
    for block in &body.blocks {
        for statement in &block.statements {
            if let StatementKind::Assign { place, value } = &statement.kind {
                if place.local == LocalId(0) {
                    if let Rvalue::Use(Operand::Const(constant)) = value {
                        if let ConstValue::Str { value, .. } = &constant.value {
                            return Some(value.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

#[test]
fn lowers_nameof_to_string_constant() {
    let source = r#"
namespace Sample;

public struct Point
{
    public int X;
    public int Y;
}

public enum Direction
{
    North,
    South,
}

public class Utilities
{
    public static void Compute() {}
}

public str FieldName()
{
    return nameof(Point.X);
}

public str TypeName()
{
    return nameof(Point);
}

public str LocalName(int value)
{
    return nameof(value);
}

public str MethodName()
{
    return nameof(Utilities.Compute);
}

public str VariantName()
{
    return nameof(Direction.North);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let mut assertions = vec![
        ("FieldName", "X"),
        ("TypeName", "Point"),
        ("LocalName", "value"),
        ("MethodName", "Compute"),
        ("VariantName", "North"),
    ];

    for (suffix, expected) in assertions.drain(..) {
        let function = lowering
            .module
            .functions
            .iter()
            .find(|func| func.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing {suffix} function"));
        let value = extract_string_constant(&function.body)
            .unwrap_or_else(|| panic!("expected constant return for {suffix}"));
        assert_eq!(value, expected, "unexpected value for {suffix}");
    }
}

#[test]
fn reports_diagnostic_for_unknown_symbol() {
    let source = r#"
namespace Sample;

public string UnknownName()
{
    return nameof(DoesNotExist);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(lowering.diagnostics.len(), 1);
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("cannot resolve symbol"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn reports_diagnostic_for_unknown_member() {
    let source = r#"
namespace Sample;

public struct Point
{
    public int X;
}

public string UnknownMember()
{
    return nameof(Point.Y);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(lowering.diagnostics.len(), 1);
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("does not contain member"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn reports_diagnostic_for_overload_group() {
    let source = r#"
namespace Sample;

public class Math
{
    public static void Hypot(int value) {}
    public static void Hypot(double value) {}
}

public string Ambiguous()
{
    return nameof(Math.Hypot);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert_eq!(lowering.diagnostics.len(), 1);
    let message = &lowering.diagnostics[0].message;
    assert!(
        message.contains("resolves to"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn reports_diagnostic_for_complex_expression() {
    let source = r#"
namespace Sample;

public string NotSimple(int value)
{
    return nameof(value + 1);
}
"#;

    let Err(error) = parse_module(source) else {
        panic!("expected parse failure for complex nameof operand");
    };
    assert_eq!(error.diagnostics().len(), 1);
    let message = &error.diagnostics()[0].message;
    assert!(
        message.contains("unexpected token in `nameof` operand"),
        "unexpected diagnostic: {message}"
    );
}
