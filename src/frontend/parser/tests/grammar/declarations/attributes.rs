use super::*;
use crate::frontend::ast::{ExternBinding, ImportKind};

#[test]
fn parses_extern_function_with_attributes() {
    let source = r#"
namespace Interop;

@cimport("stdio.h")
@extern("C")
@link("c")
public extern int puts(int value);
"#;
    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    assert_eq!(module.namespace.as_deref(), Some("Interop"));
    assert_eq!(module.items.len(), 2, "expected cimport + extern function");
    match &module.items[0] {
        Item::Import(import) => match &import.kind {
            ImportKind::CImport { header } => assert_eq!(header, "stdio.h"),
            other => panic!("expected cimport directive, found {other:?}"),
        },
        other => panic!("expected import directive, found {other:?}"),
    }
    let func = match &module.items[1] {
        Item::Function(func) => func,
        other => panic!("expected extern function, found {other:?}"),
    };
    assert!(func.is_extern, "function should be marked extern");
    assert_eq!(func.extern_abi.as_deref(), Some("C"));
    assert_eq!(func.link_library.as_deref(), Some("c"));
    assert!(
        func.body.is_none(),
        "extern function should not have a body"
    );
    let options = func
        .extern_options
        .as_ref()
        .expect("extern function should record options");
    assert_eq!(options.convention, "C");
    assert!(options.library.is_none());
    assert_eq!(options.binding, ExternBinding::Static);
    assert!(!options.optional);
}

#[test]
fn rejects_unknown_extern_abi() {
    let source = r#"
@extern("bad_abi")
public extern void Foo();
"#;
    let err = parse_module(source).expect_err("expected ABI diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("unsupported ABI")),
        "expected unsupported ABI diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_extended_extern_attribute_metadata() {
    let source = r#"
@extern("system", library = "user32", alias = "MessageBoxW", binding = "eager", optional = true)
public extern int MessageBox(void* hwnd, string text, string caption, uint kind);
"#;
    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let func = match &parse.module.items[0] {
        Item::Function(func) => func,
        other => panic!("expected extern function, found {other:?}"),
    };
    assert!(func.is_extern);
    assert_eq!(func.extern_abi.as_deref(), Some("system"));
    let options = func
        .extern_options
        .as_ref()
        .expect("expected extern options to be recorded");
    assert_eq!(options.convention, "system");
    assert_eq!(options.library.as_deref(), Some("user32"));
    assert_eq!(options.alias.as_deref(), Some("MessageBoxW"));
    assert_eq!(options.binding, ExternBinding::Eager);
    assert!(options.optional);
}

#[test]
fn rejects_unknown_extern_binding() {
    let source = r#"
@extern(binding = "unknown", library = "foo")
public extern void Foo();
"#;
    let err = parse_module(source).expect_err("expected binding diagnostic");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("binding must be")),
        "expected binding diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn extern_static_binding_with_library_reports_error() {
    let source = r#"
@extern(binding = "static", library = "foo")
public extern void Foo();
"#;
    let err = parse_module(source).expect_err("expected binding/library diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`binding=\"static\"` may only be used without a `library`")),
        "expected static binding diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_flags_enum_with_discriminants() {
    let source = r"
namespace Demo;

@flags
public enum Mode
{
    None = 0,
    Read = 1,
    Write = Read << 1,
    ReadWrite = Read | Write,
}
";
    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    let enum_decl = match &module.items[0] {
        Item::Enum(enm) => enm,
        other => panic!("expected flags enum, found {other:?}"),
    };
    assert!(enum_decl.is_flags, "expected `is_flags` to be set");
    assert_eq!(enum_decl.variants.len(), 4);
    assert_eq!(enum_decl.variants[0].name, "None");
    let none_expr = enum_decl.variants[0]
        .discriminant
        .as_ref()
        .expect("expected discriminant for `None`");
    assert!(
        matches!(none_expr.node.as_ref(), Some(ExprNode::Literal(_))),
        "expected literal expression for `None`, found {:?}",
        none_expr.node
    );
    assert_eq!(enum_decl.variants[3].name, "ReadWrite");
    assert_eq!(
        enum_decl.variants[3]
            .discriminant
            .as_ref()
            .map(|expr| expr.text.trim()),
        Some("Read | Write"),
        "expected infix expression for composite flag"
    );
}

#[test]
fn rejects_flags_attribute_on_structs() {
    let source = r"
namespace Demo;

@flags
public struct Invalid { }
";
    let err = parse_module(source).expect_err("expected flags attribute diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`@flags` attribute is only supported on enum declarations")),
        "expected flags misuse diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn reports_dangling_attribute_at_end_of_file() {
    let source = r"
@thread_safe
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("attributes are not supported")),
        "expected attribute misuse diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn attributes_not_supported_on_extension_declarations() {
    let source = r"
@thread_safe
public extension Point { }
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("attributes are not supported")),
        "expected extension attribute diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn allows_attributes_on_testcase_declarations() {
    let source = r"
@category(smoke)
@id(foo-123)
testcase Runs()
{
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let testcase = match &parse.module.items[0] {
        Item::TestCase(testcase) => testcase,
        other => panic!("expected testcase item, found {other:?}"),
    };
    let names: Vec<_> = testcase
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(names, ["category", "id"]);
}

#[test]
fn captures_doc_comments_for_items() {
    let source = r#"
namespace Geometry;

/// Represents the geometry module.
public class GeometryModule
{
}

/// Represents a point.
public struct Point
{
    /// X coordinate.
    public double X;
}
"#;

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    assert_eq!(module.items.len(), 2);

    match &module.items[0] {
        Item::Class(def) => {
            let doc = def.doc.as_ref().expect("class doc missing");
            assert_eq!(
                doc.lines,
                vec!["Represents the geometry module.".to_string()]
            );
        }
        other => panic!("expected class item, found {other:?}"),
    }

    match &module.items[1] {
        Item::Struct(def) => {
            let doc = def.doc.as_ref().expect("struct doc missing");
            assert_eq!(doc.lines, vec!["Represents a point.".to_string()]);
            let field_doc = def.fields[0].doc.as_ref().expect("field doc missing");
            assert_eq!(field_doc.lines, vec!["X coordinate.".to_string()]);
        }
        other => panic!("expected struct item, found {other:?}"),
    }
}
