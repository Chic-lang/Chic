#![cfg(test)]

use std::collections::HashSet;

use super::fixtures::module_with_struct;
use crate::frontend::ast::{GenericParam, GenericParams, Module, TypeExpr, TypeSuffix, Visibility};
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutInfo, ClassLayoutKind, StructLayout, TypeLayout,
    TypeLayoutTable, TypeRepr,
};
use crate::typeck::arena::{
    BaseTypeBinding, InterfaceDefaultKind, InterfaceDefaultProvider, TypeChecker, TypeInfo,
    TypeKind,
};

fn empty_struct_layout(name: &str, bases: Vec<String>) -> TypeLayout {
    TypeLayout::Class(StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases,
            vtable_offset: None,
        }),
    })
}

fn simple_interface(_name: &str, bases: Vec<BaseTypeBinding>) -> TypeInfo {
    TypeInfo {
        kind: TypeKind::Interface {
            methods: Vec::new(),
            properties: Vec::new(),
            bases,
        },
        generics: None,
        repr_c: false,
        packing: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        visibility: Visibility::Public,
    }
}

#[test]
fn generic_tracking_skips_empty_and_reads_all_sources() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);

    let empty = GenericParams::default();
    checker.push_pending_generics("Empty", Some(&empty));
    assert!(
        !checker.pending_generics_contain("Empty", "T"),
        "empty generic lists should be ignored"
    );
    assert!(checker.generic_param_in_owner("Empty", "T").is_none());

    let pending = GenericParams::new(None, vec![GenericParam::type_param("T", None)]);
    checker.push_pending_generics("PendingOwner", Some(&pending));
    assert!(checker.pending_generics_contain("PendingOwner", "T"));
    assert!(
        checker
            .generic_param_in_owner("PendingOwner", "T")
            .is_some()
    );
    checker.pop_pending_generics("PendingOwner");

    let fn_params = GenericParams::new(None, vec![GenericParam::type_param("U", None)]);
    checker.register_function_generics("FuncOwner", Some(&fn_params));
    assert!(checker.function_generics_contain("FuncOwner", "U"));
    assert!(checker.generic_param_in_owner("FuncOwner", "U").is_some());

    let type_info = TypeInfo {
        kind: TypeKind::Struct {
            constructors: Vec::new(),
            is_record: false,
            bases: Vec::new(),
        },
        generics: Some(GenericParams::new(
            None,
            vec![GenericParam::type_param("V", None)],
        )),
        repr_c: false,
        packing: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        visibility: Visibility::Public,
    };
    checker.insert_type_info("Container".to_string(), type_info);
    assert!(checker.generic_param_in_owner("Container", "V").is_some());
    assert!(
        checker
            .generic_param_in_owner("Container", "Missing")
            .is_none()
    );
}

#[test]
fn interface_closure_collects_transitive_bases() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);

    let base = simple_interface("BaseIface", Vec::new());
    let derived = simple_interface(
        "ChildIface",
        vec![BaseTypeBinding::new(
            "BaseIface".to_string(),
            TypeExpr::simple("BaseIface"),
        )],
    );
    checker.insert_type_info("BaseIface".to_string(), base);
    checker.insert_type_info("ChildIface".to_string(), derived);

    let closure = checker.collect_interface_closure(&[
        BaseTypeBinding::new("ChildIface".to_string(), TypeExpr::simple("ChildIface")),
        BaseTypeBinding::new("BaseIface".to_string(), TypeExpr::simple("BaseIface")),
    ]);
    assert!(closure.contains("BaseIface"));
    assert!(closure.contains("ChildIface"));
    assert_eq!(closure.len(), 2);
}

#[test]
fn interface_defaults_handle_all_selection_paths() {
    let module = Module::new(None);
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    let implemented = HashSet::new();

    assert!(
        !checker.try_apply_interface_default("Impl", "IMissing", "Run", &implemented, None),
        "no providers should return false"
    );

    checker.register_interface_default_provider(
        "IFace",
        InterfaceDefaultProvider {
            method: "Run".to_string(),
            symbol: "inline_impl".to_string(),
            kind: InterfaceDefaultKind::Inline,
            conditions: Vec::new(),
            span: None,
            origin: "inline".to_string(),
        },
    );
    assert!(checker.try_apply_interface_default("Impl", "IFace", "Run", &implemented, None));
    assert_eq!(checker.interface_default_bindings.len(), 1);

    let mut checker = TypeChecker::new(&module, &layouts);
    let mut implemented = HashSet::new();
    implemented.insert("Condition".to_string());
    checker.register_interface_default_provider(
        "IFace",
        InterfaceDefaultProvider {
            method: "Run".to_string(),
            symbol: "ext_impl".to_string(),
            kind: InterfaceDefaultKind::Extension,
            conditions: vec!["Condition".to_string()],
            span: None,
            origin: "extension".to_string(),
        },
    );
    assert!(checker.try_apply_interface_default("Impl", "IFace", "Run", &implemented, None));
    assert_eq!(checker.interface_default_bindings.len(), 1);

    let mut checker = TypeChecker::new(&module, &layouts);
    let mut implemented = HashSet::new();
    implemented.insert("Cond".to_string());
    checker.register_interface_default_provider(
        "IFace",
        InterfaceDefaultProvider {
            method: "Run".to_string(),
            symbol: "first".to_string(),
            kind: InterfaceDefaultKind::Extension,
            conditions: vec!["Cond".to_string()],
            span: None,
            origin: "extA".to_string(),
        },
    );
    checker.register_interface_default_provider(
        "IFace",
        InterfaceDefaultProvider {
            method: "Run".to_string(),
            symbol: "second".to_string(),
            kind: InterfaceDefaultKind::Extension,
            conditions: vec!["Cond".to_string()],
            span: None,
            origin: "extB".to_string(),
        },
    );
    assert!(
        !checker.try_apply_interface_default("Impl", "IFace", "Run", &implemented, None),
        "ambiguous defaults should report diagnostics"
    );
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("DIM0003")),
        "expected ambiguity diagnostic, got {:?}",
        checker.diagnostics
    );
}

#[test]
fn resolve_type_for_expr_strips_pointer_suffixes() {
    let module = module_with_struct("Demo");
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);

    let mut ty = TypeExpr::simple("Demo");
    ty.suffixes.push(TypeSuffix::Pointer {
        mutable: false,
        modifiers: Vec::new(),
    });
    let resolution = checker.resolve_type_for_expr(&ty, None, None);
    assert!(matches!(
        resolution,
        ImportResolution::Found(found) if found == "Demo"
    ));
}

#[test]
fn effect_matching_walks_layout_hierarchy() {
    let module = Module::new(None);
    let mut layouts = TypeLayoutTable::default();
    layouts
        .types
        .insert("Root".to_string(), empty_struct_layout("Root", Vec::new()));
    layouts.types.insert(
        "Child".to_string(),
        empty_struct_layout("Child", vec!["Root".to_string()]),
    );

    let checker = TypeChecker::new(&module, &layouts);
    assert!(checker.effect_inherits("Child", "Root"));
    assert!(!checker.effect_inherits("Child", "MissingEffect"));
}
