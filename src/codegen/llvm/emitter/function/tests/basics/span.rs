use super::super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, FnSig, FunctionKind, ListLayout,
    LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, Place, PointerTy,
    ReadOnlySpanTy, Rvalue, SpanTy, Statement, StatementKind, StructLayout, Terminator, Ty,
    TypeLayout, TypeLayoutTable, TypeRepr, make_field,
};
use std::collections::{BTreeSet, HashMap, HashSet};

fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        return value;
    }
    ((value + align - 1) / align) * align
}

fn span_layouts(element: &Ty) -> TypeLayoutTable {
    let mut layouts = TypeLayoutTable::default();
    let word = std::mem::size_of::<usize>();
    let align = std::mem::align_of::<usize>();

    let span_name = Ty::Span(SpanTy::new(Box::new(element.clone()))).canonical_name();
    let readonly_span_name =
        Ty::ReadOnlySpan(ReadOnlySpanTy::new(Box::new(element.clone()))).canonical_name();

    let mut value_ptr_fields = Vec::new();
    value_ptr_fields.push(make_field(
        "ptr",
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
        0,
        0,
    ));
    value_ptr_fields.push(make_field("size", Ty::named("usize"), 1, word));
    value_ptr_fields.push(make_field("align", Ty::named("usize"), 2, word * 2));
    let value_ptr_layout = StructLayout {
        name: "Std::Runtime::Collections::ValueMutPtr".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: value_ptr_fields.clone(),
        positional: Vec::new(),
        list: None,
        size: Some(word * 3),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts.types.insert(
        value_ptr_layout.name.clone(),
        TypeLayout::Struct(value_ptr_layout),
    );

    let value_const_layout = StructLayout {
        name: "Std::Runtime::Collections::ValueConstPtr".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: value_ptr_fields,
        positional: Vec::new(),
        list: None,
        size: Some(word * 3),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts.types.insert(
        value_const_layout.name.clone(),
        TypeLayout::Struct(value_const_layout),
    );

    let value_ptr_size = align_to(word * 3, align);

    let mut span_fields = Vec::new();
    let mut offset = 0usize;
    span_fields.push(make_field(
        "data",
        Ty::named("Std::Runtime::Collections::ValueMutPtr"),
        0,
        offset,
    ));
    offset += value_ptr_size;
    offset = align_to(offset, align);
    span_fields.push(make_field("len", Ty::named("usize"), 1, offset));
    offset += word;
    offset = align_to(offset, align);
    span_fields.push(make_field("elem_size", Ty::named("usize"), 2, offset));
    offset += word;
    offset = align_to(offset, align);
    span_fields.push(make_field("elem_align", Ty::named("usize"), 3, offset));
    offset += word;
    let span_size = align_to(offset, align);
    let span_layout = StructLayout {
        name: span_name.clone(),
        repr: TypeRepr::Default,
        packing: None,
        fields: span_fields,
        positional: Vec::new(),
        list: Some(ListLayout {
            element_index: Some(0),
            length_index: Some(1),
            span: None,
        }),
        size: Some(span_size),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    let mut readonly_fields = Vec::new();
    let mut readonly_offset = 0usize;
    readonly_fields.push(make_field(
        "data",
        Ty::named("Std::Runtime::Collections::ValueConstPtr"),
        0,
        readonly_offset,
    ));
    readonly_offset += value_ptr_size;
    readonly_offset = align_to(readonly_offset, align);
    readonly_fields.push(make_field("len", Ty::named("usize"), 1, readonly_offset));
    readonly_offset += word;
    readonly_offset = align_to(readonly_offset, align);
    readonly_fields.push(make_field(
        "elem_size",
        Ty::named("usize"),
        2,
        readonly_offset,
    ));
    readonly_offset += word;
    readonly_offset = align_to(readonly_offset, align);
    readonly_fields.push(make_field(
        "elem_align",
        Ty::named("usize"),
        3,
        readonly_offset,
    ));
    readonly_offset += word;
    let readonly_size = align_to(readonly_offset, align);
    let readonly_layout = StructLayout {
        name: readonly_span_name.clone(),
        repr: TypeRepr::Default,
        packing: None,
        fields: readonly_fields,
        positional: Vec::new(),
        list: Some(ListLayout {
            element_index: Some(0),
            length_index: Some(1),
            span: None,
        }),
        size: Some(readonly_size),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert(span_name, TypeLayout::Struct(span_layout.clone()));
    layouts
        .types
        .insert(readonly_span_name, TypeLayout::Struct(readonly_layout));

    layouts.finalize_auto_traits();
    layouts
}

fn span_stackalloc_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let len_local = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("len".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let span_ty = Ty::Span(SpanTy::new(Box::new(Ty::named("int"))));
    let source = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("source".into()),
        span_ty.clone(),
        true,
        None,
        LocalKind::Local,
    ));
    let dest = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("dest".into()),
        span_ty,
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(crate::mir::ConstOperand::new(
                crate::mir::ConstValue::Int(3),
            ))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(source),
            value: Rvalue::SpanStackAlloc {
                element: Ty::named("int"),
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(dest),
            value: Rvalue::SpanStackAlloc {
                element: Ty::named("int"),
                length: Operand::Copy(Place::new(len_local)),
                source: Some(Operand::Copy(Place::new(source))),
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let mut module = MirModule::default();
    module.type_layouts = span_layouts(&Ty::named("int"));
    let function = MirFunction {
        name: "Demo::SpanCopy".into(),
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
    module.functions.push(function);
    module
}

fn emit_function_ir(module: &MirModule, index: usize) -> String {
    let target = test_target();
    let signatures = build_signatures(module, None, &target).expect("signatures");
    let function = &module.functions[index];
    let sig = signatures
        .get(&function.name)
        .unwrap_or_else(|| panic!("missing signature for {}", function.name));
    let mut externals = BTreeSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut out = String::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut externals,
        &HashSet::new(),
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit function");
    function_ir(&out, &sig.symbol).to_string()
}

#[test]
fn span_stackalloc_from_span_invokes_runtime_copy() {
    let module = span_stackalloc_module();
    let body = emit_function_ir(&module, 0);
    assert!(
        body.contains("@chic_rt_span_copy_to"),
        "expected span stackalloc with source to call runtime copy: {body}"
    );
    assert!(
        body.contains("call i32 @chic_rt_span_copy_to"),
        "expected runtime copy call to return status: {body}"
    );
}
