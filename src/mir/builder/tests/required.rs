use super::*;
use crate::frontend::ast::{
    Block, ClassDecl, ClassKind, ClassMember, Expression, FieldDecl, FunctionDecl, Item,
    MemberDispatch, Module, PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind,
    PropertyDecl, Signature, Statement, StatementKind as AstStatementKind, StructDecl, TypeExpr,
    Visibility,
};
use crate::syntax::expr::ExprNode;
use crate::syntax::expr::builders::LiteralConst;

fn initializer_expression(text: &str) -> Expression {
    Expression::with_node(
        text,
        None,
        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Unknown)),
    )
}

fn make_returning_function(name: &str, return_ty: &str, initializer: &str) -> FunctionDecl {
    let body = Block {
        statements: vec![Statement::new(
            None,
            AstStatementKind::Return {
                expression: Some(initializer_expression(initializer)),
            },
        )],
        span: None,
    };

    FunctionDecl {
        visibility: Visibility::Public,
        name: name.into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple(return_ty),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(body),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: Vec::new(),
        is_extern: false,
        extern_abi: None,
        extern_options: None,
        link_name: None,
        link_library: None,
        operator: None,
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn required_property(name: &str, ty: &str) -> PropertyDecl {
    PropertyDecl {
        visibility: Visibility::Public,
        modifiers: Vec::new(),
        name: name.into(),
        ty: TypeExpr::simple(ty),
        parameters: Vec::new(),
        accessors: vec![
            PropertyAccessor {
                kind: PropertyAccessorKind::Get,
                visibility: None,
                body: PropertyAccessorBody::Auto,
                doc: None,
                attributes: None,
                span: None,
                dispatch: MemberDispatch::default(),
            },
            PropertyAccessor {
                kind: PropertyAccessorKind::Init,
                visibility: None,
                body: PropertyAccessorBody::Auto,
                doc: None,
                attributes: None,
                span: None,
                dispatch: MemberDispatch::default(),
            },
        ],
        doc: None,
        is_required: true,
        is_static: false,
        initializer: None,
        span: None,
        attributes: Vec::new(),
        di_inject: None,
        dispatch: MemberDispatch::default(),
        explicit_interface: None,
    }
}

fn required_field(name: &str, ty: &str) -> FieldDecl {
    FieldDecl {
        visibility: Visibility::Public,
        name: name.into(),
        ty: TypeExpr::simple(ty),
        initializer: None,
        mmio: None,
        doc: None,
        attributes: Vec::new(),
        is_required: true,
        display_name: None,
        is_readonly: false,
        is_static: false,
        view_of: None,
    }
}

fn optional_field(name: &str, ty: &str) -> FieldDecl {
    FieldDecl {
        visibility: Visibility::Public,
        name: name.into(),
        ty: TypeExpr::simple(ty),
        initializer: None,
        mmio: None,
        doc: None,
        attributes: Vec::new(),
        is_required: false,
        display_name: None,
        is_readonly: false,
        is_static: false,
        view_of: None,
    }
}

#[test]
fn object_initializer_missing_required_field_reports_error() {
    let mut module = Module::new(Some("Demo".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Point".into(),
        fields: vec![required_field("X", "int"), optional_field("Y", "int")],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: Vec::new(),
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        mmio: None,
        generics: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));

    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Builder".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(make_returning_function(
            "Create",
            "Point",
            "new Point { Y = 42 }",
        ))],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    let lowering = lower_module(&module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required member `X`")),
        "expected required-member diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn object_initializer_missing_required_property_reports_error() {
    let mut module = Module::new(Some("Demo".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Holder".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Property(required_property("Value", "int"))],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Factory".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(make_returning_function(
            "Build",
            "Holder",
            "new Holder { }",
        ))],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    let lowering = lower_module(&module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required member `Value`")),
        "expected required-property diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn initializer_satisfying_required_members_succeeds() {
    let mut module = Module::new(Some("Demo".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Point".into(),
        fields: vec![required_field("X", "int"), optional_field("Y", "int")],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: Vec::new(),
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        mmio: None,
        generics: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Holder".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Property(required_property("Value", "int"))],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Factory".into(),
        bases: Vec::new(),
        members: vec![
            ClassMember::Method(make_returning_function(
                "MakePoint",
                "Point",
                "new Point { X = 1 }",
            )),
            ClassMember::Method(make_returning_function(
                "MakeHolder",
                "Holder",
                "new Holder { Value = 5 }",
            )),
        ],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn struct_initializer_without_new_enforces_required_members() {
    let mut module = Module::new(Some("Demo".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Coordinates".into(),
        fields: vec![required_field("X", "int"), optional_field("Y", "int")],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: Vec::new(),
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        mmio: None,
        generics: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Builder".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(make_returning_function(
            "Create",
            "Coordinates",
            "Coordinates { Y = 10 }",
        ))],
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }));

    let lowering = lower_module(&module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required member `X`")),
        "expected diagnostic for required struct field, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn readonly_struct_field_mutation_outside_constructor_reports_error() {
    let source = r"
namespace Demo;

public readonly struct Counter
{
    public int Value;

    public init(int value)
    {
        this.Value = value;
    }

public int Increment(int delta)
    {
        this.Value = this.Value + delta;
        return this.Value;
    }
}
";
    let parsed = parse_module(source).expect("parse readonly struct");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag
                .message
                .contains("readonly struct `Demo::Counter` fields can only be assigned within its constructors")),
        "expected readonly mutation diagnostic, found {:?}",
        lowering.diagnostics
    );
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected a single readonly diagnostic when mutating outside constructors"
    );
}

#[test]
fn readonly_class_field_mutation_outside_constructor_reports_error() {
    let source = r"
public class Counter
{
    public int Value;

    public init(int value)
    {
        Value = value;
    }

    public void Reset(int delta)
    {
        this.Value = delta;
    }
}
";
    let parsed = parse_module(source).expect("parse class");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut module = parsed.module;
    if let Some(Item::Class(class)) = module.items.get_mut(0) {
        for member in &mut class.members {
            if let ClassMember::Field(field) = member {
                if field.name == "Value" {
                    field.is_readonly = true;
                }
            }
        }
    }
    let lowering = lower_module(&module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains(
                "readonly field `Counter::Value` can only be assigned within its constructors"
            )),
        "expected readonly field diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn required_initializer_reports_missing_members() {
    let source = r#"
namespace Required;

public struct Config
{
    public required int Value;
    public int Optional;
}

public class Builder
{
    public Config Build()
    {
        return new Config {};
    }
}
"#;
    let parsed = parse_module(source).expect("parse required struct");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("must assign required member `Value`")),
        "expected missing required member diagnostic, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn required_initializer_reports_multiple_members() {
    let source = r#"
namespace Demo;

public struct Pair
{
    public required int A;
    public required int B;
}

public class Factory
{
    public Pair Build()
    {
        return new Pair {};
    }
}
"#;
    let parsed = parse_module(source).expect("parse required struct with multiple members");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required members `A` and `B`")),
        "expected diagnostic listing both required members, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn required_initializer_includes_base_members() {
    let source = r#"
namespace Demo;

public class Base
{
    public required int X { get; init; }
}

public class Derived : Base
{
    public required int Y { get; init; }
}

public class Builder
{
    public Derived Create()
    {
        return new Derived { Y = 3 };
    }
}
"#;
    let parsed = parse_module(source).expect("parse required inheritance");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required member `X`")),
        "expected diagnostic for missing base requirement, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn self_initializer_resolves_current_type() {
    let source = r#"
namespace Demo;

public class Widget
{
    public required int Value { get; init; }

    public Widget CloneMissing()
    {
        return new Self { };
    }
}
"#;
    let parsed = parse_module(source).expect("parse self initializer");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("required member `Value`")),
        "expected required-member diagnostic for Self initializer, got {:?}",
        lowering.diagnostics
    );
}
