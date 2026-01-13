use super::fixtures::{const_generic_argument, simple_class};
use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::expressions::Expression;
use crate::frontend::ast::{
    ClassMember, ConstWherePredicate, FieldDecl, GenericParam, GenericParams, Item, Module,
    TypeExpr, TypeSuffix, Visibility,
};
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::lowered(
        "const_fn_accepts_pure_body_and_const_consumers",
        r#"
namespace Demo;

public const fn Add(int lhs, int rhs) -> int
{
    return lhs + rhs;
}

public const int Sum = Add(2, 3);
"#,
        Expectation::lacks(&["[TCK160]", "[TCK161]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "const_fn_rejects_unsupported_statements",
        r#"
namespace Demo;

public const fn Spin() -> int
{
    var acc = 0;
    while (acc < 3) { acc = acc + 1; }
    return acc;
}
"#,
        Expectation::contains(&["[TCK161]"]),
    ),
    ArenaDiagnosticCase::lowered(
        "const_fn_rejects_async_signature",
        r#"
namespace Demo;

public async const fn Bad() -> int
{
    return 1;
}
"#,
        Expectation::contains(&["[TCK160]"]),
    ),
    ArenaDiagnosticCase::custom(
        "const_generic_argument_satisfying_predicate_is_accepted",
        const_generic_argument_satisfying_predicate,
        Expectation::clean(),
    ),
    ArenaDiagnosticCase::custom(
        "const_generic_constraint_violation_reports_error",
        const_generic_constraint_violation,
        Expectation::contains(&["evaluated to false"]),
    ),
];

fn const_generic_argument_satisfying_predicate(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let mut param = GenericParam::const_param("N", None, TypeExpr::simple("int"));
    param
        .as_const_mut()
        .expect("const param")
        .constraints
        .push(ConstWherePredicate::new(
            Expression::new("N > 0".to_string(), None),
            None,
        ));
    let buffer = simple_class(
        "Buffer",
        Vec::new(),
        Some(GenericParams {
            span: None,
            params: vec![param],
        }),
    );
    let mut field_ty = TypeExpr::simple("Buffer");
    field_ty
        .suffixes
        .push(TypeSuffix::GenericArgs(vec![const_generic_argument("4")]));
    let usage = simple_class(
        "Usage",
        vec![ClassMember::Field(FieldDecl {
            visibility: Visibility::Public,
            name: "Data".into(),
            ty: field_ty,
            initializer: None,
            mmio: None,
            doc: None,
            is_required: false,
            display_name: None,
            attributes: Vec::new(),
            is_readonly: false,
            is_static: false,
            view_of: None,
        })],
        None,
    );
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![Item::Class(buffer), Item::Class(usage)],
    );
    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn const_generic_constraint_violation(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut param = GenericParam::const_param("N", None, TypeExpr::simple("int"));
    param
        .as_const_mut()
        .expect("const param")
        .constraints
        .push(ConstWherePredicate::new(
            Expression::new("N > 0".to_string(), None),
            None,
        ));
    let buffer = simple_class(
        "Buffer",
        Vec::new(),
        Some(GenericParams {
            span: None,
            params: vec![param],
        }),
    );
    let mut field_ty = TypeExpr::simple("Buffer");
    field_ty
        .suffixes
        .push(TypeSuffix::GenericArgs(vec![const_generic_argument("0")]));
    let usage = simple_class(
        "Usage",
        vec![ClassMember::Field(FieldDecl {
            visibility: Visibility::Public,
            name: "Data".into(),
            ty: field_ty,
            initializer: None,
            mmio: None,
            doc: None,
            is_required: false,
            display_name: None,
            attributes: Vec::new(),
            is_readonly: false,
            is_static: false,
            view_of: None,
        })],
        None,
    );
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![Item::Class(buffer), Item::Class(usage)],
    );
    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

#[test]
fn const_related_diagnostics() {
    run_cases("consts", CASES);
}
