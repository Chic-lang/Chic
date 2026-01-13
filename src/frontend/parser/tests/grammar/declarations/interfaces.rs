use super::*;

#[test]
fn required_modifier_rejected_on_interface_property() {
    let source = r"
public interface IThing
{
    public required int Value { get; }
}
";

    let err = parse_module(source).expect_err("expected interface required modifier diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`required` modifier is not supported on interface members")),
        "expected required modifier diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_interface_members_without_bodies() {
    let source = r"
namespace Geometry;

public interface IShape
{
    double Area(in this);
    void Move(ref this, int dx, int dy);
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let interface = match &module.items[0] {
        Item::Interface(decl) => decl,
        other => panic!("expected Item::Interface, found {other:?}"),
    };
    for member in &interface.members {
        match member {
            InterfaceMember::Method(func) => assert!(func.body.is_none()),
            InterfaceMember::Property(prop) => {
                panic!("expected method, found property {prop:?}")
            }
            InterfaceMember::Const(constant) => {
                panic!("expected method, found const member {constant:?}")
            }
            InterfaceMember::AssociatedType(ty) => {
                panic!("expected method, found associated type {ty:?}")
            }
        }
    }
}

#[test]
fn parses_interface_method_with_generics() {
    let source = r"
namespace Geometry;

public interface IResolver
{
    T Resolve<T>(string name);
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let interface = match &module.items[0] {
        Item::Interface(decl) => decl,
        other => panic!("expected Item::Interface, found {other:?}"),
    };
    let method = match &interface.members[0] {
        InterfaceMember::Method(method) => method,
        other => panic!("expected method member, found {other:?}"),
    };
    assert!(method.generics.is_some(), "expected generic parameters");
}

#[test]
fn parses_interface_default_method_body() {
    let source = r#"
namespace Geometry;

public interface IWidget
{
    void Draw(ref this) { }
    double Area(in this) => 0.0;
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let interface = match &parse.module.items[0] {
        Item::Interface(decl) => decl,
        other => panic!("expected interface, found {other:?}"),
    };
    assert_eq!(interface.members.len(), 2);
    for member in &interface.members {
        if let InterfaceMember::Method(method) = member {
            assert!(
                method.body.is_some(),
                "expected default body to be captured for {:?}",
                method.name
            );
        }
    }
}

#[test]
fn interface_pin_attribute_is_rejected() {
    let source = r"
@pin
public interface IShape {}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@pin") && diag.message.contains("variable")),
        "expected interface pin diagnostic, found {:?}",
        diagnostics
    );
}
