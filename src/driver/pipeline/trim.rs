use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::drop_glue::drop_glue_symbol_for;
use crate::manifest::Manifest;
use crate::mir::{Abi, FunctionKind, MirModule};
use crate::package::resolver::ResolvedPackage;

use super::FrontendModuleState;

pub(super) fn collect_used_packages(modules: &[FrontendModuleState]) -> HashSet<String> {
    let mut used = HashSet::new();
    for module in modules {
        if module.is_stdlib {
            continue;
        }
        for import in &module.parse.module.package_imports {
            used.insert(import.name.clone());
        }
    }
    used
}

pub(super) fn compute_reachable_packages(
    used: &HashSet<String>,
    resolved: &HashMap<String, ResolvedPackage>,
) -> HashSet<String> {
    let mut reachable = if used.is_empty() {
        resolved.keys().cloned().collect()
    } else {
        used.clone()
    };
    let mut queue: VecDeque<String> = reachable.iter().cloned().collect();
    while let Some(name) = queue.pop_front() {
        if let Some(pkg) = resolved.get(&name) {
            for dep in pkg.manifest.dependencies() {
                if reachable.insert(dep.name.clone()) {
                    queue.push_back(dep.name.clone());
                }
            }
        }
    }
    reachable
}

#[derive(Default)]
pub(super) struct TrimStats {
    pub(super) trimmed_functions: usize,
    pub(super) trimmed_exports: usize,
}

pub(super) fn trim_unreachable_package_exports(
    for_tests: bool,
    root_manifest: Option<&Manifest>,
    modules: &[FrontendModuleState],
    mir_module: &mut MirModule,
    unit_functions: &mut [Vec<usize>],
) -> TrimStats {
    let root_manifest_path = root_manifest
        .and_then(|manifest| manifest.path())
        .map(PathBuf::from);
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        let bucket_sizes: Vec<_> = unit_functions.iter().map(|bucket| bucket.len()).collect();
        eprintln!(
            "[chic-debug] trim analysis: {} functions across {} units (bucket sizes {:?})",
            mir_module.functions.len(),
            unit_functions.len(),
            bucket_sizes
        );
        for (idx, module) in modules.iter().enumerate() {
            let bucket_len = unit_functions
                .get(idx)
                .map(|bucket| bucket.len())
                .unwrap_or(0);
            eprintln!(
                "[chic-debug]  unit {idx}: {} (functions {})",
                module.input.display(),
                bucket_len
            );
        }
    }
    let mut function_to_module = HashMap::new();
    for (module_idx, bucket) in unit_functions.iter().enumerate() {
        for &func_idx in bucket {
            function_to_module.insert(func_idx, module_idx);
        }
    }

    let mut always_keep = HashSet::new();
    let mut root_keep = HashSet::new();
    if !for_tests {
        for (module_idx, bucket) in unit_functions.iter().enumerate() {
            if let Some(module) = modules.get(module_idx) {
                if module.is_stdlib {
                    always_keep.extend(bucket.iter().copied());
                }
                if is_root_module(module, root_manifest_path.as_deref()) && !module.is_stdlib {
                    always_keep.extend(bucket.iter().copied());
                    root_keep.extend(bucket.iter().copied());
                }
            }
        }
    }

    let name_to_indices = index_function_names(&mir_module.functions);
    let base_name_to_indices = index_function_bases(&mir_module.functions);
    let default_arg_edges = build_default_arg_edges(&mir_module.default_arguments);
    let graph = build_symbol_graph(mir_module);
    let function_names: Vec<_> = mir_module
        .functions
        .iter()
        .map(|func| func.name.clone())
        .collect();
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        for (idx, node) in graph.iter().enumerate() {
            eprintln!(
                "[chic-debug] symbols for {}: {:?}",
                function_names.get(idx).cloned().unwrap_or_default(),
                node.symbols
            );
        }
        for function in &mir_module.functions {
            for block in &function.body.blocks {
                if let Some(crate::mir::Terminator::Call { func, .. }) = &block.terminator {
                    eprintln!("[chic-debug] call target in {}: {:?}", function.name, func);
                }
            }
        }
    }

    let mut seeds = Vec::new();
    if for_tests {
        for (idx, function) in mir_module.functions.iter().enumerate() {
            let is_root_testcase = if function.kind == FunctionKind::Testcase {
                function_to_module
                    .get(&idx)
                    .and_then(|module_idx| modules.get(*module_idx))
                    .is_some_and(|module| is_root_module(module, root_manifest_path.as_deref()))
            } else {
                false
            };
            if is_root_testcase
                || function.extern_spec.is_some()
                || matches!(function.signature.abi, Abi::Extern(_))
                || function.name.starts_with("__cl_")
            {
                seeds.push(idx);
            }
        }
        for vtable in &mir_module.trait_vtables {
            if let Some(indices) = name_to_indices.get(&vtable.symbol) {
                seeds.extend(indices.iter().copied());
            }
            for slot in &vtable.slots {
                if let Some(indices) = name_to_indices.get(&slot.symbol) {
                    seeds.extend(indices.iter().copied());
                }
            }
        }
        for vtable in &mir_module.class_vtables {
            if let Some(indices) = name_to_indices.get(&vtable.symbol) {
                seeds.extend(indices.iter().copied());
            }
            for slot in &vtable.slots {
                if let Some(indices) = name_to_indices.get(&slot.symbol) {
                    seeds.extend(indices.iter().copied());
                }
            }
        }
        for default_impl in &mir_module.interface_defaults {
            if let Some(indices) = name_to_indices.get(&default_impl.symbol) {
                seeds.extend(indices.iter().copied());
            }
        }
        for arg in &mir_module.default_arguments {
            if let crate::mir::DefaultArgumentKind::Thunk { symbol, .. } = &arg.value {
                if let Some(indices) = name_to_indices.get(symbol) {
                    seeds.extend(indices.iter().copied());
                }
            }
        }
        for plan in &mir_module.async_plans {
            if let Some(indices) = name_to_indices.get(&plan.poll_fn) {
                seeds.extend(indices.iter().copied());
            }
            if let Some(indices) = name_to_indices.get(&plan.drop_fn) {
                seeds.extend(indices.iter().copied());
            }
        }
    } else {
        seeds.extend(root_keep.iter().copied());
        for (idx, function) in mir_module.functions.iter().enumerate() {
            let Some(module_idx) = function_to_module.get(&idx) else {
                continue;
            };
            let Some(module) = modules.get(*module_idx) else {
                continue;
            };
            if !is_root_module(module, root_manifest_path.as_deref()) || module.is_stdlib {
                continue;
            }
            if function.kind == FunctionKind::Testcase
                || function.extern_spec.is_some()
                || matches!(function.signature.abi, Abi::Extern(_))
            {
                seeds.push(idx);
            }
        }
    }

    for testcase in &mir_module.test_cases {
        seeds.push(testcase.function_index);
    }

    if !for_tests {
        for export in &mir_module.exports {
            if let Some(indices) = name_to_indices.get(&export.function) {
                for idx in indices {
                    if let Some(module_idx) = function_to_module.get(idx) {
                        if let Some(module) = modules.get(*module_idx) {
                            if is_root_module(module, root_manifest_path.as_deref())
                                && !module.is_stdlib
                            {
                                seeds.push(*idx);
                            }
                        }
                    }
                }
            }
        }
    }

    let has_roots = if for_tests {
        true
    } else {
        modules.iter().any(|module| {
            is_root_module(module, root_manifest_path.as_deref()) && !module.is_stdlib
        })
    };

    let mut reachable = reachable_from_roots(
        &graph,
        &name_to_indices,
        &base_name_to_indices,
        &default_arg_edges,
        &function_names,
        seeds,
    );
    reachable.extend(always_keep.iter().copied());

    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        eprintln!(
            "[chic-debug] pre-trim reachable functions = {} (exports before = {})",
            reachable.len(),
            mir_module.exports.len()
        );
    }

    let before_exports = mir_module.exports.len();
    mir_module.exports.retain(|export| {
        name_to_indices
            .get(&export.function)
            .map(|indices| indices.iter().any(|idx| reachable.contains(idx)))
            .unwrap_or(true)
    });
    let trimmed_exports = before_exports.saturating_sub(mir_module.exports.len());

    let trimmed_functions = if !for_tests {
        0
    } else if !has_roots {
        if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
            eprintln!("[chic-debug] skipping function trim: no root modules found");
        }
        0
    } else if reachable.is_empty() {
        if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
            eprintln!("[chic-debug] skipping function trim: no reachable functions");
        }
        0
    } else {
        let before = mir_module.functions.len();
        let mut old_to_new: Vec<Option<usize>> = vec![None; before];
        let mut retained = Vec::with_capacity(reachable.len());
        for (idx, function) in mir_module.functions.iter().enumerate() {
            if !reachable.contains(&idx) {
                continue;
            }
            let new_idx = retained.len();
            old_to_new[idx] = Some(new_idx);
            retained.push(function.clone());
        }
        mir_module.functions = retained;

        for bucket in unit_functions.iter_mut() {
            let mut remapped = Vec::with_capacity(bucket.len());
            for &idx in bucket.iter() {
                if let Some(new_idx) = old_to_new.get(idx).and_then(|v| *v) {
                    remapped.push(new_idx);
                }
            }
            remapped.sort_unstable();
            remapped.dedup();
            *bucket = remapped;
        }

        mir_module.test_cases.retain_mut(|meta| {
            if let Some(new_idx) = old_to_new.get(meta.function_index).and_then(|v| *v) {
                meta.function_index = new_idx;
                true
            } else {
                false
            }
        });

        mir_module.async_plans.retain_mut(|plan| {
            if let Some(new_idx) = old_to_new.get(plan.function_index).and_then(|v| *v) {
                plan.function_index = new_idx;
                true
            } else {
                false
            }
        });

        before.saturating_sub(mir_module.functions.len())
    };

    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        eprintln!(
            "[chic-debug] post-trim exports = {} (trimmed_exports={trimmed_exports})",
            mir_module.exports.len()
        );
    }

    TrimStats {
        trimmed_functions,
        trimmed_exports,
    }
}

fn is_root_module(module: &FrontendModuleState, root_manifest_path: Option<&Path>) -> bool {
    if module.is_stdlib {
        return false;
    }
    match (
        root_manifest_path,
        module
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.path()),
    ) {
        (Some(root), Some(path)) => {
            let root_canonical = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
            let path_canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            path_canonical == root_canonical
        }
        (Some(_), None) => true,
        (None, _) => true,
    }
}

#[derive(Default)]
struct SymbolGraphNode {
    symbols: HashSet<String>,
}

fn build_symbol_graph(mir_module: &MirModule) -> Vec<SymbolGraphNode> {
    mir_module
        .functions
        .iter()
        .map(|function| analyse_body_symbols(function, &mir_module.type_layouts))
        .collect()
}

fn analyse_body_symbols(
    function: &crate::mir::MirFunction,
    layouts: &crate::mir::TypeLayoutTable,
) -> SymbolGraphNode {
    let mut node = SymbolGraphNode::default();
    let body = &function.body;
    for block in &body.blocks {
        for statement in &block.statements {
            scan_statement(statement, body, layouts, &mut node);
        }
        if let Some(terminator) = &block.terminator {
            scan_terminator(terminator, &mut node);
        }
    }
    node
}

fn scan_statement(
    statement: &crate::mir::Statement,
    body: &crate::mir::MirBody,
    layouts: &crate::mir::TypeLayoutTable,
    node: &mut SymbolGraphNode,
) {
    use crate::mir::InlineAsmOperandKind;
    use crate::mir::StatementKind;
    match &statement.kind {
        StatementKind::Assign { value, .. } => scan_rvalue(value, node),
        StatementKind::ZeroInitRaw { pointer, length } => {
            scan_operand(pointer, node);
            scan_operand(length, node);
        }
        StatementKind::AtomicStore { value, .. } => scan_operand(value, node),
        StatementKind::Assert { cond, .. } => scan_operand(cond, node),
        StatementKind::EnqueueKernel {
            kernel,
            args,
            completion,
            ..
        } => {
            scan_operand(kernel, node);
            for arg in args {
                scan_operand(arg, node);
            }
            if let Some(event) = completion {
                scan_place(event, node);
            }
        }
        StatementKind::EnqueueCopy {
            stream,
            dst,
            src,
            bytes,
            completion,
            ..
        } => {
            scan_place(stream, node);
            scan_place(dst, node);
            scan_place(src, node);
            scan_operand(bytes, node);
            if let Some(event) = completion {
                scan_place(event, node);
            }
        }
        StatementKind::RecordEvent { stream, event } => {
            scan_place(stream, node);
            scan_place(event, node);
        }
        StatementKind::WaitEvent { event, stream } => {
            scan_place(event, node);
            if let Some(stream) = stream {
                scan_place(stream, node);
            }
        }
        StatementKind::MmioStore { value, .. } => scan_operand(value, node),
        StatementKind::StaticStore { value, .. } => scan_operand(value, node),
        StatementKind::InlineAsm(asm) => {
            for operand in &asm.operands {
                match &operand.kind {
                    InlineAsmOperandKind::In { value }
                    | InlineAsmOperandKind::Const { value }
                    | InlineAsmOperandKind::InOut { input: value, .. } => scan_operand(value, node),
                    InlineAsmOperandKind::Out { .. } => {}
                    InlineAsmOperandKind::Sym { symbol } => {
                        node.symbols.insert(symbol.clone());
                    }
                }
            }
        }
        StatementKind::Drop { place, .. } => {
            scan_place(place, node);
            if let Some(ty) = place_value_ty(body, layouts, place) {
                node.symbols
                    .insert(drop_glue_symbol_for(&ty.canonical_name()));
            }
        }
        StatementKind::DeferDrop { place } => {
            scan_place(place, node);
            if let Some(ty) = place_value_ty(body, layouts, place) {
                node.symbols
                    .insert(drop_glue_symbol_for(&ty.canonical_name()));
            }
        }
        StatementKind::Deinit(place) => {
            scan_place(place, node);
            if let Some(ty) = place_value_ty(body, layouts, place)
                && let Some(symbol) = dispose_symbol_for_ty(layouts, &ty)
            {
                node.symbols.insert(symbol);
            }
        }
        StatementKind::Borrow { place, .. }
        | StatementKind::Retag { place }
        | StatementKind::DefaultInit { place }
        | StatementKind::ZeroInit { place } => scan_place(place, node),
        StatementKind::StorageLive(_)
        | StatementKind::StorageDead(_)
        | StatementKind::MarkFallibleHandled { .. }
        | StatementKind::AtomicFence { .. }
        | StatementKind::EnterUnsafe
        | StatementKind::ExitUnsafe
        | StatementKind::Eval(_)
        | StatementKind::Nop
        | StatementKind::Pending(_) => {}
    }
}

fn place_value_ty(
    body: &crate::mir::MirBody,
    layouts: &crate::mir::TypeLayoutTable,
    place: &crate::mir::Place,
) -> Option<crate::mir::Ty> {
    use crate::mir::{ProjectionElem, Ty, TypeLayout};

    let mut ty = body.locals.get(place.local.0)?.ty.clone();
    let mut enum_variant: Option<crate::mir::EnumVariantLayout> = None;

    for projection in &place.projection {
        match projection {
            ProjectionElem::Deref => match ty {
                Ty::Pointer(ptr) => ty = ptr.element.clone(),
                Ty::Ref(reference) => ty = reference.element.clone(),
                Ty::Nullable(inner) => ty = (*inner).clone(),
                _ => return None,
            },
            ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } => match ty {
                Ty::Array(array) => ty = (*array.element).clone(),
                Ty::Vec(vec) => ty = (*vec.element).clone(),
                Ty::Span(span) => ty = (*span.element).clone(),
                Ty::ReadOnlySpan(span) => ty = (*span.element).clone(),
                Ty::Pointer(ptr) => ty = ptr.element.clone(),
                Ty::Ref(reference) => ty = reference.element.clone(),
                _ => return None,
            },
            ProjectionElem::Subslice { .. } => {
                // Conservatively keep the existing type; subslices are not expected to drive
                // dispose/drop glue reachability.
            }
            ProjectionElem::Downcast { variant } => {
                let Ty::Named(_) = &ty else {
                    return None;
                };
                let layout = layouts.layout_for_name(&ty.canonical_name())?;
                let TypeLayout::Enum(enum_layout) = layout else {
                    return None;
                };
                let selected = enum_layout
                    .variants
                    .iter()
                    .find(|candidate| candidate.index == *variant)?
                    .clone();
                enum_variant = Some(selected);
            }
            ProjectionElem::Field(index) => {
                if let Some(variant) = enum_variant.take() {
                    ty = variant
                        .fields
                        .iter()
                        .find(|field| field.index == *index)?
                        .ty
                        .clone();
                    continue;
                }
                let layout = layouts.layout_for_name(&ty.canonical_name())?;
                match layout {
                    TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) => {
                        ty = struct_layout
                            .fields
                            .iter()
                            .find(|field| field.index == *index)?
                            .ty
                            .clone();
                    }
                    _ => return None,
                }
            }
            ProjectionElem::FieldNamed(name) => {
                if let Some(variant) = enum_variant.take() {
                    ty = variant
                        .fields
                        .iter()
                        .find(|field| field.name == *name)?
                        .ty
                        .clone();
                    continue;
                }
                let layout = layouts.layout_for_name(&ty.canonical_name())?;
                match layout {
                    TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) => {
                        ty = struct_layout
                            .fields
                            .iter()
                            .find(|field| field.name == *name)?
                            .ty
                            .clone();
                    }
                    _ => return None,
                }
            }
            ProjectionElem::UnionField { index, name } => {
                let layout = layouts.layout_for_name(&ty.canonical_name())?;
                let TypeLayout::Union(union_layout) = layout else {
                    return None;
                };
                let view = union_layout
                    .views
                    .iter()
                    .find(|field| field.index == *index && field.name == *name)?;
                ty = view.ty.clone();
            }
        }
    }

    Some(ty)
}

fn dispose_symbol_for_ty(
    layouts: &crate::mir::TypeLayoutTable,
    ty: &crate::mir::Ty,
) -> Option<String> {
    use crate::mir::TypeLayout;

    let crate::mir::Ty::Named(_) = ty else {
        return None;
    };
    let layout = layouts.layout_for_name(&ty.canonical_name())?;
    match layout {
        TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout.dispose.clone(),
        _ => None,
    }
}

fn scan_terminator(terminator: &crate::mir::Terminator, node: &mut SymbolGraphNode) {
    use crate::mir::Terminator;
    match terminator {
        Terminator::Goto { .. }
        | Terminator::Return
        | Terminator::Panic
        | Terminator::Unreachable
        | Terminator::Pending(_) => {}
        Terminator::SwitchInt { discr, .. } => scan_operand(discr, node),
        Terminator::Match { value, .. } => scan_place(value, node),
        Terminator::Call { func, args, .. } => {
            scan_operand(func, node);
            for arg in args {
                scan_operand(arg, node);
            }
        }
        Terminator::Yield { value, .. } => scan_operand(value, node),
        Terminator::Await { future, .. } => scan_place(future, node),
        Terminator::Throw { exception, ty } => {
            if let Some(exception) = exception {
                scan_operand(exception, node);
            }
            if let Some(ty) = ty {
                scan_type(ty, node);
            }
        }
    }
}

fn scan_rvalue(rvalue: &crate::mir::Rvalue, node: &mut SymbolGraphNode) {
    use crate::mir::Rvalue;
    match rvalue {
        Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => scan_operand(op, node),
        Rvalue::Binary { lhs, rhs, .. } => {
            scan_operand(lhs, node);
            scan_operand(rhs, node);
        }
        Rvalue::Aggregate { fields, .. } => {
            for field in fields {
                scan_operand(field, node);
            }
        }
        Rvalue::AddressOf { place, .. } | Rvalue::Len(place) => scan_place(place, node),
        Rvalue::SpanStackAlloc { length, source, .. } => {
            scan_operand(length, node);
            if let Some(source) = source {
                scan_operand(source, node);
            }
        }
        Rvalue::Cast { operand, .. } => scan_operand(operand, node),
        Rvalue::StringInterpolate { segments } => {
            for segment in segments {
                match segment {
                    crate::mir::InterpolatedStringSegment::Text { .. } => {}
                    crate::mir::InterpolatedStringSegment::Expr { operand, .. } => {
                        scan_operand(operand, node)
                    }
                }
            }
        }
        Rvalue::NumericIntrinsic(intrinsic) => {
            node.symbols.insert(intrinsic.symbol.clone());
            for operand in &intrinsic.operands {
                scan_operand(operand, node);
            }
            if let Some(place) = &intrinsic.out {
                scan_place(place, node);
            }
        }
        Rvalue::DecimalIntrinsic(intrinsic) => {
            scan_operand(&intrinsic.lhs, node);
            scan_operand(&intrinsic.rhs, node);
            if let Some(addend) = &intrinsic.addend {
                scan_operand(addend, node);
            }
            scan_operand(&intrinsic.rounding, node);
            scan_operand(&intrinsic.vectorize, node);
        }
        Rvalue::AtomicLoad { target, .. } => scan_place(target, node),
        Rvalue::AtomicRmw { target, value, .. } => {
            scan_place(target, node);
            scan_operand(value, node);
        }
        Rvalue::AtomicCompareExchange {
            target,
            expected,
            desired,
            ..
        } => {
            scan_place(target, node);
            scan_operand(expected, node);
            scan_operand(desired, node);
        }
        Rvalue::Pending(_) | Rvalue::StaticLoad { .. } | Rvalue::StaticRef { .. } => {}
    }
}

fn scan_operand(operand: &crate::mir::Operand, node: &mut SymbolGraphNode) {
    use crate::mir::Operand;
    match operand {
        Operand::Copy(place) | Operand::Move(place) => scan_place(place, node),
        Operand::Borrow(borrow) => scan_borrow(borrow, node),
        Operand::Mmio(_) => {}
        Operand::Const(constant) => collect_symbols_in_const(constant, node),
        Operand::Pending(pending) => scan_pending_operand(pending, node),
    }
}

fn scan_borrow(borrow: &crate::mir::BorrowOperand, node: &mut SymbolGraphNode) {
    scan_place(&borrow.place, node);
    scan_region(borrow.region, node);
}

fn scan_pending_operand(pending: &crate::mir::PendingOperand, node: &mut SymbolGraphNode) {
    if let Some(info) = pending.info.as_deref() {
        match info {
            crate::mir::PendingOperandInfo::FunctionGroup {
                path,
                candidates,
                receiver,
            } => {
                node.symbols.insert(path.clone());
                for candidate in candidates {
                    node.symbols.insert(candidate.qualified.clone());
                }
                if let Some(receiver) = receiver {
                    scan_operand(receiver, node);
                }
            }
        }
    } else if !pending.repr.is_empty() {
        node.symbols.insert(pending.repr.clone());
        if pending.repr.contains('.') {
            node.symbols.insert(pending.repr.replace('.', "::"));
        }
    }
}

fn scan_region(_region: crate::mir::RegionVar, _node: &mut SymbolGraphNode) {}

fn scan_place(place: &crate::mir::Place, node: &mut SymbolGraphNode) {
    for elem in &place.projection {
        if let crate::mir::ProjectionElem::Index(local) = elem {
            scan_local(*local, node);
        }
    }
}

fn scan_local(_local: crate::mir::LocalId, _node: &mut SymbolGraphNode) {}

fn scan_type(ty: &crate::mir::Ty, node: &mut SymbolGraphNode) {
    match ty {
        crate::mir::Ty::Named(nominal) => {
            node.symbols.insert(nominal.name.clone());
            for arg in &nominal.args {
                if let crate::mir::GenericArg::Type(inner) = arg {
                    scan_type(inner, node);
                }
            }
        }
        crate::mir::Ty::Array(array) => scan_type(&array.element, node),
        crate::mir::Ty::Vec(vec) => scan_type(&vec.element, node),
        crate::mir::Ty::Span(span) => scan_type(&span.element, node),
        crate::mir::Ty::ReadOnlySpan(span) => scan_type(&span.element, node),
        crate::mir::Ty::Rc(rc) => scan_type(&rc.element, node),
        crate::mir::Ty::Arc(arc) => scan_type(&arc.element, node),
        crate::mir::Ty::Tuple(elements) => {
            for elem in &elements.elements {
                scan_type(elem, node);
            }
        }
        crate::mir::Ty::Fn(fn_ty) => {
            scan_type(&fn_ty.ret, node);
            for param in &fn_ty.params {
                scan_type(param, node);
            }
        }
        crate::mir::Ty::Vector(vector) => scan_type(&vector.element, node),
        crate::mir::Ty::Pointer(ptr) => scan_type(&ptr.element, node),
        crate::mir::Ty::Ref(reference) => scan_type(&reference.element, node),
        crate::mir::Ty::Nullable(inner) => scan_type(inner, node),
        crate::mir::Ty::TraitObject(obj) => {
            for trait_name in &obj.traits {
                node.symbols.insert(trait_name.clone());
            }
        }
        _ => {}
    }
}

fn collect_symbols_in_const(value: &crate::mir::ConstOperand, node: &mut SymbolGraphNode) {
    collect_symbols_in_const_value(&value.value, node);
}

fn collect_symbols_in_const_value(value: &crate::mir::ConstValue, node: &mut SymbolGraphNode) {
    match value {
        crate::mir::ConstValue::Symbol(symbol) => {
            node.symbols.insert(symbol.clone());
        }
        crate::mir::ConstValue::Struct { fields, .. } => {
            for (_, field) in fields {
                collect_symbols_in_const_value(field, node);
            }
        }
        _ => {}
    }
}

fn index_function_names(functions: &[crate::mir::MirFunction]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, function) in functions.iter().enumerate() {
        map.entry(function.name.clone()).or_default().push(idx);
    }
    map
}

fn index_function_bases(functions: &[crate::mir::MirFunction]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, function) in functions.iter().enumerate() {
        map.entry(symbol_base(function.name.as_str()))
            .or_default()
            .push(idx);
    }
    map
}

fn build_default_arg_edges(
    records: &[crate::mir::DefaultArgumentRecord],
) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for record in records {
        map.entry(record.function.clone())
            .or_default()
            .push(record.internal.clone());
    }
    map
}

fn strip_signature(symbol: &str) -> &str {
    symbol.split('(').next().unwrap_or(symbol)
}

fn strip_instantiation(symbol: &str) -> &str {
    symbol.split('<').next().unwrap_or(symbol)
}

fn canonicalize_symbol(symbol: &str) -> String {
    let mut out = String::with_capacity(symbol.len());
    for ch in symbol.chars() {
        if ch.is_whitespace() {
            continue;
        }
        out.push(ch);
    }
    out.replace('.', "::")
}

fn symbol_base(symbol: &str) -> String {
    canonicalize_symbol(strip_instantiation(strip_signature(symbol)))
}

fn symbol_variants(symbol: &str) -> Vec<String> {
    let candidates = [
        symbol.to_string(),
        symbol.replace('.', "::"),
        strip_signature(symbol).to_string(),
        strip_signature(symbol).replace('.', "::"),
        strip_instantiation(symbol).to_string(),
        strip_instantiation(symbol).replace('.', "::"),
        strip_instantiation(strip_signature(symbol)).to_string(),
        strip_instantiation(strip_signature(symbol)).replace('.', "::"),
        canonicalize_symbol(symbol),
    ];

    let mut seen = HashSet::<String>::new();
    let mut out = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }
        if seen.insert(candidate.clone()) {
            out.push(candidate);
        }
    }
    out
}

fn reachable_from_roots(
    graph: &[SymbolGraphNode],
    name_to_indices: &HashMap<String, Vec<usize>>,
    base_name_to_indices: &HashMap<String, Vec<usize>>,
    default_arg_edges: &HashMap<String, Vec<String>>,
    function_names: &[String],
    roots: Vec<usize>,
) -> HashSet<usize> {
    let mut reachable = HashSet::new();
    let mut stack = roots;
    while let Some(index) = stack.pop() {
        if !reachable.insert(index) {
            continue;
        }
        if let Some(node) = graph.get(index) {
            for symbol in &node.symbols {
                let mut matched = false;
                for candidate in symbol_variants(symbol) {
                    if let Some(targets) = name_to_indices.get(candidate.as_str()) {
                        matched = true;
                        for target in targets {
                            stack.push(*target);
                        }
                        break;
                    }
                    if let Some(targets) = base_name_to_indices.get(&symbol_base(&candidate)) {
                        matched = true;
                        for target in targets {
                            stack.push(*target);
                        }
                        break;
                    }
                }
                if matched {
                    continue;
                }
                if let Some(targets) = base_name_to_indices.get(&symbol_base(symbol)) {
                    for target in targets {
                        stack.push(*target);
                    }
                    continue;
                }
                let canonical = canonicalize_symbol(symbol);
                let base = symbol_base(symbol);
                let stripped = strip_instantiation(strip_signature(symbol));
                for (name, indices) in name_to_indices {
                    if name.ends_with(symbol)
                        || name.ends_with(&canonical)
                        || name.ends_with(&base)
                        || name.ends_with(stripped)
                    {
                        for target in indices {
                            stack.push(*target);
                        }
                    }
                }
            }
        }
        if let Some(name) = function_names.get(index) {
            if let Some(targets) = default_arg_edges.get(name) {
                for target in targets {
                    if let Some(indices) = name_to_indices.get(target) {
                        for idx in indices {
                            stack.push(*idx);
                        }
                    }
                }
            }
        }
    }
    reachable
}
