#![allow(unused_imports)]

use super::common::{self, *};
use super::*;
use crate::chic_kind::ChicKind;
use crate::codegen::wasm::layout::is_scalar_named;
use crate::frontend::ast::{
    Block as AstBlock, FunctionDecl, Item as AstItem, Module as AstModule, NamespaceDecl,
    Signature, TypeExpr, Visibility,
};
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BinOp, BlockId, BorrowKind, BorrowOperand,
    ConstValue, EnumLayout, EnumVariantLayout, FieldLayout, FnSig, FunctionKind, GenericArg,
    LocalDecl, LocalId, LocalKind, MatchArm, MirBody, MirFunction, MirModule, Operand, Pattern,
    PatternBinding, PendingOperand, PendingRvalue, PendingTerminator, PendingTerminatorKind, Place,
    ProjectionElem, RegionVar, Rvalue, Statement, StatementKind, StructLayout, Terminator, TupleTy,
    Ty, TypeLayout, TypeLayoutTable, TypeRepr, UnOp, UnionFieldLayout, UnionFieldMode, UnionLayout,
    ValueCategory,
};
use crate::target::Target;
use std::collections::HashMap;
use std::fs;

#[test]
fn scalar_detection_matches_known_names() {
    assert!(is_scalar_named("int"));
    assert!(is_scalar_named("System::Double"));
    assert!(is_scalar_named("Std::Double"));
    assert!(!is_scalar_named("Demo::Pair"));
}

#[test]
fn local_requires_memory_distinguishes_scalars_and_aggregates() {
    let layouts = sample_pair_layout();
    assert!(
        !super::local_requires_memory(&Ty::named("int"), &layouts),
        "scalar locals should not require linear memory"
    );
    assert!(
        super::local_requires_memory(&Ty::named("Demo::Pair"), &layouts),
        "struct locals should require linear memory"
    );
    let empty = wasm_layouts();
    assert!(
        !super::local_requires_memory(&Ty::named("Demo::Pair"), &empty),
        "without layouts aggregates fall back to register locals"
    );
}

#[test]
fn local_requires_memory_treats_classes_as_scalars() {
    let layouts = sample_class_layout();
    assert!(
        !super::local_requires_memory(&Ty::named("Demo::Window"), &layouts),
        "class locals should store heap references in registers"
    );
}

#[test]
fn local_requires_memory_respects_tuple_layouts() {
    let layouts = tuple_layout_table();
    let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
    assert!(
        super::local_requires_memory(&tuple_ty, &layouts),
        "tuple locals should require linear memory when layouts are known"
    );
    let empty = wasm_layouts();
    assert!(
        !super::local_requires_memory(&tuple_ty, &empty),
        "tuple locals should fall back to registers without layout metadata"
    );
}

#[test]
fn local_requires_memory_flags_128_bit_integers() {
    let layouts = wasm_layouts();
    assert!(
        super::local_requires_memory(&Ty::named("i128"), &layouts),
        "128-bit signed integers should spill to linear memory"
    );
    assert!(
        super::local_requires_memory(&Ty::named("u128"), &layouts),
        "128-bit unsigned integers should spill to linear memory"
    );
    assert!(
        super::local_requires_memory(&Ty::named("Std::Int128"), &layouts),
        "namespaced 128-bit integers should also require memory slots"
    );
}

#[test]
fn compute_aggregate_allocation_supports_all_layouts() {
    let struct_layouts = sample_pair_layout();
    let allocation = super::compute_aggregate_allocation(&Ty::named("Demo::Pair"), &struct_layouts)
        .expect("struct allocation");
    assert_eq!(allocation.size, 8);
    assert_eq!(allocation.align, 4);

    let enum_layouts = enum_layout_table();
    let enum_alloc = super::compute_aggregate_allocation(&Ty::named("Demo::Choice"), &enum_layouts)
        .expect("enum allocation");
    assert_eq!(enum_alloc.size, 4);
    assert_eq!(enum_alloc.align, 4);

    let union_layouts = union_layout_table();
    let union_alloc =
        super::compute_aggregate_allocation(&Ty::named("Demo::UnionValue"), &union_layouts)
            .expect("union allocation");
    assert_eq!(union_alloc.size, 4);
    assert_eq!(union_alloc.align, 4);

    let empty = wasm_layouts();
    let wide_int = super::compute_aggregate_allocation(&Ty::named("i128"), &empty)
        .expect("i128 should reserve linear memory");
    assert_eq!(wide_int.size, 16);
    assert_eq!(wide_int.align, 16);
}

#[test]
fn compute_aggregate_allocation_requires_size_metadata() {
    let layouts = struct_without_size_layout();
    assert!(
        super::compute_aggregate_allocation(&Ty::named("Demo::Incomplete"), &layouts).is_none(),
        "allocation should fail without size metadata"
    );
    let empty = wasm_layouts();
    assert!(
        super::compute_aggregate_allocation(&Ty::named("Missing"), &empty).is_none(),
        "unknown layouts should not allocate memory"
    );
}

#[test]
fn compute_aggregate_allocation_rejects_overflowing_layouts() {
    let mut layouts = wasm_layouts();
    let huge_layout = StructLayout {
        name: "Huge::Struct".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: Some(usize::MAX),
        align: Some(usize::MAX),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert("Huge::Struct".into(), TypeLayout::Struct(huge_layout));
    assert!(
        super::compute_aggregate_allocation(&Ty::named("Huge::Struct"), &layouts).is_none(),
        "overflowing aggregate layout should not produce an allocation"
    );
}

#[test]
fn compute_aggregate_allocation_supports_tuple_layouts() {
    let layouts = tuple_layout_table();
    let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
    let allocation =
        super::compute_aggregate_allocation(&tuple_ty, &layouts).expect("tuple allocation");
    assert_eq!(allocation.size, 8);
    assert_eq!(allocation.align, 4);
}

#[test]
fn lookup_layout_resolves_async_task_instantiations() {
    let layouts = wasm_layouts();
    let bool_task = Ty::named_generic(
        "Std::Async::Task",
        vec![GenericArg::Type(Ty::named("bool"))],
    );
    let int_task = Ty::named_generic("Std::Async::Task", vec![GenericArg::Type(Ty::named("int"))]);
    assert!(
        super::lookup_layout(&layouts, &bool_task).is_some(),
        "expected layout metadata for Std.Async.Task<bool>"
    );
    assert!(
        super::lookup_layout(&layouts, &int_task).is_some(),
        "expected layout metadata for Std.Async.Task<int>"
    );
}

#[test]
fn lookup_layout_handles_unqualified_task_name() {
    let layouts = wasm_layouts();
    let task_ty = Ty::named("Task");
    let layout = super::lookup_layout(&layouts, &task_ty)
        .and_then(|entry| match entry {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
            _ => None,
        })
        .expect("expected async task layout");
    assert!(
        layout.fields.iter().any(|field| field.name == "Header"),
        "async task layout should expose the Header field"
    );
}

#[test]
fn map_type_handles_scalars_and_unit() {
    assert_eq!(super::map_type(&Ty::Unit), ValueType::I32);
    assert_eq!(super::map_type(&Ty::named("int")), ValueType::I32);
    assert_eq!(super::map_type(&Ty::named("i64")), ValueType::I64);
    assert_eq!(super::map_type(&Ty::named("float")), ValueType::F32);
    assert_eq!(super::map_type(&Ty::named("double")), ValueType::F64);
    assert_eq!(super::map_type(&Ty::Unknown), ValueType::I32);
}

#[test]
fn compute_projection_offset_supports_named_fields() {
    let (layouts, function) = struct_projection_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let projection = vec![ProjectionElem::FieldNamed("Y".into())];
    let plan = emitter
        .compute_projection_offset(&Ty::named("Demo::Pair"), &projection)
        .expect("named field projection");
    assert_eq!(plan.offset, 4, "expected offset for named field");
    assert_eq!(plan.value_ty, Ty::named("int"), "field type should be int");
}

#[test]
fn compute_projection_offset_supports_tuple_elements() {
    let (layouts, function) = tuple_copy_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
    let plan = emitter
        .compute_projection_offset(&tuple_ty, &[ProjectionElem::Field(1)])
        .expect("tuple element projection");
    assert_eq!(plan.offset, 4, "second tuple field should have offset 4");
    assert_eq!(
        plan.value_ty,
        Ty::named("int"),
        "tuple element type should be int"
    );
}

#[test]
fn compute_projection_offset_rejects_unknown_field() {
    let (layouts, function) = struct_projection_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let projection = vec![ProjectionElem::FieldNamed("Z".into())];
    let err = emitter
        .compute_projection_offset(&Ty::named("Demo::Pair"), &projection)
        .expect_err("unknown field should be rejected");
    let message = format!("{err}");
    assert!(
        message.contains("unable to resolve field `Z`"),
        "unexpected error for unknown field: {message}"
    );
}

#[test]
fn compute_projection_offset_rejects_missing_struct_layout() {
    let layouts = wasm_layouts();
    let function = simple_function("Demo::MissingLayout", FunctionKind::Function, Ty::Unit);
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let projection = vec![ProjectionElem::Field(0)];
    let err = emitter
        .compute_projection_offset(&Ty::named("Demo::Missing"), &projection)
        .expect_err("missing layout should be rejected");
    let message = format!("{err}");
    assert!(
        message.contains("unable to resolve field index 0"),
        "unexpected error for missing layout: {message}"
    );
}

#[test]
fn compute_projection_offset_rejects_tuple_without_layout() {
    let (_, function) = tuple_aggregate_fixture();
    let layouts = wasm_layouts();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");
    let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
    let projection = vec![ProjectionElem::Field(0)];
    let err = emitter
        .compute_projection_offset(&tuple_ty, &projection)
        .expect_err("tuple projection should require layout metadata");
    let message = format!("{err}");
    assert!(
        message.contains("unable to resolve field index 0"),
        "unexpected error message: {message}"
    );
}

#[test]
fn compute_projection_offset_rejects_large_offsets() {
    let mut layouts = wasm_layouts();
    let huge_layout = StructLayout {
        name: "Huge::Offsets".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![FieldLayout {
            name: "Field".into(),
            ty: Ty::named("int"),
            index: 0,
            offset: Some(usize::MAX),
            span: None,
            mmio: None,
            display_name: None,
            is_required: false,
            is_nullable: false,
            is_readonly: false,

            view_of: None,
        }],
        positional: Vec::new(),
        list: None,
        size: Some(usize::MAX),
        align: Some(1),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert("Huge::Offsets".into(), TypeLayout::Struct(huge_layout));

    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let function = simple_function("Main", FunctionKind::Function, Ty::Unit);
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let projection = vec![ProjectionElem::FieldNamed("Field".into())];
    let err = emitter
        .compute_projection_offset(&Ty::named("Huge::Offsets"), &projection)
        .expect_err("offset computation should overflow");
    assert!(
        format!("{err}").contains("addressable range"),
        "unexpected overflow error: {err}"
    );
}

#[test]
fn compute_projection_offset_rejects_complex_projections() {
    let (layouts, function) = struct_projection_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let projection = vec![ProjectionElem::Index(LocalId(0))];
    let err = emitter
        .compute_projection_offset(&Ty::named("Demo::Pair"), &projection)
        .expect_err("index projections are not supported yet");
    let message = format!("{err}");
    assert!(
        message.contains("complex projections") || message.contains("index projection"),
        "unexpected error for complex projection: {message}"
    );
}

#[test]
fn resolve_memory_access_returns_pointer_and_offset() {
    let (layouts, function) = struct_projection_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let mut place = Place::new(LocalId(2));
    place
        .projection
        .push(ProjectionElem::FieldNamed("Y".into()));
    let pointer_slot = emitter.locals[2].expect("pointer slot exists for struct local");
    let access = emitter
        .resolve_memory_access(&place)
        .expect("resolve memory access");
    assert_eq!(access.pointer_local, pointer_slot);
    assert_eq!(access.offset, 4);
    assert_eq!(access.value_ty, Ty::named("int"));
}

#[test]
fn resolve_memory_access_rejects_scalar_projection() {
    let function = scalar_local_function();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let layouts = wasm_layouts();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let mut place = Place::new(LocalId(1));
    place.projection.push(ProjectionElem::Field(0));
    let err = emitter
        .resolve_memory_access(&place)
        .expect_err("scalar locals cannot be projected");
    let message = format!("{err}");
    assert!(
        message.contains("projection"),
        "unexpected error for scalar projection: {message}"
    );
}

#[test]
fn resolve_memory_access_requires_field_offsets() {
    let (layouts, function) = struct_with_missing_offsets_fixture();
    let functions = HashMap::new();
    let function_return_tys = HashMap::new();
    let emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        None,
    )
    .expect("construct emitter");

    let mut place = Place::new(LocalId(1));
    place.projection.push(ProjectionElem::Field(0));
    let err = emitter
        .resolve_memory_access(&place)
        .expect_err("missing layout offsets should be rejected");
    let message = format!("{err}");
    assert!(
        message.contains("missing field offset"),
        "unexpected error for missing offsets: {message}"
    );
}

#[test]
fn lookup_layout_resolves_fully_qualified_names() {
    let layouts = struct_projection_fixture().0;
    let layout = super::lookup_layout(&layouts, &Ty::named("Demo::Pair"))
        .expect("lookup should succeed for fully qualified name");
    assert!(
        matches!(layout, TypeLayout::Struct(_)),
        "expected struct layout for Demo::Pair"
    );
}

#[test]
fn lookup_layout_supports_unique_suffixes() {
    let mut layouts = wasm_layouts();
    let layout = StructLayout {
        name: "Outer::Inner::Widget".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: Some(0),
        align: Some(1),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert(layout.name.clone(), TypeLayout::Struct(layout));

    let resolved =
        super::lookup_layout(&layouts, &Ty::named("Widget")).expect("suffix lookup should work");
    assert!(
        matches!(resolved, TypeLayout::Struct(_)),
        "expected struct layout for Widget suffix"
    );

    layouts.types.insert(
        "Another::Widget".into(),
        TypeLayout::Struct(StructLayout {
            name: "Another::Widget".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(0),
            align: Some(1),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_yes(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
    assert!(
        super::lookup_layout(&layouts, &Ty::named("Widget")).is_none(),
        "ambiguous suffix lookup should return None"
    );
}

#[test]
fn align_to_rounds_up_to_alignment() {
    assert_eq!(super::align_to(0, 1), 0);
    assert_eq!(super::align_to(4, 4), 4);
    assert_eq!(
        super::align_to(5, 4),
        8,
        "expected rounding up to next multiple"
    );
    assert_eq!(
        super::align_to(15, 8),
        16,
        "15 should align to 16 for alignment 8"
    );
}
