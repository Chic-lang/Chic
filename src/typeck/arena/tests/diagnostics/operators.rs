use super::fixtures::operator_function;
use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::Module;
use crate::frontend::ast::{
    BinaryOperator, ClassDecl, ClassKind, ClassMember, ConversionKind, Item, OperatorKind,
    UnaryOperator, Visibility,
};
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::TypeCheckResult;

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::custom(
        "unary_operator_requires_owner_operand",
        unary_operator_requires_owner_operand,
        Expectation::contains(&["must take `Value` as its operand"]),
    ),
    ArenaDiagnosticCase::custom(
        "binary_operator_requires_owner_parameter",
        binary_operator_requires_owner_parameter,
        Expectation::contains(&["must have at least one parameter of type `Value`"]),
    ),
    ArenaDiagnosticCase::custom(
        "conversion_operator_requires_owner_participation",
        conversion_operator_requires_owner_participation,
        Expectation::contains(&["must convert to or from `Value`"]),
    ),
    ArenaDiagnosticCase::custom(
        "operator_requires_bool_return",
        operator_requires_bool_return,
        Expectation::contains(&["must return `bool`"]),
    ),
    ArenaDiagnosticCase::custom(
        "operator_pairing_required",
        operator_pairing_required,
        Expectation::contains(&["requires a matching operator !="]),
    ),
    ArenaDiagnosticCase::custom(
        "operator_must_be_public",
        operator_must_be_public,
        Expectation::contains(&["declared `public`"]),
    ),
];

fn unary_operator_requires_owner_operand(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(operator_function(
            OperatorKind::Unary(UnaryOperator::Negate),
            &["int"],
            "Value",
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn binary_operator_requires_owner_parameter(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(operator_function(
            OperatorKind::Binary(BinaryOperator::Add),
            &["int", "double"],
            "Value",
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn conversion_operator_requires_owner_participation(
    fixture: &ArenaDiagnosticFixture,
) -> TypeCheckResult {
    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(operator_function(
            OperatorKind::Conversion(ConversionKind::Implicit),
            &["int"],
            "string",
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn operator_requires_bool_return(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(operator_function(
            OperatorKind::Binary(BinaryOperator::Equal),
            &["Value", "Value"],
            "Value",
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn operator_pairing_required(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(operator_function(
            OperatorKind::Binary(BinaryOperator::Equal),
            &["Value", "Value"],
            "bool",
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

fn operator_must_be_public(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let mut op = operator_function(
        OperatorKind::Unary(UnaryOperator::Negate),
        &["Value"],
        "Value",
    );
    op.visibility = Visibility::Private;

    let mut module = Module::new(Some("Numbers".into()));
    module.push_item(Item::Class(ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members: vec![ClassMember::Method(op)],
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

    fixture.check_module(&module, &[], &TypeLayoutTable::default())
}

#[test]
fn operator_diagnostics() {
    run_cases("operators", CASES);
}
