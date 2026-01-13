//! Minimal in-house WebAssembly backend for Impact.
//!
//! This module translates Chic MIR into a subset of the WebAssembly
//! binary format without relying on external encoder/runtime crates. The
//! generated modules target `wasm32` and are consumed by Impact's own
//! lightweight executor.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::chic_kind::ChicKind;
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::error::Error;
use crate::frontend::ast::{ClassMember, FunctionDecl, Item, Module as AstModule};
use crate::frontend::metadata::collect_and_serialize_reflection;
use crate::hash_glue::SynthesisedHashGlue;
#[cfg(test)]
use crate::mir::{BlockId, Operand};
use crate::mir::{GenericArg, MirFunction, MirModule, Ty, TypeLayout};
use crate::perf::PerfMetadata;
use crate::target::Target;
use crate::type_metadata::SynthesisedTypeMetadata;
#[cfg(test)]
use emitter::function::FunctionEmitter;

use super::{Backend, CodegenArtifact, CodegenOptions, generate_text};

pub(super) const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
pub(super) const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
// Reserve a larger space for data segments before the stack begins; stdlib-heavy
// builds easily exceed the old 4 KiB threshold.
pub(super) const STACK_BASE: u32 = 0x10000;
pub(super) const STACK_POINTER_GLOBAL_INDEX: u32 = 0;
// Stdlib-heavy builds (and their test suites) routinely require more than the
// default 2 pages (128 KiB) once the heap starts allocating; keep a safer floor.
//
// NOTE: some runtime/native stdlib paths use a bump allocator without `memory.grow`,
// so the module's initial memory must be large enough for realistic test suites.
pub(super) const LINEAR_MEMORY_MIN_PAGES: u32 = 64;

pub(crate) fn wasm_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("CHIC_WASM_TRACE").is_some())
}

pub(crate) fn trace_log(args: std::fmt::Arguments<'_>) {
    if wasm_trace_enabled() {
        eprintln!("[wasm::trace] {args}");
    }
}

macro_rules! wasm_debug {
    ($($arg:tt)*) => {
        $crate::codegen::wasm::trace_log(format_args!($($arg)*));
    };
}

pub mod accelerator;
mod emitter;
mod encoding;
pub mod expr;
pub mod graphs;
mod layout;
mod limits;
pub mod memory;
mod metadata;
mod module_builder;
pub mod numerics;
mod runtime_hooks;
mod types;
pub(crate) use encoding::{
    push_atomic_fence, push_atomic_memory, push_block_like, push_br, push_byte, push_call,
    push_call_indirect, push_f32_const, push_f64_const, push_global, push_i32_const,
    push_i32_const_expr, push_i64_const, push_local, push_memory, push_string, write_u32,
};
pub(crate) use layout::{
    AggregateAllocation, align_to, compute_aggregate_allocation, local_requires_memory,
    lookup_layout,
};
pub(crate) use limits::ensure_u32;
use module_builder::ModuleBuilder;
pub(crate) use runtime_hooks::RuntimeHook;
pub(crate) use types::{ValueType, map_type};

/// Emit a Chic module as WebAssembly bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "Codegen entrypoint mirrors other backends (AST, MIR, target, kind, output, options)."
)]
pub fn compile(
    ast: &AstModule,
    mir: &MirModule,
    perf_metadata: &PerfMetadata,
    target: &Target,
    kind: ChicKind,
    output: &Path,
    options: &CodegenOptions,
    drop_glue: &[SynthesisedDropGlue],
    _hash_glue: &[SynthesisedHashGlue],
    _eq_glue: &[SynthesisedEqGlue],
    type_metadata: &[SynthesisedTypeMetadata],
) -> Result<CodegenArtifact, Error> {
    if options.backend != Backend::Wasm {
        return Err(Error::Codegen(
            "WASM backend invoked with non-WASM options state".into(),
        ));
    }

    let skip_stdlib = std::env::var("CHIC_SKIP_STDLIB")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let textual_ir = generate_text(ast, target, kind);
    let mut entry = find_entry_function(ast);
    if mir.attributes.suppress_startup_descriptor {
        entry = None;
    }
    if kind == ChicKind::Executable
        && entry.is_none()
        && !mir.attributes.is_no_std()
        && !mir.attributes.is_no_main()
        && !mir.attributes.suppress_startup_descriptor
        && !skip_stdlib
    {
        return Err(Error::Codegen(
            "executable builds require a `Main` function in the root namespace".into(),
        ));
    }

    debug_assert!(
        drop_glue
            .iter()
            .all(|entry| mir.functions.iter().any(|f| f.name == entry.symbol)),
        "drop glue function missing from WASM MIR module"
    );

    let reflection_json = collect_and_serialize_reflection(ast)
        .map_err(|err| Error::Codegen(format!("failed to serialise reflection metadata: {err}")))?;
    let reflection_bytes = reflection_json.into_bytes();

    if module_contains_vectors(mir) {
        return Err(Error::Codegen(
            "[TYPE0704] WASM backend does not yet support SIMD vectors; enable wasm_simd128 or use the LLVM backend until scalar fallback is implemented"
                .into(),
        ));
    }

    let builder = ModuleBuilder::new(
        mir,
        entry,
        kind,
        type_metadata,
        _hash_glue,
        _eq_glue,
        Some(reflection_bytes),
        perf_metadata,
        options.coverage,
    )?;
    wasm_debug!(
        "compile: emitting module with {} functions (entry = {:?}, kind = {:?})",
        mir.functions.len(),
        builder.entry.as_deref().unwrap_or("<none>"),
        builder.kind
    );
    let bytes = builder.emit()?;

    if let Some(parent) = output.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, &bytes)?;

    if options.emit_wat_text {
        let wat = builder.emit_wat(&bytes)?;
        let wat_path = output.with_extension("wat");
        if let Some(parent) = wat_path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(&wat_path, wat)?;
    }

    Ok(CodegenArtifact {
        textual_ir,
        object_path: output.to_path_buf(),
        artifact_path: output.to_path_buf(),
        library_pack: None,
        constant_folds: 0,
        inlined_functions: Vec::new(),
        metadata_path: None,
        metadata_telemetry: None,
        reflection_metadata_path: None,
    })
}

pub(crate) fn module_contains_vectors(mir: &MirModule) -> bool {
    mir.statics
        .iter()
        .any(|static_var| ty_contains_vector(&static_var.ty))
        || mir.functions.iter().any(function_uses_vector)
        || mir.type_layouts.types.values().any(layout_contains_vector)
}

fn function_uses_vector(function: &MirFunction) -> bool {
    if ty_contains_vector(&function.signature.ret)
        || function.signature.params.iter().any(ty_contains_vector)
        || function.signature.effects.iter().any(ty_contains_vector)
        || function
            .async_result
            .as_ref()
            .is_some_and(ty_contains_vector)
    {
        return true;
    }

    body_contains_vector(&function.body)
}

fn body_contains_vector(body: &crate::mir::MirBody) -> bool {
    if body.effects.iter().any(ty_contains_vector)
        || body
            .stream_metadata
            .iter()
            .any(|stream| stream.mem_space.as_ref().is_some_and(ty_contains_vector))
    {
        return true;
    }

    for local in &body.locals {
        if ty_contains_vector(&local.ty) {
            return true;
        }
    }

    for block in &body.blocks {
        for statement in &block.statements {
            if statement_contains_vector(statement) {
                return true;
            }
        }
        if let Some(terminator) = &block.terminator {
            if terminator_contains_vector(terminator) {
                return true;
            }
        }
    }

    false
}

fn statement_contains_vector(statement: &crate::mir::Statement) -> bool {
    match &statement.kind {
        crate::mir::StatementKind::Assign { value, .. } => rvalue_contains_vector(value),
        crate::mir::StatementKind::InlineAsm(asm) => {
            asm.operands.iter().any(inline_asm_operand_contains_vector)
        }
        _ => false,
    }
}

fn inline_asm_operand_contains_vector(operand: &crate::mir::InlineAsmOperand) -> bool {
    match &operand.kind {
        crate::mir::InlineAsmOperandKind::In { value }
        | crate::mir::InlineAsmOperandKind::Const { value } => operand_contains_vector(value),
        crate::mir::InlineAsmOperandKind::InOut { input, .. } => operand_contains_vector(input),
        crate::mir::InlineAsmOperandKind::Out { .. }
        | crate::mir::InlineAsmOperandKind::Sym { .. } => false,
    }
}

fn rvalue_contains_vector(value: &crate::mir::Rvalue) -> bool {
    match value {
        crate::mir::Rvalue::Use(operand) | crate::mir::Rvalue::Unary { operand, .. } => {
            operand_contains_vector(operand)
        }
        crate::mir::Rvalue::Binary { lhs, rhs, .. } => {
            operand_contains_vector(lhs) || operand_contains_vector(rhs)
        }
        crate::mir::Rvalue::Aggregate { fields, .. } => fields.iter().any(operand_contains_vector),
        crate::mir::Rvalue::SpanStackAlloc {
            element,
            length,
            source,
        } => {
            ty_contains_vector(element)
                || operand_contains_vector(length)
                || source.as_ref().is_some_and(operand_contains_vector)
        }
        crate::mir::Rvalue::Cast {
            operand,
            source,
            target,
            ..
        } => {
            ty_contains_vector(source)
                || ty_contains_vector(target)
                || operand_contains_vector(operand)
        }
        crate::mir::Rvalue::StringInterpolate { segments } => {
            segments.iter().any(|segment| match segment {
                crate::mir::InterpolatedStringSegment::Text { .. } => false,
                crate::mir::InterpolatedStringSegment::Expr { operand, .. } => {
                    operand_contains_vector(operand)
                }
            })
        }
        crate::mir::Rvalue::NumericIntrinsic(intrinsic) => {
            intrinsic.operands.iter().any(operand_contains_vector)
        }
        crate::mir::Rvalue::DecimalIntrinsic(intrinsic) => {
            operand_contains_vector(&intrinsic.lhs)
                || operand_contains_vector(&intrinsic.rhs)
                || intrinsic
                    .addend
                    .as_ref()
                    .is_some_and(operand_contains_vector)
                || operand_contains_vector(&intrinsic.rounding)
                || operand_contains_vector(&intrinsic.vectorize)
        }
        crate::mir::Rvalue::AtomicLoad { .. }
        | crate::mir::Rvalue::AtomicRmw { .. }
        | crate::mir::Rvalue::AtomicCompareExchange { .. }
        | crate::mir::Rvalue::AddressOf { .. }
        | crate::mir::Rvalue::Len(_)
        | crate::mir::Rvalue::Pending(_)
        | crate::mir::Rvalue::StaticLoad { .. }
        | crate::mir::Rvalue::StaticRef { .. } => false,
    }
}

fn terminator_contains_vector(terminator: &crate::mir::Terminator) -> bool {
    match terminator {
        crate::mir::Terminator::SwitchInt { discr, .. } => operand_contains_vector(discr),
        crate::mir::Terminator::Call { func, args, .. } => {
            operand_contains_vector(func) || args.iter().any(operand_contains_vector)
        }
        crate::mir::Terminator::Yield { value, .. } => operand_contains_vector(value),
        crate::mir::Terminator::Throw { exception, ty } => {
            exception.as_ref().is_some_and(operand_contains_vector)
                || ty.as_ref().is_some_and(ty_contains_vector)
        }
        crate::mir::Terminator::Pending(_)
        | crate::mir::Terminator::Goto { .. }
        | crate::mir::Terminator::Match { .. }
        | crate::mir::Terminator::Return
        | crate::mir::Terminator::Await { .. }
        | crate::mir::Terminator::Panic
        | crate::mir::Terminator::Unreachable => false,
    }
}

fn operand_contains_vector(operand: &crate::mir::Operand) -> bool {
    match operand {
        crate::mir::Operand::Mmio(mmio) => ty_contains_vector(&mmio.ty),
        crate::mir::Operand::Pending(pending) => pending_operand_contains_vector(pending),
        crate::mir::Operand::Copy(_)
        | crate::mir::Operand::Move(_)
        | crate::mir::Operand::Borrow(_)
        | crate::mir::Operand::Const(_) => false,
    }
}

fn pending_operand_contains_vector(operand: &crate::mir::PendingOperand) -> bool {
    operand
        .info
        .as_deref()
        .is_some_and(pending_operand_info_contains_vector)
}

fn pending_operand_info_contains_vector(info: &crate::mir::PendingOperandInfo) -> bool {
    match info {
        crate::mir::PendingOperandInfo::FunctionGroup {
            candidates,
            receiver,
            ..
        } => {
            receiver.as_deref().is_some_and(operand_contains_vector)
                || candidates
                    .iter()
                    .any(|candidate| fnty_contains_vector(&candidate.signature))
        }
    }
}

fn fnty_contains_vector(fn_ty: &crate::mir::FnTy) -> bool {
    ty_contains_vector(&fn_ty.ret) || fn_ty.params.iter().any(ty_contains_vector)
}

fn layout_contains_vector(layout: &TypeLayout) -> bool {
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data
            .fields
            .iter()
            .any(|field| ty_contains_vector(&field.ty)),
        TypeLayout::Enum(data) => {
            ty_contains_vector(&data.underlying)
                || data.variants.iter().any(|variant| {
                    variant
                        .fields
                        .iter()
                        .any(|field| ty_contains_vector(&field.ty))
                })
        }
        TypeLayout::Union(data) => data.views.iter().any(|view| ty_contains_vector(&view.ty)),
    }
}

fn ty_contains_vector(ty: &Ty) -> bool {
    match ty {
        Ty::Vector(_) => true,
        Ty::Array(array) => ty_contains_vector(&array.element),
        Ty::Vec(vec) => ty_contains_vector(&vec.element),
        Ty::Span(span) => ty_contains_vector(&span.element),
        Ty::ReadOnlySpan(span) => ty_contains_vector(&span.element),
        Ty::Rc(rc) => ty_contains_vector(&rc.element),
        Ty::Arc(arc) => ty_contains_vector(&arc.element),
        Ty::Pointer(pointer) => ty_contains_vector(&pointer.element),
        Ty::Ref(reference) => ty_contains_vector(&reference.element),
        Ty::Tuple(tuple) => tuple.elements.iter().any(ty_contains_vector),
        Ty::Fn(fun) => fnty_contains_vector(fun),
        Ty::Nullable(inner) => ty_contains_vector(inner),
        Ty::Named(named) => named.args.iter().any(|arg| match arg {
            GenericArg::Type(inner) => ty_contains_vector(inner),
            GenericArg::Const(_) => false,
        }),
        Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => false,
    }
}

pub(crate) fn find_entry_function(module: &AstModule) -> Option<String> {
    find_entry_in_items(&module.items, module.namespace.as_deref())
}

fn find_entry_in_items(items: &[Item], namespace: Option<&str>) -> Option<String> {
    for item in items {
        match item {
            Item::Function(func) if func.name == "Main" => {
                return Some(qualify(namespace, &func.name));
            }
            Item::Struct(def) => {
                let struct_ns = qualify(namespace, &def.name);
                let methods: Vec<_> = def.methods.iter().collect();
                if let Some(found) = find_entry_in_methods(&methods, Some(&struct_ns)) {
                    return Some(found);
                }
                if let Some(found) = find_entry_in_items(&def.nested_types, Some(&struct_ns)) {
                    return Some(found);
                }
            }
            Item::Class(def) => {
                let class_ns = qualify(namespace, &def.name);
                let methods: Vec<_> = def
                    .members
                    .iter()
                    .filter_map(|member| {
                        if let ClassMember::Method(method) = member {
                            Some(method)
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Some(found) = find_entry_in_methods(&methods, Some(&class_ns)) {
                    return Some(found);
                }
                if let Some(found) = find_entry_in_items(&def.nested_types, Some(&class_ns)) {
                    return Some(found);
                }
            }
            Item::Namespace(ns) => {
                let nested = qualify(namespace, &ns.name);
                if let Some(found) = find_entry_in_items(&ns.items, Some(&nested)) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_entry_in_methods(methods: &[&FunctionDecl], namespace: Option<&str>) -> Option<String> {
    for method in methods {
        if method.name == "Main" {
            return Some(qualify(namespace, &method.name));
        }
    }
    None
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

pub(crate) fn ensure_wat_artifact(
    mir: &MirModule,
    entry: Option<String>,
    kind: ChicKind,
    wasm_path: &Path,
    type_metadata: &[SynthesisedTypeMetadata],
    hash_glue: &[SynthesisedHashGlue],
    eq_glue: &[SynthesisedEqGlue],
    perf_metadata: &PerfMetadata,
) -> Result<PathBuf, Error> {
    let bytes = fs::read(wasm_path)?;
    let builder = ModuleBuilder::new(
        mir,
        entry,
        kind,
        type_metadata,
        hash_glue,
        eq_glue,
        None,
        perf_metadata,
        false,
    )?;
    let wat = builder.emit_wat(&bytes)?;
    let wat_path = wasm_path.with_extension("wat");
    if let Some(parent) = wat_path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(&wat_path, wat)?;
    Ok(wat_path)
}

// ModuleBuilder lives in module_builder.rs

// helper types (Section, FunctionSignature) are defined in module_builder.rs

#[cfg(test)]
pub(crate) fn test_emit_module(
    functions: Vec<MirFunction>,
    entry: Option<String>,
    kind: ChicKind,
) -> Vec<u8> {
    crate::mir::configure_pointer_width(4, 4);
    let type_layouts = crate::mir::TypeLayoutTable::default();
    let primitive_registry = type_layouts.primitive_registry.clone();
    let module = MirModule {
        functions,
        test_cases: Vec::new(),
        statics: Vec::new(),
        type_layouts,
        primitive_registry,
        interned_strs: Vec::new(),
        exports: Vec::new(),
        attributes: crate::mir::module_metadata::ModuleAttributes::default(),
        trait_vtables: Vec::new(),
        class_vtables: Vec::new(),
        interface_defaults: Vec::new(),
        default_arguments: Vec::new(),
        type_variance: std::collections::HashMap::new(),
        async_plans: Vec::new(),
    };
    let perf_metadata = PerfMetadata::default();
    let builder = ModuleBuilder::new(
        &module,
        entry,
        kind,
        &[],
        &[],
        &[],
        None,
        &perf_metadata,
        false,
    )
    .unwrap_or_else(|err| panic!("construct module: {err}"));
    builder
        .emit()
        .unwrap_or_else(|err| panic!("emit wasm module: {err}"))
}

#[cfg(test)]
#[expect(
    clippy::too_many_lines,
    reason = "Sample fixture intentionally inlines a full MIR body for testing."
)]
pub(crate) fn sample_loop_function() -> MirFunction {
    use crate::mir::{
        BasicBlock, BinOp, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
        LocalKind, MirBody, Operand, Place, Rvalue, Statement, StatementKind, Terminator, Ty,
    };

    fn place(local: usize) -> Place {
        Place::new(LocalId(local))
    }

    fn assign_const(local: usize, value: i128) -> Statement {
        Statement {
            span: None,
            kind: StatementKind::Assign {
                place: place(local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(value)))),
            },
        }
    }

    fn add_assign(local: usize, lhs: usize, rhs: Operand) -> Statement {
        Statement {
            span: None,
            kind: StatementKind::Assign {
                place: place(local),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(place(lhs)),
                    rhs,
                    rounding: None,
                },
            },
        }
    }

    fn binary_assign(local: usize, op: BinOp, left: usize, right: Operand) -> Statement {
        Statement {
            span: None,
            kind: StatementKind::Assign {
                place: place(local),
                value: Rvalue::Binary {
                    op,
                    lhs: Operand::Copy(place(left)),
                    rhs: right,
                    rounding: None,
                },
            },
        }
    }

    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("total".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("index".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("cond".into()),
        Ty::named("bool"),
        true,
        None,
        LocalKind::Temp,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.statements.push(assign_const(1, 0));
    block0.statements.push(assign_const(2, 0));
    block0.terminator = Some(Terminator::Goto { target: BlockId(1) });

    let mut block1 = BasicBlock::new(BlockId(1), None);
    block1.statements.push(binary_assign(
        4,
        BinOp::Lt,
        2,
        Operand::Const(ConstOperand::new(ConstValue::Int(6))),
    ));
    block1.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(place(4)),
        targets: vec![(1, BlockId(2))],
        otherwise: BlockId(3),
    });

    let mut block2 = BasicBlock::new(BlockId(2), None);
    block2.statements.push(binary_assign(
        3,
        BinOp::Rem,
        2,
        Operand::Const(ConstOperand::new(ConstValue::Int(3))),
    ));
    block2.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(place(3)),
        targets: vec![(0, BlockId(4)), (1, BlockId(5))],
        otherwise: BlockId(6),
    });

    let mut block4 = BasicBlock::new(BlockId(4), None);
    block4.statements.push(add_assign(
        1,
        1,
        Operand::Const(ConstOperand::new(ConstValue::Int(3))),
    ));
    block4.terminator = Some(Terminator::Goto { target: BlockId(7) });

    let mut block5 = BasicBlock::new(BlockId(5), None);
    block5.statements.push(add_assign(
        1,
        1,
        Operand::Const(ConstOperand::new(ConstValue::Int(1))),
    ));
    block5.terminator = Some(Terminator::Goto { target: BlockId(7) });

    let mut block6 = BasicBlock::new(BlockId(6), None);
    block6.statements.push(add_assign(
        1,
        1,
        Operand::Const(ConstOperand::new(ConstValue::Int(2))),
    ));
    block6.terminator = Some(Terminator::Goto { target: BlockId(7) });

    let mut block7 = BasicBlock::new(BlockId(7), None);
    block7.statements.push(add_assign(
        2,
        2,
        Operand::Const(ConstOperand::new(ConstValue::Int(1))),
    ));
    block7.terminator = Some(Terminator::Goto { target: BlockId(1) });

    let mut block3 = BasicBlock::new(BlockId(3), None);
    block3.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: place(0),
            value: Rvalue::Use(Operand::Copy(place(1))),
        },
    });
    block3.terminator = Some(Terminator::Return);

    let mut block_set = vec![
        block0, block1, block2, block3, block4, block5, block6, block7,
    ];
    for (idx, block) in block_set.iter_mut().enumerate() {
        block.id = BlockId(idx);
    }
    body.blocks = block_set;

    MirFunction {
        name: "Exec::Main".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
            abi: crate::mir::Abi::Chic,
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
    }
}

#[cfg(test)]
#[expect(
    clippy::too_many_lines,
    reason = "Match fixture purposely enumerates a full MIR body for integration tests."
)]
pub(crate) fn sample_match_function() -> MirFunction {
    use crate::mir::{
        BasicBlock, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind,
        MatchArm, MirBody, Pattern, Place, Rvalue, Statement, StatementKind, Terminator, Ty,
    };

    fn assign_const(local: usize, value: i128) -> Statement {
        Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(local)),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(value)))),
            },
        }
    }

    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("input".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.statements.push(assign_const(1, 3));
    block0.terminator = Some(Terminator::Goto { target: BlockId(1) });

    let mut block1 = BasicBlock::new(BlockId(1), None);
    block1.terminator = Some(Terminator::Match {
        value: Place::new(LocalId(1)),
        arms: vec![
            MatchArm {
                pattern: Pattern::Literal(ConstValue::Int(0)),
                guard: None,
                bindings: Vec::new(),
                target: BlockId(2),
            },
            MatchArm {
                pattern: Pattern::Literal(ConstValue::Int(3)),
                guard: None,
                bindings: Vec::new(),
                target: BlockId(3),
            },
        ],
        otherwise: BlockId(4),
    });

    let mut block2 = BasicBlock::new(BlockId(2), None);
    block2.statements.push(assign_const(0, 10));
    block2.terminator = Some(Terminator::Return);

    let mut block3 = BasicBlock::new(BlockId(3), None);
    block3.statements.push(assign_const(0, 20));
    block3.terminator = Some(Terminator::Return);

    let mut block4 = BasicBlock::new(BlockId(4), None);
    block4.statements.push(assign_const(0, 30));
    block4.terminator = Some(Terminator::Return);

    body.blocks = vec![block0, block1, block2, block3, block4];

    MirFunction {
        name: "Exec::Main".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
            abi: crate::mir::Abi::Chic,
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
    }
}

#[cfg(test)]
mod tests;
