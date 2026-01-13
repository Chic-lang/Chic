use std::collections::{HashMap, VecDeque};

use crate::const_eval_config::{ConstEvalConfig, current as current_const_eval_config};
use crate::frontend::ast::Expression;

use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::ImportResolver;
use crate::mir::builder::support::resolve_type_layout_name;
use crate::mir::builder::symbol_index::{ConstSymbol, SymbolIndex};
use crate::mir::builder::{MIN_ALIGN, pointer_align, pointer_size};
use crate::mir::data::{ConstValue, Ty};
use crate::mir::layout::{TypeLayout, TypeLayoutTable};

use super::ConstEvalResult;
use super::diagnostics::ConstEvalError;

const DEFAULT_FN_CACHE_CAPACITY: usize = 256;

pub trait LocalResolver {
    fn get(&mut self, name: &str) -> Option<ConstValue>;
    fn assign(
        &mut self,
        name: &str,
        value: ConstValue,
        span: Option<Span>,
    ) -> Result<(), ConstEvalError>;
}

pub struct ImmutableLocals<'a> {
    bindings: &'a HashMap<String, ConstValue>,
}

impl<'a> ImmutableLocals<'a> {
    pub fn new(bindings: &'a HashMap<String, ConstValue>) -> Self {
        Self { bindings }
    }
}

impl LocalResolver for ImmutableLocals<'_> {
    fn get(&mut self, name: &str) -> Option<ConstValue> {
        self.bindings.get(name).cloned()
    }

    fn assign(
        &mut self,
        name: &str,
        _value: ConstValue,
        span: Option<Span>,
    ) -> Result<(), ConstEvalError> {
        Err(ConstEvalError {
            message: format!("identifier `{name}` is not assignable in a constant expression"),
            span,
        })
    }
}

struct Binding {
    value: ConstValue,
    mutable: bool,
    span: Option<Span>,
}

struct ScopeFrame {
    bindings: HashMap<String, Binding>,
}

impl ScopeFrame {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

pub struct FunctionFrame {
    scopes: Vec<ScopeFrame>,
}

impl FunctionFrame {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(ScopeFrame::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(
        &mut self,
        name: &str,
        value: ConstValue,
        mutable: bool,
        span: Option<Span>,
    ) -> Result<(), ConstEvalError> {
        let scope = self.scopes.last_mut().ok_or_else(|| ConstEvalError {
            message: format!(
                "internal error: attempted to declare `{name}` without an active scope"
            ),
            span,
        })?;
        if let Some(existing) = scope.bindings.get(name) {
            return Err(ConstEvalError {
                message: format!(
                    "variable `{name}` is already declared in this scope of compile-time function"
                ),
                span: existing.span.or(span),
            });
        }
        scope.bindings.insert(
            name.to_string(),
            Binding {
                value,
                mutable,
                span,
            },
        );
        Ok(())
    }
}

impl LocalResolver for FunctionFrame {
    fn get(&mut self, name: &str) -> Option<ConstValue> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                return Some(binding.value.clone());
            }
        }
        None
    }

    fn assign(
        &mut self,
        name: &str,
        value: ConstValue,
        span: Option<Span>,
    ) -> Result<(), ConstEvalError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.bindings.get_mut(name) {
                if !binding.mutable {
                    return Err(ConstEvalError {
                        message: format!(
                            "cannot assign to immutable variable `{name}` in compile-time function"
                        ),
                        span: binding.span.or(span),
                    });
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(ConstEvalError {
            message: format!(
                "identifier `{name}` is not declared in this scope of compile-time function"
            ),
            span,
        })
    }
}

pub struct ConstEvalContext<'a> {
    pub(crate) symbol_index: &'a mut SymbolIndex,
    pub(crate) type_layouts: &'a mut TypeLayoutTable,
    pub(crate) cache: HashMap<String, Option<ConstEvalResult>>,
    expression_cache: HashMap<ExpressionMemoKey, ConstEvalResult>,
    fn_cache: FunctionMemoCache,
    pub(crate) const_stack: Vec<String>,
    pub(crate) fn_stack: Vec<String>,
    pub(crate) errors: Vec<ConstEvalError>,
    options: ConstEvalConfig,
    fuel_remaining: Option<usize>,
    metrics: ConstEvalMetrics,
    import_resolver: Option<&'a ImportResolver>,
}

impl<'a> ConstEvalContext<'a> {
    pub fn new(
        symbol_index: &'a mut SymbolIndex,
        type_layouts: &'a mut TypeLayoutTable,
        import_resolver: Option<&'a ImportResolver>,
    ) -> Self {
        Self::with_config(
            symbol_index,
            type_layouts,
            import_resolver,
            current_const_eval_config(),
        )
    }

    pub fn with_config(
        symbol_index: &'a mut SymbolIndex,
        type_layouts: &'a mut TypeLayoutTable,
        import_resolver: Option<&'a ImportResolver>,
        options: ConstEvalConfig,
    ) -> Self {
        let fuel_remaining = options.fuel_limit;
        let mut metrics = ConstEvalMetrics::default();
        metrics.fuel_limit = options.fuel_limit;

        Self {
            symbol_index,
            type_layouts,
            cache: HashMap::new(),
            expression_cache: HashMap::new(),
            fn_cache: FunctionMemoCache::new(DEFAULT_FN_CACHE_CAPACITY),
            const_stack: Vec::new(),
            fn_stack: Vec::new(),
            errors: Vec::new(),
            options,
            fuel_remaining,
            metrics,
            import_resolver,
        }
    }

    pub(crate) fn import_resolver(&self) -> Option<&'a ImportResolver> {
        self.import_resolver
    }

    pub fn evaluate_all(mut self) -> ConstEvalSummary {
        let names: Vec<String> = self
            .symbol_index
            .constant_names()
            .cloned()
            .collect::<Vec<_>>();
        for qualified in names {
            if self.cache.contains_key(&qualified) {
                continue;
            }
            self.evaluate_const(&qualified, None);
        }
        self.metrics.cache_entries = self.expression_cache.len() + self.fn_cache.len();
        ConstEvalSummary {
            errors: self.errors,
            metrics: self.metrics,
        }
    }

    pub fn evaluate_const(
        &mut self,
        qualified: &str,
        span: Option<Span>,
    ) -> Option<ConstEvalResult> {
        if let Some(existing) = self.cache.get(qualified) {
            return existing.clone();
        }
        if self
            .const_stack
            .iter()
            .any(|item| item.as_str() == qualified)
        {
            self.errors.push(ConstEvalError {
                message: format!("cycle detected while evaluating constant `{qualified}`"),
                span,
            });
            self.cache.insert(qualified.to_string(), None);
            return None;
        }

        let symbol = match self.symbol_index.const_symbol(qualified) {
            Some(symbol) => {
                if let Some(existing) = &symbol.value {
                    let result = ConstEvalResult::new(existing.clone());
                    self.cache
                        .insert(qualified.to_string(), Some(result.clone()));
                    return Some(result);
                }
                symbol.clone()
            }
            None => {
                self.errors.push(ConstEvalError {
                    message: format!("internal error: unknown constant `{qualified}`"),
                    span,
                });
                self.cache.insert(qualified.to_string(), None);
                return None;
            }
        };

        self.const_stack.push(qualified.to_string());
        let result = self.evaluate_const_symbol(&symbol);
        self.const_stack.pop();

        match result {
            Some(value) => {
                self.symbol_index
                    .update_const_value(qualified, value.value.clone());
                self.cache
                    .insert(qualified.to_string(), Some(value.clone()));
                Some(value)
            }
            None => {
                self.cache.insert(qualified.to_string(), None);
                None
            }
        }
    }

    fn evaluate_const_symbol(&mut self, symbol: &ConstSymbol) -> Option<ConstEvalResult> {
        let ty = Ty::from_type_expr(&symbol.ty);
        let span = symbol.initializer.span.or(symbol.span);
        match self.evaluate_expression(
            &symbol.initializer,
            symbol.namespace.as_deref(),
            symbol.owner.as_deref(),
            None,
            None,
            &ty,
            span,
        ) {
            Ok(result) => Some(result),
            Err(err) => {
                self.errors.push(err.with_span_if_missing(span));
                None
            }
        }
    }

    pub fn ensure_ty_layout(&mut self, ty: &Ty) {
        match ty {
            Ty::Tuple(tuple) => {
                self.type_layouts.ensure_tuple_layout(tuple);
                for element in &tuple.elements {
                    self.ensure_ty_layout(element);
                }
            }
            Ty::Array(array) => self.ensure_ty_layout(array.element.as_ref()),
            Ty::Vec(vec) => self.ensure_ty_layout(vec.element.as_ref()),
            Ty::Nullable(inner) => {
                self.ensure_ty_layout(inner);
                self.type_layouts.ensure_nullable_layout(inner);
            }
            _ => {}
        }
    }

    pub(crate) fn should_memoise(&self) -> bool {
        self.options.enable_expression_memo
    }

    pub(crate) fn expression_key(
        &self,
        expr: &Expression,
        namespace: Option<&str>,
        owner: Option<&str>,
        target_ty: &Ty,
    ) -> ExpressionMemoKey {
        ExpressionMemoKey::new(expr, namespace, owner, target_ty)
    }

    pub(crate) fn expression_cache_lookup(
        &self,
        key: &ExpressionMemoKey,
    ) -> Option<ConstEvalResult> {
        self.expression_cache.get(key).cloned()
    }

    pub(crate) fn expression_cache_store(
        &mut self,
        key: ExpressionMemoKey,
        value: ConstEvalResult,
    ) {
        self.expression_cache.insert(key, value);
    }

    pub(crate) fn record_expression_request(&mut self) {
        self.metrics.expressions_requested += 1;
    }

    pub(crate) fn record_expression_eval(&mut self) {
        self.metrics.expressions_evaluated += 1;
    }

    pub(crate) fn record_memo_hit(&mut self) {
        self.metrics.memo_hits += 1;
    }

    pub(crate) fn record_memo_miss(&mut self) {
        self.metrics.memo_misses += 1;
    }

    pub(crate) fn record_fn_cache_hit(&mut self) {
        self.metrics.fn_cache_hits += 1;
    }

    pub(crate) fn record_fn_cache_miss(&mut self) {
        self.metrics.fn_cache_misses += 1;
    }

    pub(crate) fn const_fn_cache_lookup(&self, key: &ConstFnCacheKey) -> Option<&ConstEvalResult> {
        self.fn_cache.get(key)
    }

    pub(crate) fn const_fn_cache_store(&mut self, key: ConstFnCacheKey, value: ConstEvalResult) {
        self.fn_cache.insert(key, value);
    }

    pub(crate) fn consume_fuel(&mut self, span: Option<Span>) -> Result<(), ConstEvalError> {
        if let Some(remaining) = &mut self.fuel_remaining {
            if *remaining == 0 {
                self.metrics.fuel_exhaustions += 1;
                return Err(ConstEvalError {
                    message: format!(
                        "constant evaluation aborted: fuel limit of {} exhausted; increase via chic.cfg (consteval.fuel_limit) or the --consteval-fuel CLI flag",
                        self.options
                            .fuel_limit
                            .map_or_else(|| "unlimited".into(), |limit| limit.to_string())
                    ),
                    span,
                });
            }
            *remaining -= 1;
            self.metrics.fuel_consumed += 1;
        }
        Ok(())
    }

    pub(crate) fn size_and_align_for_ty(
        &self,
        ty: &Ty,
        namespace: Option<&str>,
    ) -> Option<(usize, usize)> {
        match ty {
            Ty::Unit => Some((0, MIN_ALIGN)),
            Ty::Unknown => None,
            Ty::Array(_)
            | Ty::Vec(_)
            | Ty::Fn(_)
            | Ty::Pointer(_)
            | Ty::Ref(_)
            | Ty::Rc(_)
            | Ty::Arc(_) => Some((pointer_size(), pointer_align())),
            Ty::TraitObject(_) => Some((pointer_size() * 2, pointer_align())),
            Ty::Span(_) | Ty::ReadOnlySpan(_) => {
                let name = ty.canonical_name();
                self.type_layouts
                    .types
                    .get(&name)
                    .and_then(layout_size_and_align)
            }
            Ty::Tuple(tuple) => {
                let name = tuple.canonical_name();
                self.type_layouts
                    .types
                    .get(&name)
                    .and_then(layout_size_and_align)
            }
            Ty::String => self.size_and_align_for_named_type("string", namespace),
            Ty::Str => self.size_and_align_for_named_type("str", namespace),
            Ty::Named(name) => self.size_and_align_for_named_type(name.as_str(), namespace),
            Ty::Vector(_) => self.type_layouts.size_and_align_for_ty(ty),
            Ty::Nullable(inner) => {
                let key = nullable_type_name(inner);
                self.type_layouts
                    .types
                    .get(&key)
                    .and_then(layout_size_and_align)
            }
        }
    }

    fn size_and_align_for_named_type(
        &self,
        name: &str,
        namespace: Option<&str>,
    ) -> Option<(usize, usize)> {
        if let Some((size, align)) = self
            .type_layouts
            .primitive_registry
            .size_align_for_name(name, pointer_size() as u32, pointer_align() as u32)
            .map(|(size, align)| (size as usize, align as usize))
        {
            return Some((size, align));
        }

        if let Some(layout) = self.type_layouts.types.get(name) {
            return layout_size_and_align(layout);
        }

        if name.contains("::") {
            return self
                .type_layouts
                .types
                .get(name)
                .and_then(layout_size_and_align);
        }

        let resolved = resolve_type_layout_name(
            self.type_layouts,
            self.import_resolver,
            namespace,
            None,
            name,
        )?;
        self.type_layouts
            .types
            .get(&resolved)
            .and_then(layout_size_and_align)
    }
}

pub struct EvalEnv<'a, 'b> {
    pub(crate) namespace: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
    pub(crate) span: Option<Span>,
    pub(crate) params: Option<&'a HashMap<String, ConstValue>>,
    pub(crate) locals: Option<&'b mut dyn LocalResolver>,
}

impl<'a, 'b> EvalEnv<'a, 'b> {
    pub fn resolve_identifier(&mut self, name: &str) -> Option<ConstValue> {
        if let Some(locals) = &mut self.locals {
            if let Some(value) = locals.get(name) {
                return Some(value);
            }
        }
        self.params.and_then(|params| params.get(name).cloned())
    }

    pub fn assign_identifier(
        &mut self,
        name: &str,
        value: ConstValue,
    ) -> Result<(), ConstEvalError> {
        if let Some(params) = self.params {
            if params.contains_key(name) {
                return Err(ConstEvalError {
                    message: format!(
                        "cannot assign to parameter `{name}` in compile-time function"
                    ),
                    span: self.span,
                });
            }
        }
        if let Some(locals) = &mut self.locals {
            return locals.assign(name, value, self.span);
        }
        Err(ConstEvalError {
            message: format!("identifier `{name}` is not assignable in this context"),
            span: self.span,
        })
    }
}

fn layout_size_and_align(layout: &TypeLayout) -> Option<(usize, usize)> {
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
        TypeLayout::Enum(data) => data.size.zip(data.align),
        TypeLayout::Union(data) => data.size.zip(data.align),
    }
}

fn nullable_type_name(inner: &Ty) -> String {
    format!("{}?", inner.canonical_name())
}

#[derive(Debug, Clone, Default)]
pub struct ConstEvalMetrics {
    pub expressions_requested: usize,
    pub expressions_evaluated: usize,
    pub memo_hits: usize,
    pub memo_misses: usize,
    pub fn_cache_hits: usize,
    pub fn_cache_misses: usize,
    pub fuel_consumed: usize,
    pub fuel_exhaustions: usize,
    pub fuel_limit: Option<usize>,
    pub cache_entries: usize,
}

#[derive(Debug, Clone)]
pub struct ConstEvalSummary {
    pub errors: Vec<ConstEvalError>,
    pub metrics: ConstEvalMetrics,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct ExpressionMemoKey {
    expr_text: String,
    namespace: Option<String>,
    owner: Option<String>,
    target_ty: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct ConstFnCacheKey {
    name: String,
    args: Vec<String>,
}

impl ConstFnCacheKey {
    pub fn new(name: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}

#[derive(Debug, Clone)]
struct FunctionMemoCache {
    entries: HashMap<ConstFnCacheKey, ConstEvalResult>,
    order: VecDeque<ConstFnCacheKey>,
    capacity: usize,
}

impl FunctionMemoCache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn get(&self, key: &ConstFnCacheKey) -> Option<&ConstEvalResult> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: ConstFnCacheKey, value: ConstEvalResult) {
        if self.entries.contains_key(&key) {
            self.entries.insert(key.clone(), value);
            self.promote(&key);
        } else {
            self.entries.insert(key.clone(), value);
            self.order.push_back(key);
            self.evict_if_needed();
        }
    }

    fn promote(&mut self, key: &ConstFnCacheKey) {
        self.order.retain(|existing| existing != key);
        self.order.push_back(key.clone());
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::data::ConstValue;

    fn result(value: ConstValue) -> ConstEvalResult {
        ConstEvalResult::new(value)
    }

    #[test]
    fn memo_cache_returns_stored_entry() {
        let mut cache = FunctionMemoCache::new(4);
        let key = ConstFnCacheKey::new("Demo::Double", vec!["i:2".into()]);
        cache.insert(key.clone(), result(ConstValue::Int(4)));
        assert_eq!(cache.len(), 1);
        let stored = cache.get(&key).expect("entry stored");
        match stored.value {
            ConstValue::Int(v) => assert_eq!(v, 4),
            _ => panic!("unexpected cached value {:?}", stored.value),
        }
    }

    #[test]
    fn memo_cache_evicts_oldest_entries_when_capacity_exceeded() {
        let mut cache = FunctionMemoCache::new(2);
        let key_a = ConstFnCacheKey::new("Demo::A", vec!["i:1".into()]);
        let key_b = ConstFnCacheKey::new("Demo::B", vec!["i:2".into()]);
        let key_c = ConstFnCacheKey::new("Demo::C", vec!["i:3".into()]);
        cache.insert(key_a.clone(), result(ConstValue::Int(1)));
        cache.insert(key_b.clone(), result(ConstValue::Int(2)));
        cache.insert(key_c.clone(), result(ConstValue::Int(3)));
        assert!(
            cache.get(&key_a).is_none(),
            "oldest entry should be evicted"
        );
        assert!(cache.get(&key_b).is_some(), "newer entry should remain");
        assert!(cache.get(&key_c).is_some(), "latest entry should remain");
    }
}

impl ExpressionMemoKey {
    fn new(
        expr: &Expression,
        namespace: Option<&str>,
        owner: Option<&str>,
        target_ty: &Ty,
    ) -> Self {
        Self {
            expr_text: expr.text.clone(),
            namespace: namespace.map(str::to_owned),
            owner: owner.map(str::to_owned),
            target_ty: target_ty.canonical_name(),
        }
    }
}
