#![cfg(test)]

use crate::frontend::ast::{
    GenericConstraint, GenericConstraintKind, GenericParam, GenericParamKind, GenericParams,
    MemberDispatch, Module, PropertyAccessorKind, TypeParamData, Variance, Visibility,
};
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::{
    FunctionSignature, InheritedMembers, MethodDispatchInfo, MethodRecord, PropertyAccessorInfo,
    PropertyAccessorRecord, PropertyAccessors, PropertyInfo, TypeChecker,
};

#[test]
fn method_override_conflict_reports_static_mismatch() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let base_sig = checker.allocate_signature(FunctionSignature {
        name: "Base.Run".to_string(),
        param_types: Vec::new(),
        return_type: "void".to_string(),
        span: None,
    });
    let base = MethodRecord {
        owner: "Base".to_string(),
        signature_id: base_sig,
        dispatch: MemberDispatch {
            is_virtual: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: true,
        span: None,
    };
    let meta = MethodDispatchInfo {
        dispatch: MemberDispatch {
            is_override: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: false,
        has_body: true,
        span: None,
    };

    let override_sig = FunctionSignature {
        name: "Child.Run".to_string(),
        param_types: Vec::new(),
        return_type: "void".to_string(),
        span: None,
    };

    let override_id = checker.allocate_signature(override_sig.clone());

    assert!(checker.method_override_conflicts(&override_sig, override_id, None, &meta, &base));
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK208")),
        "expected static conflict diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn method_override_conflict_reports_sealed_base() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let base_sig = checker.allocate_signature(FunctionSignature {
        name: "Base.Speak".to_string(),
        param_types: Vec::new(),
        return_type: "void".to_string(),
        span: None,
    });
    let base = MethodRecord {
        owner: "Base".to_string(),
        signature_id: base_sig,
        dispatch: MemberDispatch {
            is_virtual: true,
            is_sealed: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: false,
        span: None,
    };
    let meta = MethodDispatchInfo {
        dispatch: MemberDispatch {
            is_override: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: false,
        has_body: true,
        span: None,
    };

    let override_sig = FunctionSignature {
        name: "Child.Speak".to_string(),
        param_types: Vec::new(),
        return_type: "void".to_string(),
        span: None,
    };

    let override_id = checker.allocate_signature(override_sig.clone());

    assert!(checker.method_override_conflicts(&override_sig, override_id, None, &meta, &base));
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK201")),
        "expected sealed override diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn method_override_reports_generic_constraint_mismatch() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);

    let base_sig = checker.allocate_signature(FunctionSignature {
        name: "Base.Identity".to_string(),
        param_types: vec!["T".to_string()],
        return_type: "T".to_string(),
        span: None,
    });
    let base_param = GenericParam {
        name: "T".to_string(),
        span: None,
        kind: GenericParamKind::Type(TypeParamData {
            constraints: vec![GenericConstraint::new(GenericConstraintKind::Class, None)],
            variance: Variance::Invariant,
        }),
    };
    let base_generics = GenericParams::new(None, vec![base_param]);
    checker.record_signature_generics(base_sig, Some(&base_generics));

    let base = MethodRecord {
        owner: "Base".to_string(),
        signature_id: base_sig,
        dispatch: MemberDispatch {
            is_virtual: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: false,
        span: None,
    };

    let override_sig = FunctionSignature {
        name: "Child.Identity".to_string(),
        param_types: vec!["T".to_string()],
        return_type: "T".to_string(),
        span: None,
    };
    let override_param = GenericParam::type_param("T", None);
    let override_generics = GenericParams::new(None, vec![override_param]);
    let override_id = checker.allocate_signature(override_sig.clone());
    checker.record_signature_generics(override_id, Some(&override_generics));

    let meta = MethodDispatchInfo {
        dispatch: MemberDispatch {
            is_override: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        is_static: false,
        has_body: true,
        span: None,
    };

    assert!(checker.method_override_conflicts(&override_sig, override_id, None, &meta, &base));
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK210")),
        "expected generic mismatch diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn sealed_property_requires_override_even_without_parser() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let property = PropertyInfo {
        name: "Value".to_string(),
        ty: "int".to_string(),
        accessors: PropertyAccessors {
            get: false,
            set: true,
            init: false,
        },
        is_static: false,
        span: None,
        accessor_details: vec![PropertyAccessorInfo {
            kind: PropertyAccessorKind::Set,
            dispatch: MemberDispatch {
                is_sealed: true,
                ..MemberDispatch::default()
            },
            visibility: Visibility::Public,
            span: None,
            has_body: true,
        }],
    };
    let mut inherited = InheritedMembers::default();
    checker.validate_property_member("Child", &property, &mut inherited);
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK207")),
        "expected sealed property diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn abstract_init_accessor_cannot_have_body() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let property = PropertyInfo {
        name: "Value".to_string(),
        ty: "int".to_string(),
        accessors: PropertyAccessors {
            get: false,
            set: false,
            init: true,
        },
        is_static: false,
        span: None,
        accessor_details: vec![PropertyAccessorInfo {
            kind: PropertyAccessorKind::Init,
            dispatch: MemberDispatch {
                is_abstract: true,
                ..MemberDispatch::default()
            },
            visibility: Visibility::Public,
            span: None,
            has_body: true,
        }],
    };
    let mut inherited = InheritedMembers::default();
    checker.validate_property_member("Child", &property, &mut inherited);
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK205")),
        "expected abstract accessor body diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn property_override_conflict_reports_static_mismatch() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let accessor = PropertyAccessorInfo {
        kind: PropertyAccessorKind::Get,
        dispatch: MemberDispatch {
            is_override: true,
            ..MemberDispatch::default()
        },
        visibility: Visibility::Public,
        span: None,
        has_body: true,
    };
    let property = PropertyInfo {
        name: "Value".to_string(),
        ty: "int".to_string(),
        accessors: PropertyAccessors {
            get: true,
            set: false,
            init: false,
        },
        is_static: false,
        span: None,
        accessor_details: vec![accessor.clone()],
    };
    let base = PropertyAccessorRecord {
        owner: "Base".to_string(),
        property_type: "int".to_string(),
        dispatch: MemberDispatch {
            is_virtual: true,
            ..MemberDispatch::default()
        },
        is_static: true,
        span: None,
        visibility: Visibility::Public,
    };

    assert!(checker.property_override_conflicts("Child", &property, &accessor, &base));
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("TCK208")),
        "expected property static conflict diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn visibility_rank_covers_all_variants() {
    assert_eq!(TypeChecker::visibility_rank(Visibility::Public), 5);
    assert_eq!(
        TypeChecker::visibility_rank(Visibility::ProtectedInternal),
        4
    );
    assert_eq!(TypeChecker::visibility_rank(Visibility::Protected), 3);
    assert_eq!(TypeChecker::visibility_rank(Visibility::Internal), 2);
    assert_eq!(
        TypeChecker::visibility_rank(Visibility::PrivateProtected),
        1
    );
    assert_eq!(TypeChecker::visibility_rank(Visibility::Private), 0);
}

#[test]
fn describe_accessor_kind_is_exhaustive() {
    assert_eq!(
        TypeChecker::describe_accessor_kind(PropertyAccessorKind::Get),
        "getter"
    );
    assert_eq!(
        TypeChecker::describe_accessor_kind(PropertyAccessorKind::Set),
        "setter"
    );
    assert_eq!(
        TypeChecker::describe_accessor_kind(PropertyAccessorKind::Init),
        "init accessor"
    );
}
