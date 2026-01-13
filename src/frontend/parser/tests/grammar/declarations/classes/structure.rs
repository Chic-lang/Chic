use super::*;
use crate::syntax::expr::ExprNode;

#[test]
fn parses_class_with_members() {
    let source = r"
namespace Geometry;

public class Circle : IShape
{
    public double Radius;

    public double Area(in this) => 3.1415 * Radius * Radius;
    public void Move(ref this, int dx, int dy) { }
    public void dispose(ref this) { }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let module = parse.module;
    let class = match &module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected Item::Class, found {other:?}"),
    };
    assert_eq!(class.members.len(), 4);
    match &class.members[2] {
        ClassMember::Method(func) => {
            let body = function_body(func);
            assert!(body.statements.is_empty());
        }
        ClassMember::Field(field) => panic!("expected method, found field {field:?}"),
        ClassMember::Constructor(ctor) => panic!("expected method, found constructor {ctor:?}"),
        ClassMember::Property(prop) => panic!("expected method, found property {prop:?}"),
        ClassMember::Const(constant) => {
            panic!("expected method, found const member {constant:?}")
        }
    }
    match &class.members[3] {
        ClassMember::Method(func) => {
            let body = function_body(func);
            assert!(body.statements.is_empty());
        }
        ClassMember::Field(field) => panic!("expected method, found field {field:?}"),
        ClassMember::Constructor(ctor) => panic!("expected method, found constructor {ctor:?}"),
        ClassMember::Property(prop) => panic!("expected method, found property {prop:?}"),
        ClassMember::Const(constant) => {
            panic!("expected method, found const member {constant:?}")
        }
    }
}

#[test]
fn class_method_accepts_generic_return_type_and_method_generics() {
    let source = r"
namespace Demo;

public static class Vec
{
    public static Span<T> AsSpan<T>(ref VecPtr vec)
    {
        return 0;
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Class(class) = &parse.module.items[0] else {
        panic!("expected class item, found {:?}", parse.module.items[0]);
    };
    assert_eq!(class.name, "Vec");
    let ClassMember::Method(method) = &class.members[0] else {
        panic!("expected method member, found {:?}", class.members[0]);
    };
    assert_eq!(method.signature.parameters.len(), 1);
    assert_eq!(method.generics.as_ref().map(|g| g.params.len()), Some(1));
    assert_eq!(method.signature.return_type.base, vec!["Span".to_string()]);
    let return_args = method
        .signature
        .return_type
        .generic_arguments()
        .expect("return type should record generic args");
    assert_eq!(return_args.len(), 1);
    assert_eq!(return_args[0].ty().expect("type arg").name, "T");
}

#[test]
fn class_method_accepts_doc_comment_with_default_parameter() {
    let source = r"
namespace Demo;

public class Math
{
    /// Adds two integers with default rounding.
    public int Add(int lhs, int rhs = supportsSimd ? 2 : 1)
    {
        return lhs + rhs;
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Class(class) = &parse.module.items[0] else {
        panic!("expected class item, found {:?}", parse.module.items[0]);
    };
    let ClassMember::Method(method) = &class.members[0] else {
        panic!("expected class method, found {:?}", class.members[0]);
    };
    let doc = method
        .doc
        .as_ref()
        .expect("method should capture doc comment")
        .as_text();
    assert!(
        doc.contains("Adds two integers"),
        "doc comment mismatch: {doc:?}"
    );
    assert_eq!(method.signature.parameters.len(), 2);
    let rhs_param = &method.signature.parameters[1];
    let default = rhs_param
        .default
        .as_ref()
        .expect("rhs parameter should record default expression");
    assert_eq!(
        default.text.trim(),
        "supportsSimd ? 2 : 1",
        "default expression mismatch"
    );
    match default.node.as_ref().expect("default expression parsed") {
        ExprNode::Conditional { .. } => {}
        other => panic!("expected conditional expression, found {other:?}"),
    }
}
