use super::*;

#[test]
fn parses_virtual_method_dispatch() {
    let source = r#"
namespace Geometry;

public class Widget
{
    public virtual void Render() {}
}
"#;
    let module = parse_ok(source).module;
    let class = match &module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class item, found {other:?}"),
    };
    let method = match &class.members[0] {
        ClassMember::Method(func) => func,
        other => panic!("expected method member, found {other:?}"),
    };
    assert!(method.dispatch.is_virtual, "expected method to be virtual");
    assert!(
        !method.dispatch.is_override,
        "virtual method should not be override"
    );
    assert!(
        !method.dispatch.is_sealed,
        "virtual method should not be sealed"
    );
    assert!(
        !method.dispatch.is_abstract,
        "virtual method should not be abstract"
    );
}

#[test]
fn parses_sealed_override_dispatch() {
    let source = r#"
namespace Geometry;

public class Base
{
    public virtual void Render() {}
}

public class Derived : Base
{
    public sealed override void Render() {}
}
"#;
    let module = parse_ok(source).module;
    let derived = match &module.items[1] {
        Item::Class(class) => class,
        other => panic!("expected class item, found {other:?}"),
    };
    let method = match &derived.members[0] {
        ClassMember::Method(func) => func,
        other => panic!("expected method member, found {other:?}"),
    };
    assert!(method.dispatch.is_override, "override flag missing");
    assert!(method.dispatch.is_sealed, "sealed flag missing");
    assert!(
        !method.dispatch.is_virtual,
        "override method should not be marked virtual"
    );
}

#[test]
fn property_dispatch_applies_to_accessors() {
    let source = r#"
namespace Geometry;

public abstract class Descriptor
{
    public abstract virtual string Name { get; set; }
}
"#;
    let module = parse_ok(source).module;
    let class = match &module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class item, found {other:?}"),
    };
    let property = match &class.members[0] {
        ClassMember::Property(prop) => prop,
        other => panic!("expected property member, found {other:?}"),
    };
    assert!(property.dispatch.is_virtual, "property should be virtual");
    assert!(property.dispatch.is_abstract, "property should be abstract");
    for accessor in &property.accessors {
        assert!(
            accessor.dispatch.is_virtual,
            "accessor should inherit virtual dispatch"
        );
        assert!(
            accessor.dispatch.is_abstract,
            "accessor should inherit abstract dispatch"
        );
    }
}

#[test]
fn accessor_specific_dispatch_overrides_property_defaults() {
    let source = r#"
namespace Geometry;

public class Derived
{
    public string Label { override get; sealed set; }
}
"#;
    let module = parse_ok(source).module;
    let class = match &module.items[0] {
        Item::Class(class) => class,
        _ => panic!("expected class item"),
    };
    let property = match &class.members[0] {
        ClassMember::Property(prop) => prop,
        _ => panic!("expected property"),
    };
    assert!(
        !property.dispatch.is_override,
        "property should not be override by default"
    );
    let get_accessor = property
        .accessors
        .iter()
        .find(|accessor| matches!(accessor.kind, PropertyAccessorKind::Get))
        .expect("expected get accessor");
    assert!(
        get_accessor.dispatch.is_override,
        "get accessor should be override"
    );
    let set_accessor = property
        .accessors
        .iter()
        .find(|accessor| matches!(accessor.kind, PropertyAccessorKind::Set))
        .expect("expected set accessor");
    assert!(
        set_accessor.dispatch.is_sealed,
        "set accessor should be sealed"
    );
}

#[test]
fn rejects_virtual_on_static_methods() {
    let source = r#"
namespace Geometry;

public class Widget
{
    public static virtual void Render() {}
}
"#;
    let err = parse_module(source).expect_err("expected diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("static methods")),
        "expected static method diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_sealed_without_override() {
    let source = r#"
namespace Geometry;

public class Widget
{
    public sealed void Render() {}
}
"#;
    let err = parse_module(source).expect_err("expected sealed diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("requires `override`")),
        "expected sealed diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_virtual_on_fields() {
    let source = r#"
namespace Geometry;

public class Widget
{
    public virtual int Count;
}
"#;
    let err = parse_module(source).expect_err("expected field diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("fields")),
        "expected field diagnostic, found {:?}",
        err.diagnostics()
    );
}
