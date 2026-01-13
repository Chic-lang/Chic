use std::collections::HashMap;

use crate::frontend::ast::{Expression, SwitchSection};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::{ImportResolver, Resolution as ImportResolution};
use crate::mir::data::{
    BlockId, ConstValue, LocalId, Pattern, PatternBinding, PatternBindingMode,
    PatternBindingMutability, PatternProjectionElem, Ty,
};
use crate::mir::layout::{TypeLayout, TypeLayoutTable};

use super::super::expr::ExprNode;

pub(super) fn is_pin_type_name(name: &str) -> bool {
    let segment = name
        .rsplit(['.', ':'])
        .find(|part| !part.trim().is_empty())
        .unwrap_or(name);
    let trimmed = segment.trim();
    trimmed.starts_with("Pin<") && trimmed.ends_with('>')
}

#[derive(Debug, Default, Clone)]
pub(super) struct ScopeFrame {
    pub(super) bindings: HashMap<String, LocalId>,
    pub(super) locals: Vec<ScopeLocal>,
    pub(super) consts: HashMap<String, ConstValue>,
    pub(super) local_functions: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub(super) struct ScopeLocal {
    pub(super) local: LocalId,
    pub(super) span: Option<Span>,
    pub(super) live: bool,
}

#[derive(Debug, Clone)]
pub(super) struct LabelState {
    pub(super) block: BlockId,
    pub(super) scope_depth: usize,
    pub(super) defined: bool,
    pub(super) span: Option<Span>,
}

#[derive(Debug, Clone)]
pub(super) struct ScopeLocalSnapshot {
    pub(super) local: LocalId,
    pub(super) span: Option<Span>,
}

#[derive(Debug, Clone)]
pub(super) struct ScopeSnapshot {
    pub(super) depth: usize,
    pub(super) locals: Vec<ScopeLocalSnapshot>,
}

#[derive(Debug, Clone)]
pub(super) struct PendingGoto {
    pub(super) block: BlockId,
    pub(super) span: Option<Span>,
    pub(super) source_depth: usize,
    pub(super) scope_snapshot: Vec<ScopeSnapshot>,
}

#[derive(Clone, Copy)]
pub(super) struct TryContext {
    pub(super) exception_local: LocalId,
    pub(super) exception_flag: Option<LocalId>,
    pub(super) dispatch_block: Option<BlockId>,
    pub(super) finally_entry: Option<BlockId>,
    pub(super) after_block: BlockId,
    pub(super) unhandled_block: Option<BlockId>,
    pub(super) unwind_capture: Option<BlockId>,
    pub(super) scope_depth: usize,
}

#[derive(Clone, Copy)]
pub(super) struct SwitchTarget {
    pub(super) block: BlockId,
    pub(super) allows_goto: bool,
    pub(super) scope_depth: usize,
}

#[derive(Clone)]
pub(super) struct SwitchCase {
    pub(super) pattern: CasePatternKind,
    pub(super) pre_guards: Vec<GuardMetadata>,
    pub(super) guards: Vec<GuardMetadata>,
    pub(super) body_block: BlockId,
    pub(super) span: Option<Span>,
    pub(super) pattern_span: Option<Span>,
    pub(super) bindings: Vec<PatternBinding>,
    pub(super) list_plan: Option<ListDestructurePlan>,
}

#[derive(Clone)]
pub(super) enum CasePatternKind {
    Wildcard,
    Literal(ConstValue),
    Complex(Pattern),
}

#[derive(Clone)]
pub(super) struct GuardMetadata {
    pub(super) expr: Expression,
    pub(super) node: Option<ExprNode>,
}

#[derive(Clone, Copy)]
pub(super) struct SwitchSectionInfo {
    pub(super) body_block: BlockId,
    pub(super) section_index: usize,
    pub(super) span: Option<Span>,
}

pub(super) struct SwitchContext {
    pub(super) join_block: BlockId,
    pub(super) default_target: Option<SwitchTarget>,
    pub(super) label_map: HashMap<String, SwitchTarget>,
    pub(super) binding_name: String,
    pub(super) scope_depth: usize,
}

impl SwitchContext {
    pub(super) fn new(join_block: BlockId, binding_name: String, scope_depth: usize) -> Self {
        Self {
            join_block,
            default_target: None,
            label_map: HashMap::new(),
            binding_name,
            scope_depth,
        }
    }
}

pub(super) struct ParsedCasePattern {
    pub(super) kind: CasePatternKind,
    pub(super) key: Option<String>,
    pub(super) pre_guards: Vec<Expression>,
    pub(super) post_guards: Vec<Expression>,
    pub(super) bindings: Vec<BindingSpec>,
    pub(super) list_plan: Option<ListDestructurePlan>,
}

#[derive(Clone)]
pub(super) struct BindingSpec {
    pub(super) name: String,
    pub(super) projection: Vec<PatternProjectionElem>,
    pub(super) span: Option<Span>,
    pub(super) mutability: PatternBindingMutability,
    pub(super) mode: PatternBindingMode,
}

#[derive(Clone)]
pub(super) struct ListIndexSpec {
    pub(super) local: LocalId,
    pub(super) offset: usize,
    pub(super) from_end: bool,
}

#[derive(Clone)]
pub(super) struct ListDestructurePlan {
    pub(super) length_local: LocalId,
    pub(super) indices: Vec<ListIndexSpec>,
    pub(super) span: Option<Span>,
}

pub(super) fn literal_key_from_const(value: &ConstValue) -> String {
    match value {
        ConstValue::Int(v) | ConstValue::Int32(v) => format!("int:{v}"),
        ConstValue::UInt(v) => format!("uint:{v}"),
        ConstValue::Float(v) => format!("float:{}", v.hex_bits()),
        ConstValue::Decimal(value) => format!("decimal:{}", value.to_encoding()),
        ConstValue::Bool(v) => format!("bool:{v}"),
        ConstValue::Char(ch) => format!("char:{:04x}", *ch as u32),
        ConstValue::Str { value, .. } => format!("string:{value}"),
        ConstValue::Symbol(sym) => format!("symbol:{sym}"),
        ConstValue::Enum {
            type_name,
            variant,
            discriminant,
        } => format!("enum:{type_name}::{variant}@{discriminant}"),
        ConstValue::Struct { type_name, .. } => format!("struct:{type_name}"),
        ConstValue::Null => "null".into(),
        ConstValue::Unit => "unit".into(),
        ConstValue::Unknown => "unknown".into(),
        ConstValue::RawStr(value) => format!("raw:{value}"),
    }
}

pub(super) fn switch_section_span(section: &SwitchSection, fallback: Option<Span>) -> Option<Span> {
    section
        .statements
        .iter()
        .find_map(|stmt| stmt.span)
        .or(fallback)
}

pub(crate) fn resolve_type_layout_name(
    type_layouts: &TypeLayoutTable,
    import_resolver: Option<&ImportResolver>,
    namespace: Option<&str>,
    context_type: Option<&str>,
    name: &str,
) -> Option<String> {
    let stripped_owned;
    let stripped_name = match strip_type_name_generic_args(name) {
        std::borrow::Cow::Borrowed(value) => value,
        std::borrow::Cow::Owned(value) => {
            stripped_owned = value;
            stripped_owned.as_str()
        }
    };

    if type_layouts.types.contains_key(stripped_name) {
        return Some(stripped_name.to_string());
    }

    if let Some(resolver) = import_resolver {
        let segments = split_name_segments(stripped_name);
        if !segments.is_empty() {
            if let ImportResolution::Found(candidate) =
                resolver.resolve_type(&segments, namespace, context_type, |candidate| {
                    type_layouts.types.contains_key(candidate)
                })
            {
                return Some(candidate);
            }
        }
    }

    if stripped_name.contains("::") {
        return None;
    }

    if let Some(mut current) = namespace {
        loop {
            let candidate = format!("{current}::{stripped_name}");
            if type_layouts.types.contains_key(&candidate) {
                return Some(candidate);
            }
            match current.rfind("::") {
                Some(pos) => current = &current[..pos],
                None => break,
            }
        }
    }

    if type_layouts.types.contains_key(stripped_name) {
        Some(stripped_name.to_string())
    } else {
        None
    }
}

pub(crate) fn find_type_layout<'a>(
    type_layouts: &'a TypeLayoutTable,
    import_resolver: Option<&ImportResolver>,
    namespace: Option<&str>,
    context_type: Option<&str>,
    name: &str,
) -> Option<&'a TypeLayout> {
    let resolved =
        resolve_type_layout_name(type_layouts, import_resolver, namespace, context_type, name)?;
    type_layouts.types.get(&resolved)
}

fn builtin_size_and_align(type_layouts: &TypeLayoutTable, name: &str) -> Option<(usize, usize)> {
    let base = name.rsplit("::").next().unwrap_or(name);
    let registry = &type_layouts.primitive_registry;
    registry
        .size_align_for_name(
            base,
            super::pointer_size() as u32,
            super::pointer_align() as u32,
        )
        .map(|(size, align)| (size as usize, align as usize))
}

pub(crate) fn type_size_and_align_for_named_type(
    type_layouts: &TypeLayoutTable,
    import_resolver: Option<&ImportResolver>,
    namespace: Option<&str>,
    context_type: Option<&str>,
    name: &str,
) -> Option<(usize, usize)> {
    if let Some(size) = builtin_size_and_align(type_layouts, name) {
        return Some(size);
    }

    let layout = find_type_layout(type_layouts, import_resolver, namespace, context_type, name)?;
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
        TypeLayout::Enum(data) => data.size.zip(data.align),
        TypeLayout::Union(data) => data.size.zip(data.align),
    }
}

pub(crate) fn type_size_and_align_for_ty(
    ty: &Ty,
    type_layouts: &TypeLayoutTable,
    import_resolver: Option<&ImportResolver>,
    namespace: Option<&str>,
    context_type: Option<&str>,
) -> Option<(usize, usize)> {
    match ty {
        Ty::Unit => Some((0, super::MIN_ALIGN)),
        Ty::Unknown => None,
        Ty::Array(array) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &Ty::Array(array.clone()).canonical_name(),
        ),
        Ty::Vec(vec) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &Ty::Vec(vec.clone()).canonical_name(),
        ),
        Ty::Vector(vector) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &Ty::Vector(vector.clone()).canonical_name(),
        ),
        Ty::Span(span) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &Ty::Span(span.clone()).canonical_name(),
        ),
        Ty::ReadOnlySpan(span) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &Ty::ReadOnlySpan(span.clone()).canonical_name(),
        ),
        Ty::Fn(_) | Ty::Pointer(_) | Ty::Ref(_) | Ty::Rc(_) | Ty::Arc(_) => {
            Some((super::pointer_size(), super::pointer_align()))
        }
        Ty::TraitObject(_) => Some((super::pointer_size() * 2, super::pointer_align())),
        Ty::Tuple(tuple) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &tuple.canonical_name(),
        ),
        Ty::Nullable(inner) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            &format!("{}?", inner.canonical_name()),
        ),
        Ty::Named(name) => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            name,
        ),
        Ty::String => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            "string",
        ),
        Ty::Str => type_size_and_align_for_named_type(
            type_layouts,
            import_resolver,
            namespace,
            context_type,
            "str",
        ),
    }
}

fn split_name_segments(name: &str) -> Vec<String> {
    name.replace("::", ".")
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn strip_type_name_generic_args(name: &str) -> std::borrow::Cow<'_, str> {
    let Some(idx) = name.find('<') else {
        return std::borrow::Cow::Borrowed(name);
    };

    let prefix = name[..idx].trim_end();
    if name.ends_with('?') {
        return std::borrow::Cow::Owned(format!("{prefix}?"));
    }
    std::borrow::Cow::Borrowed(prefix)
}
