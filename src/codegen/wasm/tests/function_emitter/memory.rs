#![cfg(test)]

use super::common::*;
use super::helpers::*;
use crate::codegen::wasm::emitter::function::{
    ElemSize, MemoryAccess, VecIndexAccess, VecIndexKind,
};
use crate::codegen::wasm::{RuntimeHook, push_i32_const};
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, Mutability, Operand, Place, PointerTy,
    ProjectionElem, Rvalue, Statement, StatementKind, StructLayout, Terminator, Ty, TypeLayout,
    TypeLayoutTable, TypeRepr,
};

#[test]
fn lower_address_of_assignments_load_struct_pointer() {
    let (layouts, function, source_local, dest_local) = address_of_assignment_fixture();
    let (harness, func_name) = harness_with_layouts(function, layouts);
    let (source_pointer_index, dest_index) = harness
        .with_emitter(
            &func_name,
            |_| None,
            |emitter| {
                let ptr = emitter
                    .pointer_local_index(source_local)
                    .expect("struct local should have pointer slot");
                let dest = emitter
                    .local_index(dest_local)
                    .expect("destination pointer local missing");
                (ptr, dest)
            },
        )
        .unwrap_or_else(|err| panic!("construct emitter: {err}"));

    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit body: {err}"));

    let mut pointer_load = vec![0x20];
    pointer_load.extend(leb_u32(source_pointer_index));
    assert!(
        contains_bytes(&body, &pointer_load),
        "address-of lowering should load the aggregate pointer"
    );

    let mut dest_store = vec![0x21];
    dest_store.extend(leb_u32(dest_index));
    assert!(
        contains_bytes(&body, &dest_store),
        "address-of lowering should store the pointer into the destination local"
    );
}

#[test]
fn emit_pointer_expression_emits_local_get_only_for_zero_offset() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let pointer_local = emitter.locals[2].expect("pointer local expected");
            let access = MemoryAccess {
                pointer_local,
                offset: 0,
                value_ty: Ty::named("int"),
                vec_index: None,
                pointer_steps: None,
                load_pointer_from_slot: false,
                from_scalar_value: false,
            };
            let mut buf = Vec::new();
            emitter
                .emit_pointer_expression(&mut buf, &access)
                .expect("emit pointer expression");
            let mut expected = vec![0x20];
            expected.extend(leb_u32(pointer_local));
            assert_eq!(
                buf, expected,
                "zero-offset pointer expressions should only get the base pointer"
            );
        },
    );
}

#[test]
fn emit_pointer_expression_adds_offset_when_needed() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let pointer_local = emitter.locals[2].expect("pointer local expected");
            let access = MemoryAccess {
                pointer_local,
                offset: 8,
                value_ty: Ty::named("int"),
                vec_index: None,
                pointer_steps: None,
                load_pointer_from_slot: false,
                from_scalar_value: false,
            };
            let mut buf = Vec::new();
            emitter
                .emit_pointer_expression(&mut buf, &access)
                .expect("emit pointer expression");
            let mut expected = vec![0x20];
            expected.extend(leb_u32(pointer_local));
            expected.extend([0x41, 0x08, 0x6A]);
            assert_eq!(
                buf, expected,
                "non-zero offsets should add the computed displacement"
            );
        },
    );
}

#[test]
fn emit_pointer_expression_handles_vec_index() {
    let (layouts, function) = struct_projection_fixture();
    let (harness, func_name) = harness_with_layouts(function, layouts);
    let panic_hook = harness
        .runtime_hooks()
        .get(&RuntimeHook::Panic.qualified_name())
        .copied()
        .expect("panic hook index present");
    harness
        .with_emitter(
            &func_name,
            |_| None,
            |emitter| {
                let pointer_local = emitter.locals[2].expect("pointer local expected");
                let index_local = emitter
                    .local_index(LocalId(3))
                    .expect("scalar local should have wasm slot");

                let access = MemoryAccess {
                    pointer_local,
                    offset: 4,
                    value_ty: Ty::named("int"),
                    vec_index: Some(VecIndexAccess {
                        index_local: LocalId(3),
                        pre_offset: 12,
                        kind: VecIndexKind::Vec,
                        ptr_offset: 0,
                        len_offset: 4,
                        elem_size: ElemSize::Field(8),
                        load_base_from_slot: false,
                    }),
                    pointer_steps: None,
                    load_pointer_from_slot: false,
                    from_scalar_value: false,
                };

                let mut buf = Vec::new();
                emitter
                    .emit_pointer_expression(&mut buf, &access)
                    .expect("emit pointer expression");

                let mut base_prefix = vec![0x20];
                base_prefix.extend(leb_u32(pointer_local));
                assert!(buf.starts_with(&base_prefix));

                let mut pre_offset_pattern = Vec::new();
                push_i32_const(&mut pre_offset_pattern, 12);
                pre_offset_pattern.push(0x6A);
                assert!(contains_bytes(&buf, &pre_offset_pattern));

                let mut index_pattern = vec![0x20];
                index_pattern.extend(leb_u32(index_local));
                assert!(contains_bytes(&buf, &index_pattern));

                let mut panic_pattern = Vec::new();
                push_i32_const(&mut panic_pattern, 0x2001);
                panic_pattern.push(0x10);
                panic_pattern.extend(leb_u32(panic_hook));
                assert!(
                    contains_bytes(&buf, &panic_pattern),
                    "vec indexing should call the panic hook when bounds checks fail"
                );

                let mut post_offset_pattern = Vec::new();
                push_i32_const(&mut post_offset_pattern, 4);
                post_offset_pattern.push(0x6A);
                assert!(contains_bytes(&buf, &post_offset_pattern));
            },
        )
        .unwrap_or_else(|err| panic!("construct emitter: {err}"));
}

#[test]
fn emit_pointer_expression_handles_const_elem_sizes() {
    let (layouts, function) = struct_projection_fixture();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let pointer_local = emitter.locals[2].expect("pointer local expected");
            let char_size = wasm_layouts()
                .size_and_align_for_ty(&Ty::named("char"))
                .expect("char layout")
                .0 as u32;
            let access = MemoryAccess {
                pointer_local,
                offset: 0,
                value_ty: Ty::named("char"),
                vec_index: Some(VecIndexAccess {
                    index_local: LocalId(3),
                    pre_offset: 0,
                    kind: VecIndexKind::String,
                    ptr_offset: 0,
                    len_offset: 4,
                    elem_size: ElemSize::Const(char_size),
                    load_base_from_slot: false,
                }),
                pointer_steps: None,
                load_pointer_from_slot: false,
                from_scalar_value: false,
            };
            let mut buf = Vec::new();
            emitter
                .emit_pointer_expression(&mut buf, &access)
                .expect("emit pointer expression");
            let mut const_pattern = Vec::new();
            push_i32_const(&mut const_pattern, char_size as i32);
            assert!(
                contains_bytes(&buf, &const_pattern),
                "const element sizes should emit an immediate multiplier"
            );
        },
    );
}

#[test]
fn compute_projection_offset_sets_vec_index_for_arrays() {
    let (layouts, function) = array_index_fixture();
    let array_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let plan = emitter
                .compute_projection_offset(&array_ty, &[ProjectionElem::Index(LocalId(2))])
                .expect("compute projection");
            assert!(
                plan.vec_index.is_some(),
                "array indexing should record vector index metadata"
            );
            assert_eq!(
                plan.value_ty.canonical_name(),
                "int",
                "projection should resolve array element type"
            );
        },
    );
}

#[test]
fn compute_projection_offset_sets_vec_index_for_strings() {
    let (layouts, function) = string_index_fixture();
    let string_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let plan = emitter
                .compute_projection_offset(&string_ty, &[ProjectionElem::Index(LocalId(2))])
                .expect("compute projection");
            let access = plan
                .vec_index
                .expect("string indexing should record metadata");
            assert!(
                matches!(access.kind, VecIndexKind::String),
                "string indexing should record the string vec index kind"
            );
            assert!(
                matches!(access.elem_size, ElemSize::Const(value) if value > 0),
                "string element sizes should be constant"
            );
            assert_eq!(
                plan.value_ty.canonical_name(),
                "char",
                "string projection should resolve to char elements"
            );
        },
    );
}

#[test]
fn compute_projection_offset_sets_vec_index_for_str() {
    let (layouts, function) = str_index_fixture();
    let str_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let plan = emitter
                .compute_projection_offset(&str_ty, &[ProjectionElem::Index(LocalId(2))])
                .expect("compute projection");
            let access = plan.vec_index.expect("str indexing should record metadata");
            assert!(
                matches!(access.kind, VecIndexKind::Str),
                "borrowed string indexing should record the str vec index kind"
            );
            assert!(
                matches!(access.elem_size, ElemSize::Const(value) if value > 0),
                "str element sizes should be constant"
            );
            assert_eq!(
                plan.value_ty.canonical_name(),
                "char",
                "str projection should resolve to char elements"
            );
        },
    );
}

#[test]
fn compute_projection_offset_supports_enum_variant_field() {
    let (layouts, function) = enum_projection_fixture();
    let base_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut projection = Vec::new();
            projection.push(ProjectionElem::Downcast { variant: 0 });
            projection.push(ProjectionElem::FieldNamed("Value".into()));
            let plan = emitter
                .compute_projection_offset(&base_ty, &projection)
                .expect("enum projection lowering should succeed");
            assert_eq!(
                plan.offset, 0,
                "enum variant fields should be positioned at offset zero in this fixture"
            );
            assert_eq!(
                plan.value_ty,
                Ty::named("int"),
                "projected type should match field layout"
            );
        },
    );
}

#[test]
fn compute_projection_offset_errors_on_enum_without_downcast() {
    let (layouts, function) = enum_projection_fixture();
    let base_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let mut projection = Vec::new();
            projection.push(ProjectionElem::FieldNamed("Value".into()));
            let err = emitter
                .compute_projection_offset(&base_ty, &projection)
                .expect_err("enum field access without downcast should fail");
            assert!(
                format!("{err}").contains("unable to resolve field"),
                "unexpected error message: {err}"
            );
        },
    );
}

fn address_of_assignment_fixture() -> (TypeLayoutTable, MirFunction, LocalId, LocalId) {
    let layouts = sample_pair_layout();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let pair_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("pair".into()),
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Local,
    ));
    let ptr_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("pair_ptr".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("Demo::Pair"), false))),
        false,
        None,
        LocalKind::Local,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(pair_local),
    });
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(ptr_local),
    });
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(ptr_local),
            value: Rvalue::AddressOf {
                mutability: Mutability::Immutable,
                place: Place::new(pair_local),
            },
        },
    });
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(ptr_local),
    });
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(pair_local),
    });
    block0.terminator = Some(Terminator::Return);
    body.blocks.push(block0);

    let function = MirFunction {
        name: "Demo::CaptureAddress".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
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

    (layouts, function, pair_local, ptr_local)
}

#[test]
fn compute_projection_offset_supports_union_field() {
    let (layouts, function) = union_projection_fixture();
    let base_ty = function.body.locals[1].ty.clone();
    with_emitter_using_layouts(
        layouts,
        function,
        |_| None,
        |emitter| {
            let projection = vec![ProjectionElem::UnionField {
                index: 0,
                name: "Data".into(),
            }];
            let plan = emitter
                .compute_projection_offset(&base_ty, &projection)
                .expect("union field lowering should succeed");
            assert_eq!(
                plan.offset, 0,
                "union fixture stores payload at offset zero"
            );
            assert_eq!(
                plan.value_ty,
                Ty::named("int"),
                "projected type should match union payload type"
            );
        },
    );
}
fn zero_init_fixture() -> (TypeLayoutTable, MirFunction) {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("slot".into()),
        Ty::named("Sample::Holder"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::ZeroInit {
            place: Place::new(LocalId(1)),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let mut layouts = wasm_layouts();
    layouts.types.insert(
        "Sample::Holder".into(),
        TypeLayout::Struct(StructLayout {
            name: "Sample::Holder".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
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

    (
        layouts,
        MirFunction {
            name: "Demo::ZeroInit".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: Vec::new(),
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),

                lends_to_return: None,

                variadic: false,
            },
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        },
    )
}

fn zero_init_raw_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = wasm_layouts();
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let pointer_ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true)));
    body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        pointer_ty,
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("len".into()),
        Ty::named("usize"),
        false,
        None,
        LocalKind::Arg(1),
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::ZeroInitRaw {
            pointer: Operand::Copy(Place::new(LocalId(1))),
            length: Operand::Copy(Place::new(LocalId(2))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    (
        layouts,
        MirFunction {
            name: "Demo::ZeroInitRaw".into(),
            kind: FunctionKind::Function,
            signature: FnSig {
                params: vec![
                    Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
                    Ty::named("usize"),
                ],
                ret: Ty::Unit,
                abi: Abi::Chic,
                effects: Vec::new(),

                lends_to_return: None,

                variadic: false,
            },
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        },
    )
}

#[test]
fn zero_init_uses_memory_fill() {
    let (layouts, function) = zero_init_fixture();
    let (harness, func_name) = harness_with_layouts(function, layouts);
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit body: {err}"));
    assert!(
        contains_bytes(&body, &[0xFC, 0x0B]),
        "ZeroInit should emit wasm memory.fill instructions"
    );
}

#[test]
fn zero_init_raw_uses_memory_fill() {
    let (layouts, function) = zero_init_raw_fixture();
    let (harness, func_name) = harness_with_layouts(function, layouts);
    let body = harness
        .emit_body_with(&func_name, |_| None)
        .unwrap_or_else(|err| panic!("emit body: {err}"));
    assert!(
        contains_bytes(&body, &[0xFC, 0x0B]),
        "ZeroInitRaw should emit wasm memory.fill instructions"
    );
}
