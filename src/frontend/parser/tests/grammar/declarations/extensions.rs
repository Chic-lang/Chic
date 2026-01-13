use super::*;

#[test]
fn parses_extension_methods() {
    let source = r#"
namespace Geometry;

public extension Point
{
    public string ToText(in this) => "";
    public void Translate(ref this, int dx, int dy) { }
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let extension = match &module.items[0] {
        Item::Extension(ext) => ext,
        other => panic!("expected Item::Extension, found {other:?}"),
    };
    match &extension.members[1] {
        ExtensionMember::Method(method) => {
            let body = function_body(&method.function);
            assert!(body.statements.is_empty());
        }
    }
}

#[test]
fn parses_extension_member_without_body() {
    let source = r#"
namespace Geometry;

public struct Logger { }

public extension Logger
{
    public void Flush();
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let extension = match &module.items[1] {
        Item::Extension(ext) => ext,
        other => panic!("expected Item::Extension, found {other:?}"),
    };
    assert_eq!(extension.members.len(), 1);
    match &extension.members[0] {
        ExtensionMember::Method(method) => {
            assert!(
                method.function.body.is_none(),
                "extension method without body should parse as declaration"
            );
            assert!(
                !method.is_default,
                "non-default method should not be tagged default"
            );
        }
    }
}

#[test]
fn captures_doc_comments_for_extension_and_testcase() {
    let source = r"
/// Extension docs.
public extension Point { }

/// Testcase docs.
testcase Runs()
{
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.items.len(), 2);

    let extension = match &parse.module.items[0] {
        Item::Extension(ext) => ext,
        other => panic!("expected extension, found {other:?}"),
    };
    assert_eq!(
        extension.doc.as_ref().map(|doc| doc.as_text()).as_deref(),
        Some("Extension docs."),
    );

    let testcase = match &parse.module.items[1] {
        Item::TestCase(case) => case,
        other => panic!("expected testcase, found {other:?}"),
    };
    assert_eq!(
        testcase.doc.as_ref().map(|doc| doc.as_text()).as_deref(),
        Some("Testcase docs."),
    );
}

#[test]
fn parses_default_extension_method_with_body() {
    let source = r#"
namespace Geometry;

public interface IShape { }

public extension IShape
{
    public default double Perimeter(in this) => 0;
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let extension = match &parse.module.items[1] {
        Item::Extension(ext) => ext,
        other => panic!("expected extension, found {other:?}"),
    };
    match &extension.members[0] {
        ExtensionMember::Method(method) => {
            assert!(method.is_default, "expected default flag");
            assert!(
                method.function.body.is_some(),
                "default method should capture its body"
            );
        }
    }
    assert!(extension.conditions.is_empty(), "no conditions expected");
}

#[test]
fn default_extension_method_without_body_reports_error() {
    let source = r#"
public struct Point { }

public extension Point
{
    public default void Missing();
}
"#;

    let err = parse_module(source).expect_err("expected diagnostic for missing body");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`default` extension methods must provide a body")),
        "expected missing body diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_extension_when_clause_with_conditions() {
    let source = r#"
public struct Shape { }
public interface IRenderable { }
public interface ITransient { }

public extension Shape when Self : IRenderable, Self : ITransient
{
    public default void Render(in this) { }
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let extension = match &parse.module.items[3] {
        Item::Extension(ext) => ext,
        other => panic!("expected extension, found {other:?}"),
    };
    assert_eq!(extension.conditions.len(), 2);
    assert!(extension.members.len() == 1);
    match &extension.members[0] {
        ExtensionMember::Method(method) => assert!(method.is_default),
    }
    let first = &extension.conditions[0];
    assert_eq!(first.target.name, "Self");
    assert_eq!(first.constraint.name, "IRenderable");
    let second = &extension.conditions[1];
    assert_eq!(second.constraint.name, "ITransient");
}
