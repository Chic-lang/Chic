use super::*;

#[test]
fn parses_designated_constructor_with_super_initializer() {
    let source = r"
public class Button : Control
{
    public init(int size) : super(size) { }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    assert_eq!(class.members.len(), 1);
    let ctor = class
        .members
        .iter()
        .find_map(|member| match member {
            ClassMember::Constructor(ctor) => Some(ctor),
            _ => None,
        })
        .expect("class should contain constructor");
    assert_eq!(ctor.kind, ConstructorKind::Designated);
    assert_eq!(ctor.parameters.len(), 1);
    assert_eq!(ctor.parameters[0].name, "size");
    let initializer = ctor
        .initializer
        .as_ref()
        .expect("expected super initializer");
    assert_eq!(initializer.target, ConstructorInitTarget::Super);
    assert_eq!(initializer.arguments.len(), 1);
    assert_eq!(initializer.arguments[0].text.trim(), "size");
    assert!(ctor.body.is_some());
}

#[test]
fn parses_convenience_constructor_with_self_initializer() {
    let source = r"
public class Widget
{
    public init(int value) { }

    public convenience init() : self(0) { }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    assert_eq!(class.members.len(), 2);
    let ctor = match &class.members[1] {
        ClassMember::Constructor(ctor) => ctor,
        other => panic!("expected constructor, found {other:?}"),
    };
    assert_eq!(ctor.kind, ConstructorKind::Convenience);
    let initializer = ctor
        .initializer
        .as_ref()
        .expect("expected self initializer");
    assert_eq!(initializer.target, ConstructorInitTarget::SelfType);
    assert_eq!(initializer.arguments.len(), 1);
    assert_eq!(initializer.arguments[0].text.trim(), "0");
}

#[test]
fn rejects_convenience_constructor_without_delegation() {
    let source = r"
public class Image
{
    public convenience init(int width, int height) { }
}
";

    let err = parse_module(source).expect_err("expected convenience delegation diagnostic");
    assert!(err.diagnostics().iter().any(|diag| {
        diag.message
            .contains("must delegate to another initializer")
    }));
}

#[test]
fn rejects_type_named_constructor_syntax() {
    let source = r"
public class Widget
{
    public Widget() { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0C01")),
        "expected type-named constructor diagnostic code E0C01, found {:?}",
        diagnostics
    );
}

#[test]
fn class_method_matching_type_name_reports_error() {
    let source = r"
public class Widget
{
    public int Widget() { return 1; }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0C02")),
        "expected class method type-name diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn constructor_parameters_capture_default_expression_metadata() {
    let source = r#"
public class Panel
{
    private static string DefaultTitle() { return "Panel"; }

    public init(int width, string title = DefaultTitle()) { }
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let ctor = class
        .members
        .iter()
        .find_map(|member| match member {
            ClassMember::Constructor(ctor) => Some(ctor),
            _ => None,
        })
        .expect("expected constructor in class members");
    assert_eq!(ctor.parameters.len(), 2);
    let default = ctor.parameters[1]
        .default
        .as_ref()
        .expect("expected default on second parameter");
    assert_eq!(default.text.trim(), "DefaultTitle()");
    let Some(node) = default.node.as_ref() else {
        panic!("default should retain parsed node");
    };
    assert!(matches!(node, ExprNode::Call { .. }));
}
