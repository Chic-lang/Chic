use super::*;

#[test]
fn parses_function_with_generic_types_in_signature() {
    let source = r"
namespace Geometry;

public void Process(ref Span<int> data)
{
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let func = match &module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}"),
    };
    assert_eq!(func.signature.parameters.len(), 1);
    assert!(matches!(
        func.signature.parameters[0].binding,
        BindingModifier::Ref
    ));
    let body = function_body(func);
    assert!(body.statements.is_empty());
}

#[test]
fn rejects_generic_operator_signature() {
    let source = r#"
namespace Sample;

public struct MyNumber { }

public class Broken
{
    public static MyNumber operator +<T>(MyNumber lhs, MyNumber rhs);
}
"#;

    let err = parse_module(source).expect_err("expected generic parameter diagnostic");
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("operator overloads cannot declare generic parameters")),
        "expected generic parameter diagnostic, found {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_struct_with_generic_field() {
    let source = r"
namespace Geometry;

public struct Container
{
    internal Span<List<int>> Data;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    let structure = match &module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected Item::Struct, found {other:?}"),
    };
    assert_eq!(structure.fields[0].ty.name, "Span<List<int>>");
}

#[test]
fn parses_struct_with_generic_type_parameters() {
    let source = r"
namespace Geometry;

public struct Wrapper<T, U>
    where T : struct
    where U : class
{
    public T First;
    public U Second;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    let structure = match &module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected Item::Struct, found {other:?}"),
    };

    let generics = structure
        .generics
        .as_ref()
        .expect("expected Wrapper to record generics");
    assert_eq!(generics.params.len(), 2);
    assert_eq!(generics.params[0].name, "T");
    assert_eq!(generics.params[1].name, "U");
}

#[test]
fn parses_nested_generic_structs() {
    let source = r"
namespace Demo;

public struct Outer<T>
{
    public struct Node<U>
    {
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Struct(outer) = &parse.module.items[0] else {
        panic!("expected Item::Struct for Outer");
    };
    assert_eq!(outer.name, "Outer");
    assert!(outer.generics.is_some(), "outer generics missing");
    assert_eq!(outer.nested_types.len(), 1);

    let nested = match &outer.nested_types[0] {
        Item::Struct(inner) => inner,
        other => panic!("expected nested struct, found {other:?}"),
    };
    assert_eq!(nested.name, "Node");
    assert!(nested.generics.is_some(), "nested generics missing");
}
