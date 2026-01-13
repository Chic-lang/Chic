use super::*;
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::LlvmFunctionSignature;
use crate::codegen::llvm::types::map_type_owned;
use crate::mir::{
    ArrayTy, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, ParamMode,
    Place, ProjectionElem, Ty, TypeLayout, TypeLayoutTable,
};
use crate::mir::{AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, TypeRepr};
use crate::target::TargetArch;
use std::collections::{BTreeSet, HashMap, HashSet};

fn make_layouts() -> TypeLayoutTable {
    let mut layouts = TypeLayoutTable::default();
    let tuple_name = Ty::Tuple(crate::mir::TupleTy::new(vec![
        Ty::named("int"),
        Ty::named("int"),
    ]))
    .canonical_name();
    layouts.types.insert(
        tuple_name.clone(),
        TypeLayout::Struct(StructLayout {
            name: tuple_name,
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "0".into(),
                    ty: Ty::named("int"),
                    index: 0,
                    offset: Some(0),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "1".into(),
                    ty: Ty::named("int"),
                    index: 1,
                    offset: Some(4),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: Some(8),
            align: Some(4),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    let span_ty = Ty::Span(crate::mir::SpanTy {
        element: Box::new(Ty::named("int")),
    });
    let span_name = span_ty.canonical_name();
    layouts.types.insert(
        span_name.clone(),
        TypeLayout::Struct(StructLayout {
            name: span_name,
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "ptr".into(),
                    ty: Ty::Pointer(Box::new(crate::mir::PointerTy::new(Ty::named("int"), true))),
                    index: 0,
                    offset: Some(0),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "len".into(),
                    ty: Ty::named("int"),
                    index: 1,
                    offset: Some(8),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "elem_size".into(),
                    ty: Ty::named("int"),
                    index: 2,
                    offset: Some(16),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: Some(24),
            align: Some(8),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    layouts.types.insert(
        "Demo::Point".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo::Point".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "x".into(),
                    ty: Ty::named("int"),
                    index: 0,
                    offset: Some(0),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "y".into(),
                    ty: Ty::named("int"),
                    index: 1,
                    offset: Some(4),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: Some(8),
            align: Some(4),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    layouts.types.insert(
        "Demo".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![FieldLayout {
                name: "val".into(),
                ty: Ty::named("int"),
                index: 0,
                offset: Some(0),
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
            size: Some(4),
            align: Some(4),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    layouts.types.insert(
        "Demo::Droppable".into(),
        TypeLayout::Struct(StructLayout {
            name: "Demo::Droppable".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![],
            positional: Vec::new(),
            list: None,
            size: Some(0),
            align: Some(1),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: Some("Demo::drop".into()),
            class: None,
        }),
    );

    layouts
}

fn layouts_without_readonly_span() -> TypeLayoutTable {
    let mut layouts = make_layouts();
    let span = Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy {
        element: Box::new(Ty::named("int")),
    })
    .canonical_name();
    layouts.types.remove(&span);
    layouts
}

fn with_emitter_decls<F, R>(
    locals: Vec<LocalDecl>,
    ptrs: Vec<Option<&str>>,
    mut layouts: TypeLayoutTable,
    f: F,
) -> (R, String, BTreeSet<&'static str>)
where
    F: FnOnce(&mut FunctionEmitter<'_>) -> R,
{
    layouts.finalize_auto_traits();
    let mut body = MirBody::new(0, None);
    body.locals.extend(locals.into_iter());
    let function = MirFunction {
        name: "Demo::method".into(),
        kind: FunctionKind::Method,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };
    let mut externals: Box<BTreeSet<&'static str>> = Box::new(BTreeSet::new());
    let mut metadata = MetadataRegistry::new();
    let signatures: Box<HashMap<String, LlvmFunctionSignature>> = Box::new(HashMap::new());
    let vtable_symbols: Box<HashSet<String>> = Box::new(HashSet::new());
    let trait_vtables: Box<Vec<_>> = Box::new(Vec::new());
    let class_vtables: Box<Vec<_>> = Box::new(Vec::new());
    let statics: Box<Vec<crate::mir::StaticVar>> = Box::new(Vec::new());
    let str_literals: Box<
        HashMap<crate::mir::StrId, crate::codegen::llvm::emitter::literals::StrLiteralInfo>,
    > = Box::new(HashMap::new());
    let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
    let mut emitter = FunctionEmitter::new(
        Box::leak(Box::new(function)),
        &signatures,
        &mut externals,
        &vtable_symbols,
        &trait_vtables,
        &class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        TargetArch::Aarch64,
        &target,
        &statics,
        &str_literals,
        Box::leak(Box::new(layouts)),
        &mut metadata,
        None,
    );
    let llvm_tys = emitter
        .function
        .body
        .locals
        .iter()
        .map(|decl| map_type_owned(&decl.ty, Some(emitter.type_layouts)).unwrap_or(None))
        .collect();
    emitter.set_local_types_for_tests(llvm_tys);
    emitter.local_ptrs = ptrs.into_iter().map(|p| p.map(str::to_string)).collect();
    let result = f(&mut emitter);
    let ir = emitter.ir().to_string();
    (result, ir, (*externals).clone())
}

fn with_emitter<F, R>(
    locals: Vec<Ty>,
    ptrs: Vec<Option<&str>>,
    layouts: TypeLayoutTable,
    f: F,
) -> (R, String, BTreeSet<&'static str>)
where
    F: FnOnce(&mut FunctionEmitter<'_>) -> R,
{
    let decls = locals
        .into_iter()
        .map(|ty| LocalDecl::new(None, ty, false, None, LocalKind::Local))
        .collect();
    with_emitter_decls(decls, ptrs, layouts, f)
}

#[test]
fn field_projections_on_struct() {
    let layouts = make_layouts();
    let (result, ir, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            let res = emitter.field_info_by_index(&Ty::named("Demo::Point"), 1);
            let ptr = emitter
                .place_ptr(&Place {
                    local: LocalId(0),
                    projection: vec![ProjectionElem::Field(1)],
                })
                .expect("place ptr");
            (res, ptr)
        },
    );
    let (offset, ty) = result.0.expect("field info");
    assert_eq!(offset, 4);
    assert_eq!(ty.canonical_name(), "int");

    let ptr = result.1;
    assert!(
        ptr.contains("%"),
        "place_ptr should produce a temporary pointer name"
    );
    assert!(
        ir.contains("add i64"),
        "offset_ptr should emit pointer arithmetic"
    );
}

#[test]
fn span_index_projection_emits_bounds_and_mul() {
    let layouts = make_layouts();
    let (ptr, ir, _) = with_emitter(
        vec![
            Ty::Span(crate::mir::SpanTy {
                element: Box::new(Ty::named("int")),
            }),
            Ty::named("int"),
        ],
        vec![Some("%span"), Some("%idx")],
        layouts,
        |emitter| {
            emitter.local_tys[1] = Some("i32".into());
            emitter.local_ptrs[1] = Some("%idx_ptr".into());

            emitter
                .place_ptr(&Place {
                    local: LocalId(0),
                    projection: vec![ProjectionElem::Index(LocalId(1))],
                })
                .expect("span index projection")
        },
    );
    assert!(
        ir.contains("call i32 @chic_rt_panic(i32 8195)"),
        "bounds check should panic on out-of-bounds (span code)"
    );
    assert!(ptr.contains("%"), "returned pointer should be a temp");
}

#[test]
fn missing_layout_reports_error() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_name(&Ty::named("Unknown"), "field"),
    );
    let err = err.expect_err("missing layout should error");
    assert!(
        err.to_string().contains("type layout"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn deref_named_pointer_resolves_element() {
    let layouts = TypeLayoutTable::default();
    let (ty, _, _) = with_emitter(
        vec![Ty::named("Foo*")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.deref_ty(&Ty::named("Foo*")),
    );
    let ty = ty.expect("deref");
    assert_eq!(ty.canonical_name(), "Foo");
}

#[test]
fn offset_ptr_zero_shortcuts() {
    let layouts = TypeLayoutTable::default();
    let (ptr, _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.offset_ptr("%p", 0).expect("offset"),
    );
    assert_eq!(ptr, "%p");
}

#[test]
fn decimal_unknown_field_offsets() {
    let layouts = TypeLayoutTable::default();
    let ((status_offset, status_ty), _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_name(&Ty::Unknown, "Status").unwrap(),
    );
    assert_eq!(status_offset, 0);
    assert_eq!(
        status_ty.canonical_name(),
        "Std::Numeric::Decimal::DecimalStatus"
    );
}

#[test]
fn readonly_span_without_layout_uses_handle() {
    let layouts = layouts_without_readonly_span();
    let (result, _, _) = with_emitter(
        vec![Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy {
            element: Box::new(Ty::named("int")),
        })],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.field_info_by_name(
                &Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy {
                    element: Box::new(Ty::named("int")),
                }),
                "Handle",
            )
        },
    );
    let (offset, ty) = result.expect("fallback handle");
    assert_eq!(offset, 0);
    assert_eq!(ty.canonical_name(), "Std::Span::ReadOnlySpanPtr");
}

#[test]
fn readonly_span_without_layout_raw_uses_fallback() {
    let layouts = layouts_without_readonly_span();
    let (result, _, _) = with_emitter(
        vec![Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy {
            element: Box::new(Ty::named("int")),
        })],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.field_info_by_name(
                &Ty::ReadOnlySpan(crate::mir::ReadOnlySpanTy {
                    element: Box::new(Ty::named("int")),
                }),
                "Raw",
            )
        },
    );
    let (offset, ty) = result.expect("fallback raw");
    assert_eq!(offset, 0);
    assert_eq!(ty.canonical_name(), "Std::Span::ReadOnlySpanPtr");
}

#[test]
fn inline_index_projection_uses_const_elem_size_for_string() {
    let layouts = TypeLayoutTable::default();
    let (_, ir, externals) = with_emitter(vec![Ty::String], vec![Some("%s")], layouts, |emitter| {
        emitter
            .inline_index_projection("%s", &Ty::String, "5", "string", STRING_BOUNDS_PANIC_CODE)
            .expect("index")
    });
    assert!(
        ir.contains("call i32 @chic_rt_panic(i32 8197)"),
        "string bounds panic should be emitted"
    );
    assert!(externals.contains("chic_rt_panic"));
    assert!(
        ir.contains("mul i64 5, 2"),
        "const elem size (char) should scale index"
    );
}

#[test]
fn inline_index_projection_errors_without_elem_size_metadata() {
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "CustomSpan<int>".into(),
        TypeLayout::Struct(StructLayout {
            name: "CustomSpan<int>".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "ptr".into(),
                    ty: Ty::Pointer(Box::new(crate::mir::PointerTy::new(Ty::named("int"), true))),
                    index: 0,
                    offset: Some(0),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "len".into(),
                    ty: Ty::named("usize"),
                    index: 1,
                    offset: Some(8),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: Some(16),
            align: Some(8),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
    let (err, _, _) = with_emitter(
        vec![Ty::named("CustomSpan<int>")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.inline_index_projection(
                "%p",
                &Ty::named("CustomSpan<int>"),
                "1",
                "custom",
                0x1234,
            )
        },
    );
    let err = err.expect_err("missing elem_size should error");
    assert!(
        err.to_string().contains("missing elem_size metadata"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn place_ptr_handles_named_fields() {
    let layouts = make_layouts();
    let (ptr, ir, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%point")],
        layouts,
        |emitter| {
            emitter
                .place_ptr(&Place {
                    local: LocalId(0),
                    projection: vec![ProjectionElem::FieldNamed("y".into())],
                })
                .expect("field named")
        },
    );
    assert!(ptr.contains("%"), "should produce temporary pointer");
    assert!(ir.contains("add i64"), "should offset into struct field");
}

#[test]
fn place_ptr_index_on_non_container_errors() {
    let layouts = make_layouts();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Unknown"), Ty::named("int")],
        vec![Some("%base"), Some("%idx")],
        layouts,
        |emitter| {
            emitter.local_tys[1] = Some("i32".into());
            emitter.local_ptrs[1] = Some("%idx_ptr".into());
            emitter.place_ptr(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Index(LocalId(1))],
            })
        },
    );
    let err = err.expect_err("unsupported index type");
    assert!(
        err.to_string()
            .contains("index projection on unsupported type")
    );
}

#[test]
fn deref_ty_rejects_plain_named() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Foo")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.deref_ty(&Ty::named("Foo")),
    );
    let err = err.expect_err("should error");
    assert!(
        err.to_string()
            .contains("deref projection applied to non-pointer")
    );
}

#[test]
fn projection_offset_rejects_index_on_non_container() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Foo")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.projection_offset(&Ty::named("Foo"), &[ProjectionElem::Index(LocalId(0))])
        },
    );
    let err = err.expect_err("index on unsupported type");
    assert!(
        err.to_string()
            .contains("index projection on unsupported type")
    );
}

#[test]
fn store_place_handles_reference_params() {
    let layouts = TypeLayoutTable::default();
    let mut decl = LocalDecl::new(None, Ty::named("int"), false, None, LocalKind::Arg(0));
    decl.param_mode = Some(ParamMode::Ref);
    let (_, ir, _) = with_emitter_decls(vec![decl], vec![Some("%ref_slot")], layouts, |emitter| {
        emitter.local_tys[0] = Some("ptr".into());
        emitter
            .store_place(
                &Place {
                    local: LocalId(0),
                    projection: Vec::new(),
                },
                &ValueRef::new_literal("42".into(), "i32"),
            )
            .expect("store");
    });
    assert!(
        ir.contains("load ptr, ptr %ref_slot"),
        "reference param should be loaded before storing"
    );
    assert!(
        ir.contains("store i32 42"),
        "value should be stored through reference"
    );
}

#[test]
fn load_struct_field_errors_when_unmapped() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.load_struct_field("%p", 0, &Ty::Unknown),
    );
    let err = err.expect_err("missing mapping");
    assert!(
        err.to_string()
            .contains("unknown type cannot be lowered to the LLVM backend")
    );
}

#[test]
fn tuple_field_info_by_index_uses_layout() {
    let layouts = make_layouts();
    let (result, _, _) = with_emitter(
        vec![Ty::Tuple(crate::mir::TupleTy::new(vec![
            Ty::named("int"),
            Ty::named("int"),
        ]))],
        vec![Some("%tup")],
        layouts,
        |emitter| {
            emitter.field_info_by_index(
                &Ty::Tuple(crate::mir::TupleTy::new(vec![
                    Ty::named("int"),
                    Ty::named("int"),
                ])),
                1,
            )
        },
    );
    let (offset, ty) = result.expect("field info");
    assert_eq!(offset, 4);
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn field_info_by_index_errors_for_unsupported() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_index(&Ty::Unit, 0),
    );
    let err = err.expect_err("unsupported type");
    assert!(
        err.to_string()
            .contains("field projection on unsupported type")
    );
}

#[test]
fn field_info_by_name_resolves_self_layout() {
    let layouts = make_layouts();
    let (result, _, _) = with_emitter(
        vec![Ty::named("Demo")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_name(&Ty::named("Self"), "val"),
    );
    let (offset, ty) = result.expect("self field");
    assert_eq!(offset, 0);
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn load_local_as_i64_passthrough() {
    let layouts = TypeLayoutTable::default();
    let (_, ir, _) = with_emitter(
        vec![Ty::named("long")],
        vec![Some("%i64_ptr")],
        layouts,
        |emitter| emitter.load_local_as_i64(LocalId(0)).expect("i64 load"),
    );
    assert!(
        ir.contains("load i64, ptr %i64_ptr"),
        "should load i64 without casting"
    );
}

#[test]
fn load_struct_usize_prefers_native_i64() {
    let layouts = TypeLayoutTable::default();
    let (_, ir, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%ptr")],
        layouts,
        |emitter| emitter.load_struct_usize("%ptr", 8, &Ty::named("long")),
    );
    assert!(
        ir.contains("load i64"),
        "should load i64 sized field without zero-extend"
    );
}

#[test]
fn place_ptr_loads_pointer_wrapped_struct() {
    let layouts = make_layouts();
    let (ptr, ir, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%outer")],
        layouts,
        |emitter| {
            emitter.local_tys[0] = Some("ptr".into());
            emitter.place_ptr(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Field(0)],
            })
        },
    );
    let ptr = ptr.expect("place ptr");
    assert!(ptr.contains("%"), "should produce temporary");
    assert!(
        ir.contains("load ptr, ptr %outer"),
        "should load inner pointer when local type is ptr"
    );
}

#[test]
fn deinit_symbol_for_ty_reads_layout() {
    let layouts = make_layouts();
    let (symbol, _, _) = with_emitter(
        vec![Ty::named("Demo::Droppable")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter
                .dispose_symbol_for_ty(&Ty::named("Demo::Droppable"))
                .cloned()
        },
    );
    assert_eq!(symbol.as_deref(), Some("Demo::drop"));
}

#[test]
fn field_info_by_index_resolves_self() {
    let layouts = make_layouts();
    let (result, _, _) = with_emitter(
        vec![Ty::named("Demo")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_index_named("Self", 0),
    );
    let (offset, ty) = result.expect("self field");
    assert_eq!(offset, 0);
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn field_info_by_name_missing_field_errors() {
    let layouts = make_layouts();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_name(&Ty::named("Demo::Point"), "z"),
    );
    let err = err.expect_err("missing field");
    assert!(err.to_string().contains("field `z` missing"));
}

#[test]
fn place_type_with_projection_maps_type() {
    let layouts = make_layouts();
    let (ty, _, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.place_type(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Field(1)],
            })
        },
    );
    assert_eq!(ty.unwrap().as_deref(), Some("i32"));
}

#[test]
fn reference_param_helpers_detect_modes() {
    let layouts = TypeLayoutTable::default();
    let mut decl = LocalDecl::new(None, Ty::named("int"), false, None, LocalKind::Arg(0));
    decl.param_mode = Some(ParamMode::In);
    let (modes, _, _) = with_emitter_decls(vec![decl], vec![Some("%p")], layouts, |emitter| {
        (
            emitter.local_param_mode(0),
            emitter.is_reference_param(0),
            emitter.param_value_type(0),
        )
    });
    assert_eq!(modes.0, Some(ParamMode::In));
    assert!(modes.1, "ParamMode::In counts as reference");
    assert_eq!(modes.2.expect("value type"), "i32");
}

#[test]
fn projection_offset_handles_nested_array() {
    let layouts = TypeLayoutTable::default();
    let (result, _, _) = with_emitter(
        vec![Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 2))],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.projection_offset(
                &Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 2)),
                &[ProjectionElem::Index(LocalId(0))],
            )
        },
    );
    let (_, ty) = result.expect("array projection");
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn field_info_by_index_missing_tuple_layout_errors() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::Tuple(crate::mir::TupleTy::new(vec![Ty::named("int")]))],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.field_info_by_index(
                &Ty::Tuple(crate::mir::TupleTy::new(vec![Ty::named("int")])),
                0,
            )
        },
    );
    let err = err.expect_err("missing tuple layout");
    assert!(err.to_string().contains("tuple layout"));
}

#[test]
fn field_info_by_index_named_missing_layout_errors() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Missing")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_index_named("Missing", 0),
    );
    assert!(
        err.expect_err("missing layout")
            .to_string()
            .contains("not recorded")
    );
}

#[test]
fn field_info_by_name_unknown_type_rejects_other_fields() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(vec![Ty::Unknown], vec![Some("%p")], layouts, |emitter| {
        emitter.field_info_by_name(&Ty::Unknown, "Other")
    });
    let err = err.expect_err("unknown field");
    assert!(
        err.to_string()
            .contains("named field projection on unsupported type")
    );
}

#[test]
fn field_info_by_name_span_missing_field_errors() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::Span(crate::mir::SpanTy {
            element: Box::new(Ty::named("int")),
        })],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.field_info_by_name(
                &Ty::Span(crate::mir::SpanTy {
                    element: Box::new(Ty::named("int")),
                }),
                "bogus",
            )
        },
    );
    let err = err.expect_err("bad span field");
    assert!(err.to_string().contains("missing on type"));
}

#[test]
fn field_info_by_name_str_fields_have_offsets() {
    let layouts = TypeLayoutTable::default();
    let (result, _, _) = with_emitter(vec![Ty::Str], vec![Some("%p")], layouts, |emitter| {
        emitter.field_info_by_name(&Ty::Str, "len")
    });
    let (offset, _) = result.expect("str field");
    assert!(offset > 0, "str len offset should be non-zero");
}

#[test]
fn field_info_by_name_decimal_alias_resolves() {
    let layouts = TypeLayoutTable::default();
    let (result, _, _) = with_emitter(
        vec![Ty::named("DecimalIntrinsicResult")],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.field_info_by_name(&Ty::named("DecimalIntrinsicResult"), "Value"),
    );
    let (offset, _) = result.expect("decimal result");
    assert!(offset > 0);
}

#[test]
fn field_info_by_index_vec_uses_layout() {
    let mut layouts = make_layouts();
    let vec_ty = Ty::Vec(crate::mir::VecTy::new(Box::new(Ty::named("int"))));
    let vec_name = vec_ty.canonical_name();
    layouts.types.insert(
        vec_name.clone(),
        TypeLayout::Struct(StructLayout {
            name: vec_name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "ptr".into(),
                    ty: Ty::named("ptr"),
                    index: 0,
                    offset: Some(0),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
                FieldLayout {
                    name: "len".into(),
                    ty: Ty::named("int"),
                    index: 1,
                    offset: Some(8),
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: Some(16),
            align: Some(8),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
    let (result, _, _) = with_emitter(vec![vec_ty.clone()], vec![Some("%v")], layouts, |emitter| {
        emitter.field_info_by_index(&vec_ty, 1)
    });
    let (offset, ty) = result.expect("vec layout");
    assert_eq!(offset, 8);
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn place_ptr_rejects_unsupported_projection() {
    let layouts = make_layouts();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.place_ptr(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Downcast { variant: 0 }],
            })
        },
    );
    let err = err.expect_err("unsupported projection");
    assert!(err.to_string().contains("not yet supported"));
}

#[test]
fn projection_offset_rejects_unsupported_projection() {
    let layouts = TypeLayoutTable::default();
    let (err, _, _) = with_emitter(
        vec![Ty::named("int")],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.projection_offset(
                &Ty::named("int"),
                &[ProjectionElem::Subslice { from: 0, to: 0 }],
            )
        },
    );
    let err = err.expect_err("unsupported projection");
    assert!(err.to_string().contains("unsupported projection"));
}

#[test]
fn place_ptr_index_on_str_emits_bounds() {
    let layouts = TypeLayoutTable::default();
    let (ptr, ir, _) = with_emitter(
        vec![Ty::Str, Ty::named("int")],
        vec![Some("%str"), Some("%idx")],
        layouts,
        |emitter| {
            emitter.local_tys[1] = Some("i32".into());
            emitter.local_ptrs[1] = Some("%idx_ptr".into());
            emitter.place_ptr(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Index(LocalId(1))],
            })
        },
    );
    assert!(ptr.is_ok());
    assert!(
        ir.contains("call i32 @chic_rt_panic(i32 8198)"),
        "str bounds panic should be emitted"
    );
}

#[test]
fn param_value_type_errors_on_unknown() {
    let layouts = TypeLayoutTable::default();
    let mut decl = LocalDecl::new(None, Ty::Unknown, false, None, LocalKind::Arg(0));
    decl.param_mode = Some(ParamMode::Ref);
    let (err, _, _) = with_emitter_decls(vec![decl], vec![Some("%p")], layouts, |emitter| {
        emitter.param_value_type(0)
    });
    assert!(err.is_err(), "unknown types should error");
}

#[test]
fn place_type_errors_for_missing_local_storage() {
    let layouts = make_layouts();
    let (err, _, _) = with_emitter(
        vec![Ty::named("Demo::Point")],
        vec![None],
        layouts,
        |emitter| {
            emitter.place_type(&Place {
                local: LocalId(1),
                projection: Vec::new(),
            })
        },
    );
    assert!(err.is_err());
}

#[test]
fn place_ptr_with_deref_loads_ref_target() {
    let layouts = make_layouts();
    let (ptr, ir, _) = with_emitter(
        vec![Ty::Ref(Box::new(crate::mir::RefTy::new(
            Ty::named("Demo::Point"),
            false,
        )))],
        vec![Some("%ref_ptr")],
        layouts,
        |emitter| {
            emitter.local_tys[0] = Some("ptr".into());
            emitter.place_ptr(&Place {
                local: LocalId(0),
                projection: vec![ProjectionElem::Deref, ProjectionElem::Field(0)],
            })
        },
    );
    let ptr = ptr.expect("deref ptr");
    assert!(ptr.contains("%"));
    assert!(ir.contains("load ptr, ptr %ref_ptr"));
}

#[test]
fn projection_offset_handles_deref_chain() {
    let layouts = make_layouts();
    let (result, _, _) = with_emitter(
        vec![Ty::Ref(Box::new(crate::mir::RefTy::new(
            Ty::named("Demo::Point"),
            false,
        )))],
        vec![Some("%p")],
        layouts,
        |emitter| {
            emitter.projection_offset(
                &Ty::Ref(Box::new(crate::mir::RefTy::new(
                    Ty::named("Demo::Point"),
                    false,
                ))),
                &[ProjectionElem::Deref, ProjectionElem::Field(1)],
            )
        },
    );
    let (offset, ty) = result.expect("deref offset");
    assert_eq!(offset, 4);
    assert_eq!(ty.canonical_name(), "int");
}

#[test]
fn deref_ty_handles_nullable() {
    let layouts = TypeLayoutTable::default();
    let (ty, _, _) = with_emitter(
        vec![Ty::Nullable(Box::new(Ty::named("Foo*")))],
        vec![Some("%p")],
        layouts,
        |emitter| emitter.deref_ty(&Ty::Nullable(Box::new(Ty::named("Foo*")))),
    );
    assert_eq!(ty.expect("nullable deref").canonical_name(), "Foo");
}
