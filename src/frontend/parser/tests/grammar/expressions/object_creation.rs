use crate::frontend::ast::{Item, StatementKind, VariableModifier};
use crate::frontend::parser::tests::fixtures::{function_body, parse_fail, parse_ok};
use crate::syntax::expr::{ExprNode, NewInitializer};

#[test]
fn parses_object_initializer_in_variable_declaration() {
    let source = r"
public void Build()
{
    var widget = new Demo.Widget<int>(capacity: 4) { Size = 42 };
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let func = match &parse.module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}", other = other),
    };
    let body = function_body(func);
    let decl = match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => decl,
        other => panic!(
            "expected variable declaration, found {other:?}",
            other = other
        ),
    };
    assert!(matches!(decl.modifier, VariableModifier::Var));
    let init = decl.declarators[0]
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let new_expr = init
        .as_new_expr()
        .expect("initializer should lower to ExprNode::New");
    assert_eq!(new_expr.type_name, "Demo.Widget<int>");
    assert_eq!(new_expr.args.len(), 1);
    let arg = &new_expr.args[0];
    assert!(matches!(arg.name, Some(ref name) if name.text == "capacity"));
    assert!(matches!(arg.value, ExprNode::Literal(_)));
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("object initializer should be recorded");
    let fields = match initializer {
        NewInitializer::Object { fields, .. } => fields,
        other => panic!(
            "expected object initializer, found {other:?}",
            other = other
        ),
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "Size");
    assert!(matches!(fields[0].value, ExprNode::Literal(_)));
}

#[test]
fn parses_collection_initializer_in_variable_declaration() {
    let source = r"
public void Build()
{
    var numbers = new Numbers { 1, 2, seed };
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let func = match &parse.module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}", other = other),
    };
    let body = function_body(func);
    let decl = match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => decl,
        other => panic!(
            "expected variable declaration, found {other:?}",
            other = other
        ),
    };
    let init = decl.declarators[0]
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let new_expr = init
        .as_new_expr()
        .expect("initializer should lower to ExprNode::New");
    assert_eq!(new_expr.type_name, "Numbers");
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("collection initializer should be recorded");
    let elements = match initializer {
        NewInitializer::Collection { elements, .. } => elements,
        other => panic!(
            "expected collection initializer, found {other:?}",
            other = other
        ),
    };
    assert_eq!(elements.len(), 3);
    assert!(matches!(elements[0], ExprNode::Literal(_)));
    assert!(matches!(elements[2], ExprNode::Identifier(ref name) if name == "seed"));
}

#[test]
fn parses_generic_object_initializer_with_argument_spans() {
    let source = r#"
public void Build()
{
    var widget = new Demo.Widget<int>(capacity: 4) { Width = 7, Height = 5 };
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let func = match &parse.module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}"),
    };
    let body = function_body(func);
    let decl = match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => decl,
        other => panic!("expected variable declaration, found {other:?}"),
    };
    let init = decl.declarators[0]
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let new_expr = init
        .as_new_expr()
        .expect("initializer must be ExprNode::New");
    assert_eq!(new_expr.type_name, "Demo.Widget<int>");
    assert!(
        new_expr.keyword_span.is_some(),
        "keyword span should be threaded"
    );
    assert!(new_expr.type_span.is_some(), "type span should be threaded");
    assert!(
        new_expr.arguments_span.is_some(),
        "argument span should be recorded for diagnostics"
    );
    assert_eq!(new_expr.args.len(), 1);
    let arg = &new_expr.args[0];
    assert!(
        matches!(&arg.name, Some(name) if name.text == "capacity" && name.span.is_some()),
        "named argument + span should be preserved"
    );
    assert!(
        arg.span.is_some() && arg.value_span.is_some(),
        "argument spans should survive for diagnostics"
    );
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("object initializer should be captured");
    let fields = match initializer {
        NewInitializer::Object { fields, span } => {
            assert!(
                span.is_some(),
                "object initializer span should be captured for highlighting"
            );
            fields
        }
        other => panic!("expected object initializer, found {other:?}"),
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "Width");
    assert!(fields[0].name_span.is_some());
    assert!(fields[0].value_span.is_some());
}

#[test]
fn parses_empty_object_initializer_block() {
    let source = r#"
public void Build()
{
    var widget = new Demo.Widget<int>() { };
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let func = match &parse.module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}"),
    };
    let body = function_body(func);
    let decl = match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => decl,
        other => panic!("expected variable declaration, found {other:?}"),
    };
    let init = decl.declarators[0]
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let new_expr = init
        .as_new_expr()
        .expect("initializer must be ExprNode::New");
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("initializer should be present even when block is empty");
    match initializer {
        NewInitializer::Object { fields, .. } => {
            assert!(
                fields.is_empty(),
                "empty initializer should record zero fields"
            );
        }
        other => panic!("expected object initializer, found {other:?}"),
    }
}

#[test]
fn object_initializer_then_collection_element_reports_error() {
    let source = r#"
public void Build()
{
    var widget = new Bucket { Count = 1, "extra" };
}
"#;

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("collection elements cannot be mixed with object member assignments")),
        "expected diagnostic about mixing collection and object entries, found {:?}",
        diagnostics
    );
}

#[test]
fn collection_element_then_object_initializer_reports_error() {
    let source = r#"
public void Build()
{
    var widget = new Bucket { "value", Count = 2 };
}
"#;

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("object initializer entries cannot be mixed with collection elements")),
        "expected diagnostic about mixing initializer entries, found {:?}",
        diagnostics
    );
}
