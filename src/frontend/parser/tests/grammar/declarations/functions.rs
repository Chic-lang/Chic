use super::*;
use crate::frontend::ast::RefKind;
use crate::syntax::expr::ExprNode;

#[test]
fn parses_function_struct_enum() {
    let module = parse_geometry_module();
    assert_geometry_namespace(&module);
    assert_add_function(&module.items[0]);
    assert_point_struct(&module.items[1]);
    assert!(matches!(module.items[2], Item::Enum(_)));
}

#[test]
fn parses_free_function_at_module_scope() {
    let source = r"
public int Add(int lhs, int rhs)
{
    return lhs + rhs;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.module.namespace.is_none(),
        "module should not record a namespace"
    );
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected free function, found {:?}", parse.module.items[0]);
    };
    assert_eq!(function.visibility, Visibility::Public);
    assert_eq!(function.name, "Add");
    assert_eq!(function.signature.parameters.len(), 2);
    assert_eq!(function.signature.parameters[0].ty.name, "int");
    assert_eq!(function.signature.parameters[1].ty.name, "int");
    assert_eq!(function.signature.return_type.name, "int");
}

#[test]
fn parses_free_function_inside_file_namespace() {
    let source = r"
namespace Math;

internal double Hypot(double x, double y)
{
    return sqrt(x * x + y * y);
}
";

    let parse = parse_ok(source);
    assert_eq!(parse.module.namespace.as_deref(), Some("Math"));
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected free function, found {:?}", parse.module.items[0]);
    };
    assert_eq!(function.visibility, Visibility::Internal);
    assert_eq!(function.name, "Hypot");
    assert_eq!(function.signature.parameters.len(), 2);
    assert_eq!(function.signature.parameters[0].ty.name, "double");
    assert_eq!(function.signature.parameters[1].ty.name, "double");
    assert_eq!(function.signature.return_type.name, "double");
}

#[test]
fn parses_function_with_throws_clause() {
    let source = r"
namespace Demo;

public string Load(str path) throws IoError, FormatError
{
    return Backend.Load(path);
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected function, found {:?}", parse.module.items[0]);
    };
    let throws = function
        .signature
        .throws
        .as_ref()
        .expect("expected throws clause");
    let effect_names: Vec<_> = throws.types.iter().map(|ty| ty.name.as_str()).collect();
    assert_eq!(effect_names, ["IoError", "FormatError"]);
}

#[test]
fn reports_error_for_empty_throws_clause() {
    let source = r"
namespace Demo;

public void Invalid() throws
{
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("requires at least one exception type")),
        "expected diagnostic about missing exception type, got {:?}",
        diagnostics
    );
}

#[test]
fn parses_nullable_ref_and_out_parameters() {
    let source = r"
namespace Demo;

public void Update(ref? string name, out? int count) { }
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let func = match &parse.module.items[0] {
        Item::Function(func) => func,
        other => panic!("expected function, found {other:?}"),
    };
    assert_eq!(func.signature.parameters.len(), 2);

    let name_param = &func.signature.parameters[0];
    assert!(matches!(name_param.binding, BindingModifier::Ref));
    assert!(name_param.binding_nullable);
    assert_eq!(name_param.ty.name, "string");

    let count_param = &func.signature.parameters[1];
    assert!(matches!(count_param.binding, BindingModifier::Out));
    assert!(count_param.binding_nullable);
    assert_eq!(count_param.ty.name, "int");
}

#[test]
fn rejects_nullable_in_binding() {
    let source = r"
namespace Demo;

public void Invalid(in? int value) { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`?` after parameter modifier is only valid with `ref` or `out`")),
        "expected diagnostic about invalid nullable modifier, got {:?}",
        diagnostics
    );
}

#[test]
fn rejects_duplicate_nullable_binding_modifier() {
    let source = r"
namespace Demo;

public void Invalid(ref?? string value) { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("parameter modifier accepts at most one `?`")),
        "expected diagnostic about duplicate `?`, got {:?}",
        diagnostics
    );
}

#[test]
fn parses_function_parameter_default_expression_with_metadata() {
    let source = r"
namespace Demo;

public int Add(int lhs, int rhs = Next())
{
    return lhs + rhs;
}

static int Next() { return 1; }
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected function item");
    };
    assert_eq!(function.signature.parameters.len(), 2);
    let default = function.signature.parameters[1]
        .default
        .as_ref()
        .expect("expected default expression on rhs parameter");
    assert_eq!(default.text.trim(), "Next()");
    let Some(node) = default.node.as_ref() else {
        panic!("default expression should carry parsed node");
    };
    assert!(
        matches!(node, ExprNode::Call { .. }),
        "expected call node, found {node:?}"
    );
}

#[test]
fn parses_expression_bodied_function_into_return_block() {
    let source = r"
namespace Demo;

public int Double(int value) => value * 2;
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected function, found {:?}", parse.module.items[0]);
    };
    let body = function
        .body
        .as_ref()
        .expect("expression-bodied function should synthesise a block");
    assert_eq!(
        body.statements.len(),
        1,
        "expression-bodied function should produce a single return statement"
    );
    match &body.statements[0].kind {
        StatementKind::Return { expression } => {
            assert!(
                expression.is_some(),
                "return statement should include the expression body"
            );
        }
        other => panic!("expected return statement, found {other:?}"),
    }
}

#[test]
fn function_return_type_supports_ref_qualifier() {
    let source = r"
namespace Demo;

public ref int Identity() => 0;
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected function item");
    };
    assert_eq!(
        function.signature.return_type.ref_kind,
        Some(RefKind::Ref),
        "return type should preserve ref qualifier"
    );
}

#[test]
fn parses_expression_bodied_function_with_return() {
    let source = r"
public int Identity(int value) => value;
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty(), "unexpected diagnostics");
    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected free function, found {:?}", parse.module.items[0]);
    };
    let body = function
        .body
        .as_ref()
        .expect("expected body for expression member");
    assert_eq!(body.statements.len(), 1);
    match &body.statements[0].kind {
        StatementKind::Return {
            expression: Some(expr),
        } => {
            assert_eq!(expr.text.trim(), "value");
        }
        other => panic!("expected return statement, found {other:?}"),
    }
}

#[test]
fn parses_expression_bodied_void_function_as_expression() {
    let source = r"
public void Log(int value) => Print(value);
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty(), "unexpected diagnostics");
    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected free function, found {:?}", parse.module.items[0]);
    };
    let body = function
        .body
        .as_ref()
        .expect("expected body for expression member");
    assert_eq!(body.statements.len(), 1);
    match &body.statements[0].kind {
        StatementKind::Expression(expr) => {
            assert_eq!(expr.text.trim(), "Print(value)");
        }
        other => panic!("expected expression statement, found {other:?}"),
    }
}

#[test]
fn expression_bodied_function_is_lowered_into_block() {
    let source = r"
namespace Geometry;

public double Square(double x) => x * x;
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected Item::Function, found {:?}", parse.module.items[0]);
    };
    let body = function.body.as_ref().expect("expected lowered block");
    assert_eq!(body.statements.len(), 1);
    match &body.statements[0].kind {
        StatementKind::Return {
            expression: Some(expr),
        } => {
            assert_eq!(expr.text.trim(), "x * x");
        }
        other => panic!("expected return statement, found {other:?}"),
    }
}

#[test]
fn parses_function_with_multi_declarators() {
    let source = r"
namespace Geometry;

public void Init()
{
    let x = 5, y = 6, z = 50;
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let func = match &module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}"),
    };
    let body = function_body(func);
    match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => {
            assert!(matches!(decl.modifier, VariableModifier::Let));
            assert_eq!(decl.declarators.len(), 3);
        }
        other => panic!("expected variable declaration, found {other:?}"),
    }
}

#[test]
fn parses_unicode_identifiers() {
    let source = r"
namespace Geometry;

public int Сумма(int значение, int 数据)
{
    return значение + 数据;
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let func = match &parse.module.items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, found {other:?}"),
    };
    let body = function_body(func);
    match &body.statements[0].kind {
        StatementKind::Return {
            expression: Some(expr),
        } => {
            assert!(expr.text.contains("значение"));
            assert!(expr.text.contains("数据"));
        }
        other => panic!("expected return, found {other:?}"),
    }
}

#[test]
fn parses_function_doc_comment_and_default_parameter() {
    let source = r"
namespace Demo;

/// Adds two integers with an optional right operand.
public int Add(int lhs, int rhs = supportsSimd ? 2 : 1)
{
    return lhs + rhs;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Function(function) = &parse.module.items[0] else {
        panic!("expected free function, found {:?}", parse.module.items[0]);
    };
    let doc = function
        .doc
        .as_ref()
        .expect("function should capture doc comment")
        .as_text();
    assert!(
        doc.contains("Adds two integers"),
        "doc comment mismatch: {doc:?}"
    );
    assert_eq!(function.signature.parameters.len(), 2);
    assert!(function.signature.parameters[0].default.is_none());
    let rhs_param = &function.signature.parameters[1];
    let rhs_default = rhs_param
        .default
        .as_ref()
        .expect("rhs parameter should record default expression");
    assert_eq!(
        rhs_default.text.trim(),
        "supportsSimd ? 2 : 1",
        "default expression mismatch"
    );
    match rhs_default
        .node
        .as_ref()
        .expect("default expression should be parsed")
    {
        ExprNode::Conditional { .. } => {}
        other => panic!("expected conditional expression, found {other:?}"),
    }
}

#[test]
fn impl_keyword_is_rejected_in_function_context() {
    let source = r"
impl Logger for ConsoleLogger
{
    public void Log(string message) { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("`impl` is no longer supported")),
        "expected impl keyword to be rejected, found {:?}",
        diagnostics
    );
}

#[test]
fn async_testcase_uses_task_return_type() {
    let source = r"
async testcase Runs()
{
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let Item::TestCase(case) = &parse.module.items[0] else {
        panic!("expected testcase, found {:?}", parse.module.items[0]);
    };
    let signature = case
        .signature
        .as_ref()
        .expect("async testcase should expose a signature");
    assert_eq!(signature.return_type.name, "Task");
    assert!(case.is_async, "async testcase flag should be set");
}

#[test]
fn testcase_with_parameters_records_signature() {
    let source = r"
testcase Runs(int count, string name)
{
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let Item::TestCase(case) = &parse.module.items[0] else {
        panic!("expected testcase, found {:?}", parse.module.items[0]);
    };
    let signature = case
        .signature
        .as_ref()
        .expect("testcase should expose signature");
    let param_names: Vec<_> = signature
        .parameters
        .iter()
        .map(|param| param.name.as_str())
        .collect();
    assert_eq!(param_names, ["count", "name"]);
}

#[test]
fn testcase_missing_body_reports_error() {
    let source = r"
testcase Missing();
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("expected '{' to start test body")),
        "expected missing body diagnostic, found {:?}",
        diagnostics
    );
}
