use crate::clone_glue::SynthesisedCloneGlue;
use crate::drop_glue::SynthesisedDropGlue;
use crate::frontend::ast::items::{ClassMember, ImplMember, Item, NamespaceDecl, TraitMember};
use crate::frontend::ast::{ConstructorDecl, ExtensionMember, FunctionDecl, Module, Parameter};
use crate::frontend::diagnostics::Span;
use crate::mir::{
    Abi, BorrowOperand, DefaultArgumentRecord, GenericArg, InlineAsmOperandKind,
    InterpolatedStringSegment, LocalId, LocalKind, MatchArm, MirBody, MirFunction, MirModule,
    Operand, Pattern, Place, ProjectionElem, Rvalue, Statement, StatementKind, StaticId,
    Terminator, Ty, VariantPatternFields,
};
use std::collections::{HashMap, HashSet};

use super::allow::{LintAllowance, allowance_from_attributes};
use super::config::LintConfig;
use super::diagnostic::{LintDescriptor, LintDiagnostic, LintLevel, LintSuggestion};
use super::{LintModuleInfo, find_descriptor};

#[derive(Debug, Clone)]
struct FunctionAstInfo {
    name: String,
    span: Option<Span>,
    allowance: LintAllowance,
    param_allowances: Vec<LintAllowance>,
    param_names: Vec<String>,
    kind: AstFunctionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AstFunctionKind {
    Function,
    Method,
    Constructor,
    Testcase,
    ImplMethod,
    ExtensionMethod,
}

#[derive(Default)]
struct FunctionIndex {
    entries: Vec<FunctionAstInfo>,
    span_to_index: HashMap<(usize, usize), usize>,
    name_to_indices: HashMap<String, Vec<usize>>,
}

impl FunctionIndex {
    fn insert(&mut self, info: FunctionAstInfo) {
        let idx = self.entries.len();
        if let Some(span) = info.span {
            self.span_to_index.insert((span.start, span.end), idx);
        }
        self.name_to_indices
            .entry(info.name.clone())
            .or_default()
            .push(idx);
        self.entries.push(info);
    }

    fn find(&self, name: &str, span: Option<Span>) -> Option<&FunctionAstInfo> {
        if let Some(span) = span {
            if let Some(idx) = self.span_to_index.get(&(span.start, span.end)) {
                return self.entries.get(*idx);
            }
        }
        self.name_to_indices
            .get(name)
            .and_then(|indices| indices.first())
            .and_then(|idx| self.entries.get(*idx))
    }
}

#[derive(Default)]
struct FunctionGraphNode {
    symbol_edges: HashSet<String>,
    static_edges: HashSet<StaticId>,
    local_uses: Vec<usize>,
}

pub fn run_lints(
    config: &LintConfig,
    ast: &Module,
    modules: &[LintModuleInfo<'_>],
    unit_functions: &[Vec<usize>],
    mir_module: &MirModule,
    drop_glue: &[SynthesisedDropGlue],
    clone_glue: &[SynthesisedCloneGlue],
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    let Some(dead_code) = find_descriptor("dead_code") else {
        return diagnostics;
    };
    let Some(unused_param) = find_descriptor("unused_param") else {
        return diagnostics;
    };
    let type_named_ctor = find_descriptor("type_named_constructor");

    let function_index = build_function_index(ast);
    let function_to_module = map_functions_to_modules(unit_functions);
    let name_to_indices = index_function_names(&mir_module.functions);
    let graph = analyse_bodies(mir_module);
    let exports: HashSet<_> = mir_module
        .exports
        .iter()
        .map(|export| export.function.clone())
        .collect();
    let default_arg_edges = build_default_arg_edges(&mir_module.default_arguments);

    let drop_indices: HashSet<usize> = drop_glue.iter().map(|entry| entry.function_index).collect();
    let clone_indices: HashSet<usize> = clone_glue
        .iter()
        .map(|entry| entry.function_index)
        .collect();

    let reachable = reachable_functions(
        mir_module,
        &function_index,
        &function_to_module,
        modules,
        &graph,
        &name_to_indices,
        &exports,
        &drop_indices,
        &clone_indices,
        &default_arg_edges,
    );

    if let Some(descriptor) = type_named_ctor {
        diagnostics.extend(run_type_named_constructors(
            descriptor,
            config,
            mir_module,
            modules,
            &function_index,
            &function_to_module,
        ));
    }

    diagnostics.extend(run_dead_code(
        dead_code,
        config,
        mir_module,
        modules,
        &function_index,
        &function_to_module,
        &reachable,
    ));
    diagnostics.extend(run_unused_params(
        unused_param,
        config,
        mir_module,
        modules,
        &function_index,
        &function_to_module,
        &graph,
    ));

    diagnostics
}

fn build_function_index(ast: &Module) -> FunctionIndex {
    let mut index = FunctionIndex::default();
    let root_allow = allowance_from_attributes(&ast.namespace_attributes);
    collect_items(ast.namespace.clone(), &ast.items, &root_allow, &mut index);
    index
}

fn collect_items(
    namespace: Option<String>,
    items: &[Item],
    inherited_allow: &LintAllowance,
    index: &mut FunctionIndex,
) {
    for item in items {
        match item {
            Item::Namespace(ns) => collect_namespace(namespace.clone(), ns, inherited_allow, index),
            Item::Function(func) => {
                collect_function(namespace.as_deref(), func, inherited_allow, index)
            }
            Item::Struct(strct) => {
                let owner = qualify(namespace.as_deref(), &strct.name);
                let merged = inherited_allow.merged(&allowance_from_attributes(&strct.attributes));
                for (ctor_index, ctor) in strct.constructors.iter().enumerate() {
                    collect_constructor(
                        &owner,
                        ctor,
                        ctor_index,
                        AstFunctionKind::Constructor,
                        &merged,
                        index,
                    );
                }
                for method in &strct.methods {
                    collect_method(&owner, method, AstFunctionKind::Method, &merged, index);
                }
                collect_items(Some(owner.clone()), &strct.nested_types, &merged, index);
            }
            Item::Class(class) => {
                let owner = qualify(namespace.as_deref(), &class.name);
                let merged = inherited_allow.merged(&allowance_from_attributes(&class.attributes));
                for (member_index, member) in class.members.iter().enumerate() {
                    match member {
                        ClassMember::Method(method) => {
                            collect_method(&owner, method, AstFunctionKind::Method, &merged, index)
                        }
                        ClassMember::Constructor(ctor) => collect_constructor(
                            &owner,
                            ctor,
                            member_index,
                            AstFunctionKind::Constructor,
                            &merged,
                            index,
                        ),
                        ClassMember::Field(_)
                        | ClassMember::Property(_)
                        | ClassMember::Const(_) => {}
                    }
                }
                collect_items(Some(owner.clone()), &class.nested_types, &merged, index);
            }
            Item::Impl(impl_decl) => {
                let target_name = type_expr_name(&impl_decl.target);
                let owner = if target_name.contains("::") {
                    target_name
                } else {
                    qualify(namespace.as_deref(), &target_name)
                };
                let merged =
                    inherited_allow.merged(&allowance_from_attributes(&impl_decl.attributes));
                for member in &impl_decl.members {
                    if let ImplMember::Method(method) = member {
                        collect_method(&owner, method, AstFunctionKind::ImplMethod, &merged, index);
                    }
                }
            }
            Item::Extension(ext) => {
                let target_name = type_expr_name(&ext.target);
                let owner = if target_name.contains("::") {
                    target_name
                } else {
                    qualify(namespace.as_deref(), &target_name)
                };
                let merged = inherited_allow.merged(&allowance_from_attributes(&ext.attributes));
                for ExtensionMember::Method(method) in &ext.members {
                    collect_method(
                        &owner,
                        &method.function,
                        AstFunctionKind::ExtensionMethod,
                        &merged,
                        index,
                    );
                }
            }
            Item::TestCase(testcase) => {
                let name = qualify(namespace.as_deref(), &testcase.name);
                let allowance =
                    inherited_allow.merged(&allowance_from_attributes(&testcase.attributes));
                let span = testcase.body.span.or(testcase
                    .signature
                    .as_ref()
                    .and_then(|sig| sig.throws.as_ref().and_then(|throws| throws.span)));
                let (param_names, param_allowances) = testcase
                    .signature
                    .as_ref()
                    .map(|sig| build_param_allowances(&allowance, &sig.parameters))
                    .unwrap_or_default();
                index.insert(FunctionAstInfo {
                    name,
                    span,
                    allowance,
                    param_allowances,
                    param_names,
                    kind: AstFunctionKind::Testcase,
                });
            }
            Item::Trait(trait_decl) => {
                let owner = qualify(namespace.as_deref(), &trait_decl.name);
                let merged =
                    inherited_allow.merged(&allowance_from_attributes(&trait_decl.attributes));
                for member in &trait_decl.members {
                    if let TraitMember::Method(method) = member {
                        collect_method(&owner, method, AstFunctionKind::Method, &merged, index);
                    }
                }
            }
            Item::Interface(iface) => {
                let owner = qualify(namespace.as_deref(), &iface.name);
                let merged = inherited_allow.merged(&allowance_from_attributes(&iface.attributes));
                for member in &iface.members {
                    if let crate::frontend::ast::items::InterfaceMember::Method(method) = member {
                        collect_method(&owner, method, AstFunctionKind::Method, &merged, index);
                    }
                }
            }
            Item::Union(_)
            | Item::Enum(_)
            | Item::Import(_)
            | Item::Const(_)
            | Item::Static(_)
            | Item::Delegate(_)
            | Item::TypeAlias(_) => {}
        }
    }
}

fn collect_namespace(
    parent_namespace: Option<String>,
    ns: &NamespaceDecl,
    inherited_allow: &LintAllowance,
    index: &mut FunctionIndex,
) {
    let allowance = inherited_allow.merged(&allowance_from_attributes(&ns.attributes));
    let name = qualify(parent_namespace.as_deref(), &ns.name);
    collect_items(Some(name), &ns.items, &allowance, index);
}

fn collect_function(
    namespace: Option<&str>,
    func: &FunctionDecl,
    inherited_allow: &LintAllowance,
    index: &mut FunctionIndex,
) {
    let name = qualify(namespace, &func.name);
    let allowance = inherited_allow.merged(&allowance_from_attributes(&func.attributes));
    let (param_names, param_allowances) =
        build_param_allowances(&allowance, &func.signature.parameters);
    let span = func
        .body
        .as_ref()
        .and_then(|block| block.span)
        .or_else(|| func.doc.as_ref().and_then(|_| None));
    index.insert(FunctionAstInfo {
        name,
        span,
        allowance,
        param_allowances,
        param_names,
        kind: AstFunctionKind::Function,
    });
}

fn collect_method(
    owner: &str,
    method: &FunctionDecl,
    kind: AstFunctionKind,
    inherited_allow: &LintAllowance,
    index: &mut FunctionIndex,
) {
    let name = format!("{owner}::{}", method.name);
    let allowance = inherited_allow.merged(&allowance_from_attributes(&method.attributes));
    let (param_names, param_allowances) =
        build_param_allowances(&allowance, &method.signature.parameters);
    let span = method.body.as_ref().and_then(|body| body.span);
    index.insert(FunctionAstInfo {
        name,
        span,
        allowance,
        param_allowances,
        param_names,
        kind,
    });
}

fn collect_constructor(
    owner: &str,
    ctor: &ConstructorDecl,
    index_in_owner: usize,
    kind: AstFunctionKind,
    inherited_allow: &LintAllowance,
    index: &mut FunctionIndex,
) {
    let name = format!("{owner}::init#{index_in_owner}");
    let allowance = inherited_allow.merged(&allowance_from_attributes(&ctor.attributes));
    let (param_names, param_allowances) = build_param_allowances(&allowance, &ctor.parameters);
    let span = ctor.body.as_ref().and_then(|body| body.span).or(ctor.span);
    index.insert(FunctionAstInfo {
        name,
        span,
        allowance,
        param_allowances,
        param_names,
        kind,
    });
}

fn build_param_allowances(
    parent_allow: &LintAllowance,
    params: &[Parameter],
) -> (Vec<String>, Vec<LintAllowance>) {
    let mut names = Vec::with_capacity(params.len());
    let mut allowances = Vec::with_capacity(params.len());
    for param in params {
        names.push(param.name.clone());
        let merged = parent_allow.merged(&allowance_from_attributes(&param.attributes));
        allowances.push(merged);
    }
    (names, allowances)
}

fn map_functions_to_modules(unit_functions: &[Vec<usize>]) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    for (module_idx, indices) in unit_functions.iter().enumerate() {
        for index in indices {
            map.insert(*index, module_idx);
        }
    }
    map
}

fn index_function_names(functions: &[MirFunction]) -> HashMap<String, Vec<usize>> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, function) in functions.iter().enumerate() {
        for key in function_name_keys(&function.name) {
            map.entry(key).or_default().push(idx);
        }
    }
    map
}

fn function_name_keys(name: &str) -> Vec<String> {
    let mut keys = Vec::new();
    keys.push(name.to_string());

    let normalized = if name.contains('.') {
        Some(name.replace('.', "::"))
    } else {
        None
    };
    if let Some(key) = normalized.as_ref() {
        keys.push(key.clone());
    }

    if let Some(stripped) = strip_function_ordinal_suffix(name) {
        keys.push(stripped);
    }
    if let Some(key) = normalized.as_ref() {
        if let Some(stripped) = strip_function_ordinal_suffix(key) {
            keys.push(stripped);
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

fn strip_function_ordinal_suffix(name: &str) -> Option<String> {
    fn strip_segment(segment: &str) -> Option<&str> {
        let (base, suffix) = segment.rsplit_once('#')?;
        (!base.is_empty() && !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
            .then_some(base)
    }

    if let Some((prefix, last)) = name.rsplit_once("::") {
        let base = strip_segment(last)?;
        Some(format!("{prefix}::{base}"))
    } else {
        strip_segment(name).map(str::to_string)
    }
}

fn analyse_bodies(module: &MirModule) -> Vec<FunctionGraphNode> {
    module
        .functions
        .iter()
        .map(|function| analyse_body(&function.body))
        .collect()
}

fn analyse_body(body: &MirBody) -> FunctionGraphNode {
    let mut node = FunctionGraphNode {
        symbol_edges: HashSet::new(),
        static_edges: HashSet::new(),
        local_uses: vec![0; body.locals.len()],
    };
    for block in &body.blocks {
        for statement in &block.statements {
            scan_statement(statement, &mut node);
        }
        if let Some(terminator) = &block.terminator {
            scan_terminator(terminator, &mut node);
        }
    }
    node
}

fn scan_statement(statement: &Statement, node: &mut FunctionGraphNode) {
    match &statement.kind {
        StatementKind::Assign { place, value } => {
            record_place(place, node);
            scan_rvalue(value, node);
        }
        StatementKind::StorageLive(_)
        | StatementKind::StorageDead(_)
        | StatementKind::MarkFallibleHandled { .. } => {}
        StatementKind::Deinit(place)
        | StatementKind::Drop { place, .. }
        | StatementKind::Borrow { place, .. }
        | StatementKind::Retag { place }
        | StatementKind::DeferDrop { place }
        | StatementKind::DefaultInit { place }
        | StatementKind::ZeroInit { place } => record_place(place, node),
        StatementKind::ZeroInitRaw { pointer, length } => {
            scan_operand(pointer, node);
            scan_operand(length, node);
        }
        StatementKind::AtomicStore { target, value, .. } => {
            record_place(target, node);
            scan_operand(value, node);
        }
        StatementKind::AtomicFence { .. }
        | StatementKind::EnterUnsafe
        | StatementKind::ExitUnsafe
        | StatementKind::Eval(_)
        | StatementKind::Nop => {}
        StatementKind::EnqueueKernel {
            stream,
            kernel,
            args,
            completion,
        } => {
            record_place(stream, node);
            scan_operand(kernel, node);
            for arg in args {
                scan_operand(arg, node);
            }
            if let Some(event) = completion {
                record_place(event, node);
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
            record_place(stream, node);
            record_place(dst, node);
            record_place(src, node);
            scan_operand(bytes, node);
            if let Some(event) = completion {
                record_place(event, node);
            }
        }
        StatementKind::RecordEvent { stream, event } => {
            record_place(stream, node);
            record_place(event, node);
        }
        StatementKind::WaitEvent { event, stream } => {
            record_place(event, node);
            if let Some(stream) = stream {
                record_place(stream, node);
            }
        }
        StatementKind::Assert { cond, .. } => scan_operand(cond, node),
        StatementKind::Pending(_) => {}
        StatementKind::MmioStore { value, .. } => scan_operand(value, node),
        StatementKind::StaticStore { id, value } => {
            node.static_edges.insert(*id);
            scan_operand(value, node);
        }
        StatementKind::InlineAsm(asm) => {
            for operand in &asm.operands {
                match &operand.kind {
                    InlineAsmOperandKind::In { value } | InlineAsmOperandKind::Const { value } => {
                        scan_operand(value, node)
                    }
                    InlineAsmOperandKind::InOut {
                        input: value,
                        output,
                        ..
                    } => {
                        scan_operand(value, node);
                        record_place(output, node);
                    }
                    InlineAsmOperandKind::Out { place, .. } => {
                        record_place(place, node);
                    }
                    InlineAsmOperandKind::Sym { .. } => {}
                }
            }
        }
    }
}

fn scan_terminator(terminator: &Terminator, node: &mut FunctionGraphNode) {
    match terminator {
        Terminator::Goto { .. }
        | Terminator::Return
        | Terminator::Panic
        | Terminator::Unreachable
        | Terminator::Pending(_) => {}
        Terminator::SwitchInt { discr, .. } => scan_operand(discr, node),
        Terminator::Match { value, arms, .. } => {
            record_place(value, node);
            for arm in arms {
                scan_match_arm(arm, node);
            }
        }
        Terminator::Call {
            func,
            args,
            destination,
            ..
        } => {
            scan_operand(func, node);
            for arg in args {
                scan_operand(arg, node);
            }
            if let Some(dest) = destination {
                record_place(dest, node);
            }
        }
        Terminator::Yield { value, .. } => {
            scan_operand(value, node);
        }
        Terminator::Await {
            future,
            destination,
            ..
        } => {
            record_place(future, node);
            if let Some(dest) = destination {
                record_place(dest, node);
            }
        }
        Terminator::Throw { exception, .. } => {
            if let Some(exception) = exception {
                scan_operand(exception, node);
            }
        }
    }
}

fn scan_match_arm(arm: &MatchArm, node: &mut FunctionGraphNode) {
    scan_pattern(&arm.pattern, node);
    for binding in &arm.bindings {
        record_local(binding.local, node);
    }
}

fn scan_pattern(pattern: &Pattern, node: &mut FunctionGraphNode) {
    match pattern {
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Type(_) => {}
        Pattern::Binding(_) => {}
        Pattern::Tuple(entries) => {
            for entry in entries {
                scan_pattern(entry, node);
            }
        }
        Pattern::Struct { fields, .. } => {
            for field in fields {
                scan_pattern(&field.pattern, node);
            }
        }
        Pattern::Enum { fields, .. } => match fields {
            VariantPatternFields::Unit => {}
            VariantPatternFields::Tuple(entries) => {
                for entry in entries {
                    scan_pattern(entry, node);
                }
            }
            VariantPatternFields::Struct(entries) => {
                for entry in entries {
                    scan_pattern(&entry.pattern, node);
                }
            }
        },
    }
}

fn scan_rvalue(value: &Rvalue, node: &mut FunctionGraphNode) {
    match value {
        Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => scan_operand(op, node),
        Rvalue::Len(place) => record_place(place, node),
        Rvalue::Binary { lhs, rhs, .. } => {
            scan_operand(lhs, node);
            scan_operand(rhs, node);
        }
        Rvalue::Aggregate { fields, .. } => {
            for field in fields {
                scan_operand(field, node);
            }
        }
        Rvalue::AddressOf { place, .. } => record_place(place, node),
        Rvalue::SpanStackAlloc { length, source, .. } => {
            scan_operand(length, node);
            if let Some(source) = source {
                scan_operand(source, node);
            }
        }
        Rvalue::Cast { operand, .. } => scan_operand(operand, node),
        Rvalue::StringInterpolate { segments } => {
            for segment in segments {
                if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                    scan_operand(operand, node);
                }
            }
        }
        Rvalue::NumericIntrinsic(intrinsic) => {
            for operand in &intrinsic.operands {
                scan_operand(operand, node);
            }
            if let Some(out) = &intrinsic.out {
                record_place(out, node);
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
        Rvalue::AtomicLoad { target, .. } => record_place(target, node),
        Rvalue::AtomicRmw { target, value, .. } => {
            record_place(target, node);
            scan_operand(value, node);
        }
        Rvalue::AtomicCompareExchange {
            target,
            expected,
            desired,
            ..
        } => {
            record_place(target, node);
            scan_operand(expected, node);
            scan_operand(desired, node);
        }
        Rvalue::Pending(_) => {}
        Rvalue::StaticLoad { id } => {
            node.static_edges.insert(*id);
        }
        Rvalue::StaticRef { id } => {
            node.static_edges.insert(*id);
        }
    }
}

fn scan_operand(op: &Operand, node: &mut FunctionGraphNode) {
    match op {
        Operand::Copy(place) | Operand::Move(place) => record_place(place, node),
        Operand::Borrow(BorrowOperand { place, .. }) => record_place(place, node),
        Operand::Mmio(_) => {}
        Operand::Const(constant) => {
            if let Some(symbol) = constant.symbol_name() {
                node.symbol_edges.insert(symbol.to_string());
            }
        }
        Operand::Pending(_) => {}
    }
}

fn record_place(place: &Place, node: &mut FunctionGraphNode) {
    record_local(place.local, node);
    for proj in &place.projection {
        if let ProjectionElem::Index(local) = proj {
            record_local(*local, node);
        }
    }
}

fn record_local(local: LocalId, node: &mut FunctionGraphNode) {
    if let Some(entry) = node.local_uses.get_mut(local.0) {
        *entry += 1;
    }
}

fn reachable_functions(
    mir_module: &MirModule,
    ast_index: &FunctionIndex,
    function_to_module: &HashMap<usize, usize>,
    modules: &[LintModuleInfo<'_>],
    graph: &[FunctionGraphNode],
    name_to_indices: &HashMap<String, Vec<usize>>,
    exports: &HashSet<String>,
    drop_indices: &HashSet<usize>,
    clone_indices: &HashSet<usize>,
    default_arg_edges: &HashMap<String, Vec<String>>,
) -> HashSet<usize> {
    let mut reachable = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();

    for (idx, function) in mir_module.functions.iter().enumerate() {
        if function.name.ends_with("::Main") {
            stack.push(idx);
            continue;
        }
        if drop_indices.contains(&idx) || clone_indices.contains(&idx) {
            stack.push(idx);
            continue;
        }
        if let Some(module_idx) = function_to_module.get(&idx) {
            if modules
                .get(*module_idx)
                .map(|module| module.is_stdlib)
                .unwrap_or(false)
            {
                stack.push(idx);
                continue;
            }
        }
        if matches!(function.signature.abi, Abi::Extern(_)) {
            stack.push(idx);
            continue;
        }
        if exports.contains(&function.name) {
            stack.push(idx);
            continue;
        }
        if let Some(info) = ast_index.find(&function.name, function.span) {
            if matches!(info.kind, AstFunctionKind::Testcase) {
                stack.push(idx);
                continue;
            }
        } else {
            // Skip linting functions we cannot map back to source.
            stack.push(idx);
        }
    }

    while let Some(index) = stack.pop() {
        if !reachable.insert(index) {
            continue;
        }
        if let Some(node) = graph.get(index) {
            for symbol in &node.symbol_edges {
                for key in symbol_lookup_keys(symbol) {
                    if let Some(targets) = name_to_indices.get(&key) {
                        for target in targets {
                            stack.push(*target);
                        }
                        continue;
                    }
                    for (name, indices) in name_to_indices {
                        if name.ends_with(&key) {
                            for target in indices {
                                stack.push(*target);
                            }
                        }
                    }
                }
            }
        }
        if let Some(function) = mir_module.functions.get(index) {
            if let Some(targets) = default_arg_edges.get(&function.name) {
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

fn symbol_lookup_keys(symbol: &str) -> Vec<String> {
    let mut keys = Vec::new();
    keys.push(symbol.to_string());

    if symbol.contains('.') {
        keys.push(symbol.replace('.', "::"));
    }

    let snapshot = keys.clone();
    for key in snapshot {
        if let Some(stripped) = strip_function_ordinal_suffix(&key) {
            keys.push(stripped);
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

fn run_dead_code(
    descriptor: &'static LintDescriptor,
    config: &LintConfig,
    mir_module: &MirModule,
    modules: &[LintModuleInfo<'_>],
    ast_index: &FunctionIndex,
    function_to_module: &HashMap<usize, usize>,
    reachable: &HashSet<usize>,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    for (idx, function) in mir_module.functions.iter().enumerate() {
        if reachable.contains(&idx) {
            continue;
        }
        if signature_contains_generic_params(&function.signature) {
            continue;
        }
        let Some(module_idx) = function_to_module.get(&idx) else {
            continue;
        };
        let Some(module) = modules.get(*module_idx) else {
            continue;
        };
        if module.is_stdlib {
            continue;
        }
        let Some(info) = ast_index.find(&function.name, function.span) else {
            continue;
        };
        let level = effective_level(descriptor, config, &info.allowance);
        if matches!(level, LintLevel::Allow) {
            continue;
        }

        diagnostics.push(LintDiagnostic {
            descriptor,
            level,
            message: format!("function `{}` is never used", function.name),
            file: module.path.to_path_buf(),
            span: info.span.or(function.span),
            suggestions: vec![LintSuggestion::new(
                "call this function or annotate it with `@allow(dead_code)`",
                info.span.or(function.span),
                None,
            )],
        });
    }
    diagnostics
}

fn signature_contains_generic_params(signature: &crate::mir::FnSig) -> bool {
    signature.params.iter().any(ty_contains_generic_param)
        || ty_contains_generic_param(&signature.ret)
        || signature.effects.iter().any(ty_contains_generic_param)
}

fn ty_contains_generic_param(ty: &Ty) -> bool {
    match ty {
        Ty::Named(named) => {
            if named.args.is_empty() {
                let name = named.name.as_str();
                if name == "T"
                    || (name.starts_with('T')
                        && name
                            .chars()
                            .nth(1)
                            .is_some_and(|ch| ch.is_ascii_uppercase()))
                {
                    return true;
                }
            }
            named.args.iter().any(|arg| match arg {
                GenericArg::Type(inner) => ty_contains_generic_param(inner),
                GenericArg::Const(_) => false,
            })
        }
        Ty::Array(array) => ty_contains_generic_param(&array.element),
        Ty::Vec(vec) => ty_contains_generic_param(&vec.element),
        Ty::Span(span) => ty_contains_generic_param(&span.element),
        Ty::ReadOnlySpan(span) => ty_contains_generic_param(&span.element),
        Ty::Rc(rc) => ty_contains_generic_param(&rc.element),
        Ty::Arc(arc) => ty_contains_generic_param(&arc.element),
        Ty::Tuple(tuple) => tuple.elements.iter().any(ty_contains_generic_param),
        Ty::Fn(fn_ty) => {
            fn_ty.params.iter().any(ty_contains_generic_param)
                || ty_contains_generic_param(&fn_ty.ret)
        }
        Ty::Vector(vector) => ty_contains_generic_param(&vector.element),
        Ty::Pointer(ptr) => ty_contains_generic_param(&ptr.element),
        Ty::Ref(reference) => ty_contains_generic_param(&reference.element),
        Ty::Nullable(inner) => ty_contains_generic_param(inner),
        Ty::TraitObject(obj) => obj.traits.iter().any(|name| {
            name == "T"
                || (name.starts_with('T')
                    && name
                        .chars()
                        .nth(1)
                        .is_some_and(|ch| ch.is_ascii_uppercase()))
        }),
        Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => false,
    }
}

fn run_unused_params(
    descriptor: &'static LintDescriptor,
    config: &LintConfig,
    mir_module: &MirModule,
    modules: &[LintModuleInfo<'_>],
    ast_index: &FunctionIndex,
    function_to_module: &HashMap<usize, usize>,
    graph: &[FunctionGraphNode],
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    for (idx, function) in mir_module.functions.iter().enumerate() {
        let Some(module_idx) = function_to_module.get(&idx) else {
            continue;
        };
        let Some(module) = modules.get(*module_idx) else {
            continue;
        };
        if module.is_stdlib {
            continue;
        }
        let Some(info) = ast_index.find(&function.name, function.span) else {
            continue;
        };
        let Some(node) = graph.get(idx) else {
            continue;
        };
        for (local_idx, local) in function.body.locals.iter().enumerate() {
            let LocalKind::Arg(arg_idx) = local.kind else {
                continue;
            };
            if arg_idx >= info.param_allowances.len() {
                continue;
            };
            if local
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with('_'))
            {
                continue;
            }
            let param_allow = &info.param_allowances[arg_idx];
            let level = effective_level(descriptor, config, param_allow);
            if matches!(level, LintLevel::Allow) {
                continue;
            }
            let uses = node.local_uses.get(local_idx).copied().unwrap_or(0);
            if uses == 0 {
                let name = local
                    .name
                    .clone()
                    .or_else(|| info.param_names.get(arg_idx).cloned())
                    .unwrap_or_else(|| format!("arg{local_idx}"));
                diagnostics.push(LintDiagnostic {
                    descriptor,
                    level,
                    message: format!("parameter `{name}` is never used"),
                    file: module.path.to_path_buf(),
                    span: local.span.or(info.span).or(function.span),
                    suggestions: vec![LintSuggestion::new(
                        format!("prefix `{name}` with `_` to silence this lint"),
                        local.span.or(info.span).or(function.span),
                        Some(format!("_{name}")),
                    )],
                });
            }
        }
    }
    diagnostics
}

fn run_type_named_constructors(
    descriptor: &'static LintDescriptor,
    config: &LintConfig,
    mir_module: &MirModule,
    modules: &[LintModuleInfo<'_>],
    ast_index: &FunctionIndex,
    function_to_module: &HashMap<usize, usize>,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    for (idx, function) in mir_module.functions.iter().enumerate() {
        if function.name.contains("::init#") {
            continue;
        }
        let Some(module_idx) = function_to_module.get(&idx) else {
            continue;
        };
        let Some(module) = modules.get(*module_idx) else {
            continue;
        };
        if module.is_stdlib {
            continue;
        }
        let parts: Vec<_> = function
            .name
            .split("::")
            .filter(|segment| !segment.is_empty())
            .collect();
        if parts.len() < 2 {
            continue;
        }
        let ty_name = parts[parts.len() - 2];
        let func_name = parts[parts.len() - 1];
        if ty_name != func_name {
            continue;
        }
        let Some(info) = ast_index.find(&function.name, function.span) else {
            continue;
        };
        let level = effective_level(descriptor, config, &info.allowance);
        if matches!(level, LintLevel::Allow) {
            continue;
        }

        let span = info.span.or(function.span);
        diagnostics.push(LintDiagnostic {
            descriptor,
            level,
            message: format!(
                "constructors for `{}` must use `init(...)` instead of repeating the type name",
                ty_name
            ),
            file: module.path.to_path_buf(),
            span,
            suggestions: vec![LintSuggestion::new(
                "rename this constructor to `init`",
                span,
                Some("init".to_string()),
            )],
        });
    }
    diagnostics
}

fn effective_level(
    descriptor: &LintDescriptor,
    config: &LintConfig,
    allowance: &LintAllowance,
) -> LintLevel {
    if allowance.allows(descriptor.category, descriptor.name) {
        LintLevel::Allow
    } else {
        config.level_for(
            descriptor.name,
            descriptor.category,
            descriptor.default_level,
        )
    }
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
            let mut name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            if name_parts.is_empty() {
                prefix_parts.join("::")
            } else if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else {
                prefix_parts.append(&mut name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

fn type_expr_name(expr: &crate::frontend::ast::types::TypeExpr) -> String {
    if expr.base.is_empty() {
        expr.name.replace('.', "::")
    } else {
        let mut parts = expr.base.clone();
        let canonical = expr.name.replace('.', "::");
        if parts
            .last()
            .is_some_and(|segment| segment.replace('.', "::") == canonical)
        {
            parts.join("::")
        } else {
            parts.push(canonical);
            parts.join("::")
        }
    }
}

fn build_default_arg_edges(records: &[DefaultArgumentRecord]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for record in records {
        map.entry(record.function.clone())
            .or_default()
            .push(record.internal.clone());
    }
    map
}
