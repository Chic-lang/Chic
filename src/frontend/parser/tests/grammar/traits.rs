use crate::frontend::ast::{InterfaceMember, Item};
use crate::frontend::parser::tests::fixtures::parse_ok;

#[test]
fn parses_interface_with_associated_type_and_methods() {
    let source = r#"
public interface Iterator<T>
{
    type Item<TSelf>;
    T Next();
    type Default = T;
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Interface(iface_decl) = &parse.module.items[0] else {
        panic!("expected interface item");
    };
    assert_eq!(iface_decl.name, "Iterator");
    assert_eq!(iface_decl.members.len(), 3);

    match &iface_decl.members[0] {
        InterfaceMember::AssociatedType(assoc) => {
            assert_eq!(assoc.name, "Item");
            assert!(assoc.default.is_none());
        }
        other => panic!("expected associated type, found {other:?}"),
    }

    match &iface_decl.members[1] {
        InterfaceMember::Method(method) => {
            assert_eq!(method.name, "Next");
            assert!(
                method.body.is_none(),
                "interface method should default to declaration"
            );
        }
        other => panic!("expected method, found {other:?}"),
    }

    match &iface_decl.members[2] {
        InterfaceMember::AssociatedType(assoc) => {
            assert_eq!(assoc.name, "Default");
            assert!(assoc.default.is_some(), "expected default type assignment");
        }
        other => panic!("expected associated type with default, found {other:?}"),
    }
}

#[test]
fn parses_interface_members_with_bodies() {
    let source = r#"
public interface Formatter
{
    type Output = string;
    string Format() { return; }
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Interface(decl) = &parse.module.items[0] else {
        panic!("expected interface item");
    };
    assert_eq!(decl.members.len(), 2);

    match &decl.members[0] {
        InterfaceMember::AssociatedType(assoc) => {
            assert_eq!(assoc.name, "Output");
            assert!(
                assoc.default.is_some(),
                "interface associated type with default should be recorded"
            );
        }
        other => panic!("expected associated type assignment, found {other:?}"),
    }

    match &decl.members[1] {
        InterfaceMember::Method(method) => {
            assert_eq!(method.name, "Format");
            assert!(method.body.is_some(), "interface method can provide a body");
        }
        other => panic!("expected method implementation, found {other:?}"),
    }
}
