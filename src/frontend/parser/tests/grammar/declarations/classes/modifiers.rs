use super::*;

#[test]
fn parses_static_class_with_extern_method() {
    let source = r#"
namespace Std.Collections;

public static class VecIntrinsics
{
    @extern("C")
    @link("chic_rt_vec")
    public static extern VecPtr New(int initialCapacity);
}
"#;
    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    assert_eq!(
        module.namespace.as_deref(),
        Some("Std.Collections"),
        "namespace should be recorded"
    );
    let class = match &module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class item, found {other:?}"),
    };
    assert!(class.is_static, "class should be marked static");
    assert_eq!(class.members.len(), 1, "expected single member");

    let method = match &class.members[0] {
        ClassMember::Method(func) => func,
        other => panic!("expected method member, found {other:?}"),
    };
    assert!(
        method
            .modifiers
            .iter()
            .any(|m| m.eq_ignore_ascii_case("static")),
        "expected method modifiers to retain `static`, found {:?}",
        method.modifiers
    );
    assert!(method.is_extern, "method should be marked extern");
    assert_eq!(
        method.extern_abi.as_deref(),
        Some("C"),
        "extern ABI should respect attribute argument"
    );
    assert_eq!(
        method.link_library.as_deref(),
        Some("chic_rt_vec"),
        "link attribute should propagate to method"
    );
    assert!(
        method.body.is_none(),
        "extern method should not have a body"
    );
}

#[test]
fn rejects_async_modifier_on_class() {
    let source = r"
namespace Geometry;

public async class Runner {}
";

    let err = parse_module(source).expect_err("expected async class diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`async` modifier is not supported on class declarations")),
        "expected async modifier diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_unsupported_class_modifiers() {
    let source = r"
namespace Geometry;

public noreturn override class Broken {}
";

    let err = parse_module(source).expect_err("expected unsupported class modifiers");
    let messages: Vec<String> = err
        .diagnostics()
        .iter()
        .map(|diag| diag.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("modifier `noreturn`")),
        "expected noreturn diagnostic, found {:?}",
        err.diagnostics()
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("modifier `override`")),
        "expected override diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn class_pin_attribute_is_rejected() {
    let source = r"
@pin
public class Widget {}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@pin") && diag.message.contains("variable")),
        "expected class pin diagnostic, found {:?}",
        diagnostics
    );
}
