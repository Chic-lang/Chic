use super::*;
use crate::frontend::ast::{
    ConstructorDecl, ConstructorKind, Expression, FunctionDecl, MemberDispatch, Parameter,
    Signature, TypeExpr, Visibility,
};
use crate::frontend::diagnostics::{FileId, Span};
use crate::frontend::parser::parse_module;
use crate::mir::builder::default_arguments::DefaultArgumentValue;
use crate::mir::builder::module_lowering::driver::lower_module;
use crate::mir::{ConstValue, DefaultArgumentKind, Ty};
use crate::syntax::expr::{ExprNode, LiteralConst};

fn const_text(default: &ConstValue) -> String {
    match default {
        ConstValue::Int(value) | ConstValue::Int32(value) => value.to_string(),
        ConstValue::UInt(value) => value.to_string(),
        ConstValue::Bool(value) => value.to_string(),
        ConstValue::Char(value) => value.to_string(),
        ConstValue::Str { value, .. } | ConstValue::RawStr(value) => format!("\"{value}\""),
        ConstValue::Null => "null".to_string(),
        ConstValue::Unit => "()".to_string(),
        _ => "<const>".to_string(),
    }
}

fn default_expression(default: &ConstValue) -> Expression {
    Expression::with_node(
        const_text(default),
        Some(Span {
            file_id: FileId::UNKNOWN,
            start: 0,
            end: 0,
        }),
        ExprNode::Literal(LiteralConst::without_numeric(default.clone())),
    )
}

fn make_parameter(name: &str, default: Option<ConstValue>, binding: BindingModifier) -> Parameter {
    Parameter {
        binding,
        binding_nullable: false,
        name: name.to_string(),
        name_span: None,
        ty: TypeExpr::simple("int"),
        attributes: Vec::new(),
        di_inject: None,
        default: default.as_ref().map(default_expression),
        default_span: None,
        lends: None,
        is_extension_this: false,
    }
}

fn make_function_symbol(name: &str, param_default: ConstValue) -> FunctionDeclSymbol {
    let parameters = vec![make_parameter(
        "value",
        Some(param_default),
        BindingModifier::Value,
    )];
    let signature = Signature {
        parameters,
        return_type: TypeExpr::simple("int"),
        lends_to_return: None,
        variadic: false,
        throws: None,
    };
    FunctionDeclSymbol {
        qualified: name.to_string(),
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: name.to_string(),
            name_span: None,
            signature,
            body: None,
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
        },
        owner: None,
        namespace: None,
        internal_name: name.to_string(),
    }
}

fn make_constructor_symbol(name: &str, parameters: Vec<Parameter>) -> ConstructorDeclSymbol {
    ConstructorDeclSymbol {
        qualified: format!("{name}::.ctor"),
        constructor: ConstructorDecl {
            visibility: Visibility::Public,
            kind: ConstructorKind::Designated,
            parameters,
            body: None,
            initializer: None,
            doc: None,
            span: Some(Span {
                file_id: FileId::UNKNOWN,
                start: 0,
                end: 0,
            }),
            attributes: Vec::new(),
            di_inject: None,
        },
        owner: name.to_string(),
        namespace: None,
        internal_name: format!("{name}::.ctor"),
    }
}

#[test]
fn default_argument_conflicts_are_reported() {
    let mut lowering = ModuleLowering::default();
    let decl_a = make_function_symbol("Sample::run", ConstValue::Int(0));
    let mut decl_b = make_function_symbol("Sample::run", ConstValue::Int(1));
    decl_b.internal_name = "Sample::run#alt".to_string();

    lowering.build_function_default_arguments("Sample::run".to_string(), vec![decl_a, decl_b]);

    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("conflicting default values for parameter")),
        "expected conflicting default diagnostics, found {:?}",
        lowering.diagnostics
    );
    assert!(
        lowering
            .default_argument_records
            .iter()
            .any(|record| record.param_name == "value"),
        "expected default argument metadata to be recorded"
    );
}

#[test]
fn virtual_method_registration_builds_vtable() {
    let mut lowering = ModuleLowering::default();
    let mut dispatch = MemberDispatch::default();
    dispatch.is_virtual = true;
    let meta = LoweredMethodMetadata {
        owner: "Sample".to_string(),
        member: "run".to_string(),
        dispatch,
        accessor: None,
    };

    lowering.register_virtual_method(meta, "Sample::run#0");
    let tables = lowering.finalize_class_vtables();

    assert_eq!(tables.len(), 1, "expected one vtable emitted");
    let table = &tables[0];
    assert_eq!(table.type_name, "Sample");
    assert!(
        table
            .slots
            .iter()
            .any(|slot| slot.member == "run" && slot.symbol == "Sample::run#0"),
        "expected virtual slot to be populated with symbol"
    );
}

#[test]
fn default_argument_const_eval_is_recorded() {
    let mut lowering = ModuleLowering::default();
    let decl = make_function_symbol("Sample::sum", ConstValue::Int(42));

    lowering.build_function_default_arguments("Sample::sum".to_string(), vec![decl]);

    let recorded = {
        let store = lowering.default_arguments.borrow();
        store
            .value("Sample::sum", 0)
            .expect("default argument should be stored")
            .clone()
    };
    assert!(
        matches!(recorded, DefaultArgumentValue::Const(ConstValue::Int(42))),
        "expected const default to be recorded, got {recorded:?}"
    );
    assert!(
        lowering
            .default_argument_records
            .iter()
            .any(|record| matches!(
                record.value,
                DefaultArgumentKind::Const(ConstValue::Int(42))
            )),
        "expected metadata record for default argument"
    );
}

#[test]
fn default_argument_ref_binding_rejected() {
    let mut lowering = ModuleLowering::default();
    let mut decl = make_function_symbol("Sample::refy", ConstValue::Int(1));
    decl.function.signature.parameters = vec![make_parameter(
        "value",
        Some(ConstValue::Int(1)),
        BindingModifier::Ref,
    )];

    lowering.build_function_default_arguments("Sample::refy".to_string(), vec![decl]);

    assert!(
        lowering
            .default_arguments
            .borrow()
            .value("Sample::refy", 0)
            .is_none(),
        "ref parameter default should not be stored"
    );
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("cannot declare a default value")),
        "expected diagnostic when ref parameter declares a default"
    );
}

#[test]
fn function_parameter_mismatch_reports_diagnostic() {
    let mut lowering = ModuleLowering::default();
    let decl_a = make_function_symbol("Sample::overload", ConstValue::Int(0));
    let mut decl_b = make_function_symbol("Sample::overload", ConstValue::Int(1));
    decl_b.function.signature.parameters.push(make_parameter(
        "extra",
        None,
        BindingModifier::Value,
    ));

    lowering.build_function_default_arguments("Sample::overload".to_string(), vec![decl_a, decl_b]);

    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("does not match overload parameter count")),
        "expected overload parameter count diagnostic"
    );
}

#[test]
fn constructor_default_conflicts_collect_diagnostics() {
    let mut lowering = ModuleLowering::default();
    let param_one = make_parameter("value", Some(ConstValue::Int(1)), BindingModifier::Value);
    let mut param_two = param_one.clone();
    // Change the default text to trigger merge conflict.
    param_two.default = Some(Expression::new(
        "2",
        Some(Span {
            file_id: FileId::UNKNOWN,
            start: 0,
            end: 1,
        }),
    ));
    let ctor_a = make_constructor_symbol("Sample", vec![param_one]);
    let ctor_b = make_constructor_symbol("Sample", vec![param_two]);

    lowering.build_constructor_default_arguments("Sample::.ctor".to_string(), vec![ctor_a, ctor_b]);

    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("conflicting default values for parameter")),
        "expected conflicting default diagnostic for constructor parameters"
    );
}

#[test]
fn self_named_default_argument_is_rejected() {
    let mut lowering = ModuleLowering::default();
    let param = make_parameter("self", Some(ConstValue::Int(1)), BindingModifier::Value);
    let mut decl = make_function_symbol("Sample::selfy", ConstValue::Int(0));
    decl.function.signature.parameters = vec![param];

    lowering.build_function_default_arguments("Sample::selfy".to_string(), vec![decl]);

    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("implicit receiver `self`")),
        "expected implicit receiver diagnostic when default is provided for self"
    );
}

#[test]
fn missing_default_expression_node_reports_error() {
    let mut lowering = ModuleLowering::default();
    let mut decl = make_function_symbol("Sample::missing", ConstValue::Int(0));
    decl.function.signature.parameters = vec![Parameter {
        binding: BindingModifier::Value,
        binding_nullable: false,
        name: "value".to_string(),
        name_span: None,
        ty: TypeExpr::simple("int"),
        attributes: Vec::new(),
        di_inject: None,
        default: Some(Expression::new(
            "?",
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 1,
                end: 2,
            }),
        )),
        default_span: None,
        lends: None,
        is_extension_this: false,
    }];

    lowering.build_function_default_arguments("Sample::missing".to_string(), vec![decl]);

    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("not a parsable expression")),
        "expected diagnostic for missing expression node"
    );
    assert!(
        lowering
            .default_arguments
            .borrow()
            .value("Sample::missing", 0)
            .is_none(),
        "default arguments should not be recorded for unparsable defaults"
    );
}

#[test]
fn lower_default_thunk_emits_function() {
    let mut lowering = ModuleLowering::default();
    let ctx = super::defaults::DefaultArgumentCtx {
        internal_name: "Sample::run",
        display_name: "Sample::run",
        owner: None,
        namespace: None,
        type_generics: Vec::new(),
        method_generics: Vec::new(),
    };
    let expr = Expression::with_node(
        "make",
        Some(Span {
            file_id: FileId::UNKNOWN,
            start: 0,
            end: 0,
        }),
        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(5))),
    );
    let value =
        lowering.lower_default_thunk(ctx, "Sample::run#default", &Ty::named("int"), &expr, 0);

    assert!(
        matches!(
            value,
            Some(DefaultArgumentValue::Thunk {
                symbol,
                metadata_count: 0,
                ..
            }) if symbol == "Sample::run#default"
        ),
        "expected thunk default to be created"
    );
    assert!(
        lowering
            .functions
            .iter()
            .any(|function| function.name == "Sample::run#default"),
        "thunk function should be registered"
    );
}

#[test]
fn expr_path_segments_handles_paths_and_errors() {
    let nested = ExprNode::Member {
        base: Box::new(ExprNode::Member {
            base: Box::new(ExprNode::Identifier("root".to_string())),
            member: "child".to_string(),
            null_conditional: false,
        }),
        member: "leaf".to_string(),
        null_conditional: false,
    };
    assert_eq!(
        expr_path_segments(&nested).unwrap(),
        vec!["root".to_string(), "child".to_string(), "leaf".to_string()]
    );

    let parenthesized =
        ExprNode::Parenthesized(Box::new(ExprNode::Identifier("value".to_string())));
    assert_eq!(
        expr_path_segments(&parenthesized).unwrap(),
        vec!["value".to_string()]
    );

    let call = ExprNode::Call {
        callee: Box::new(ExprNode::Identifier("f".to_string())),
        args: Vec::new(),
        generics: None,
    };
    assert!(
        expr_path_segments(&call).is_err(),
        "non-path expressions should be rejected"
    );
}

#[test]
fn is_power_of_two_covers_edge_cases() {
    assert!(!is_power_of_two(0));
    assert!(is_power_of_two(1));
    assert!(is_power_of_two(64));
    assert!(!is_power_of_two(96));
}

#[test]
#[should_panic(expected = "exceeds u32 range")]
fn expect_u32_index_panics_on_overflow() {
    let _ = expect_u32_index(u32::MAX as usize + 1, "test context");
}

#[test]
fn finalize_class_vtables_skips_incomplete_slots() {
    let mut lowering = ModuleLowering::default();
    lowering.class_vtable_plans.insert(
        "Unlowered".to_string(),
        ClassVTablePlan::testing_with_slot("Unlowered", "run"),
    );

    let tables = lowering.finalize_class_vtables();

    assert!(
        tables.is_empty(),
        "tables with unresolved symbols should be omitted"
    );
}

#[test]
fn missing_base_type_reports_lowering_diagnostic() {
    let parsed = parse_module(
        r#"
namespace Demo;

public class Derived : MissingBase { }
"#,
    )
    .expect("parse module");
    assert!(
        parsed.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let result = lower_module(&parsed.module);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("MissingBase")),
        "expected missing base diagnostic, got {:?}",
        result.diagnostics
    );
}

#[test]
fn override_without_base_emits_diagnostic() {
    let mut lowering = ModuleLowering::default();
    let mut dispatch = MemberDispatch::default();
    dispatch.is_override = true;
    let meta = LoweredMethodMetadata {
        owner: "Derived".to_string(),
        member: "run".to_string(),
        dispatch,
        accessor: None,
    };

    lowering.register_virtual_method(meta, "Derived::run#0");

    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("marked `override`")),
        "expected override diagnostic when no virtual base exists"
    );
}
