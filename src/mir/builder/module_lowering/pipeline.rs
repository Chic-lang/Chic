use super::super::body_builder::BodyBuilder;
use super::super::const_eval::{ConstEvalContext, ConstEvalSummary};
use super::super::default_arguments::DefaultArgumentMap;
use super::super::static_registry::StaticRegistry;
use super::super::symbol_index::{ConstructorDeclSymbol, FunctionDeclSymbol, SymbolIndex};
use super::super::{Item, MirModule, Module, qualify};
use super::driver::{LoweringResult, ModuleLowering, PassStageMetric};
use crate::di::collect_di_manifest;
use crate::drop_glue::drop_type_identity;
use crate::frontend::ast::GenericParamKind;
use crate::frontend::import_resolver::ImportResolver;
use crate::frontend::parser::parse_type_expression_text;
use crate::frontend::type_alias::TypeAliasRegistry;
use crate::mir::TypeLayout;
use crate::mir::builder::{FunctionSpecialization, specialised_function_name};
use crate::mir::lower_async_functions;
use crate::mir::module_metadata::ModuleAttributes;
use crate::mir::operators::OperatorRegistry;
use crate::mir::{
    CallDispatch, ConstOperand, ConstValue, DecimalIntrinsic, GenericArg, InlineAsm,
    InlineAsmOperandKind, InterpolatedStringSegment, LocalDecl, MirBody, MirFunction,
    NumericIntrinsic, Operand, Pattern, PendingFunctionCandidate, PendingOperandInfo, Place,
    ProjectionElem, Rvalue, Statement, StatementKind, Terminator, TraitObjectDispatch, Ty,
    VariantPatternFields,
};
use crate::perf::PerfMetadata;
use crate::primitives::PrimitiveRegistry;
use crate::type_identity::type_identity_for_name;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem::take;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

pub(super) struct LoweringPipeline<'a> {
    lowering: &'a mut ModuleLowering,
    stage_metrics: Vec<PassStageMetric>,
}

impl<'a> LoweringPipeline<'a> {
    pub(super) fn new(lowering: &'a mut ModuleLowering) -> Self {
        Self {
            lowering,
            stage_metrics: Vec::new(),
        }
    }

    pub(super) fn run(mut self, module: &Module, item_units: Option<&[usize]>) -> LoweringResult {
        self.stage("prepare_context", |state| {
            state.cache.reset_run();
            state.symbol_index = SymbolIndex::build(module);
            state.import_resolver = ImportResolver::build(module);
            state.unit_import_resolvers = None;
            state.type_aliases = TypeAliasRegistry::collect(module);
            state.exports.clear();
            state.exported_symbols.clear();
            state.module_attributes = ModuleAttributes::default();
            state.test_cases.clear();
            state.operator_registry = OperatorRegistry::default();
        });

        self.stage("queue_setup", |state| {
            state.item_units = item_units.map(|units| units.to_vec());
            state.root_item_index = 0;
            state.unit_slices.clear();
        });

        self.stage("build_unit_import_resolvers", |state| {
            state.unit_import_resolvers = build_unit_import_resolvers(
                module,
                state.item_units.as_deref(),
                state.unit_packages.len(),
            );
        });

        self.stage("record_packages", |state| {
            state.record_decl_packages(&module.items, module.namespace.as_deref());
        });

        self.stage("module_attributes", |state| {
            state.collect_module_attributes(module);
            state.module_attributes.di_manifest = collect_di_manifest(module);
        });

        self.stage("collect_type_layouts", |state| {
            state.collect_type_layouts(&module.items, module.namespace.as_deref());
        });

        self.stage("backfill_type_layouts", |state| {
            state.type_layouts.backfill_missing_offsets();
        });

        self.stage("collect_traits", |state| {
            state.collect_traits(&module.items, module.namespace.as_deref());
        });

        self.stage("collect_operator_overloads", |state| {
            state.collect_operator_overloads(&module.items, module.namespace.as_deref());
        });

        self.stage("evaluate_constants", |state| {
            state.evaluate_constants();
        });

        self.stage("finalise_statics", |state| {
            state.finalise_static_members();
        });

        self.stage("prepare_default_arguments", |state| {
            state.prepare_default_arguments();
        });

        self.stage("lower_root_items", |state| {
            state.lower_root_items(&module.items, module.namespace.as_deref());
        });

        self.stage("finalise_auto_traits", |state| {
            state.type_layouts.finalize_auto_traits();
        });

        let metrics = self.lowering.cache.metrics();
        info!(
            target: "lowering::cache",
            hits = metrics.hits,
            misses = metrics.misses,
            hit_rate = metrics.hit_rate(),
            "module lowering cache summary"
        );

        self.finish()
    }

    fn stage<F>(&mut self, name: &'static str, mut action: F)
    where
        F: FnMut(&mut ModuleLowering),
    {
        let start = Instant::now();
        action(self.lowering);
        let elapsed = start.elapsed();
        self.record_stage(name, elapsed);
    }

    fn record_stage(&mut self, name: &'static str, duration: Duration) {
        if let Some(metric) = self
            .stage_metrics
            .iter_mut()
            .find(|entry| entry.name == name)
        {
            metric.count += 1;
            metric.duration_ns += duration.as_nanos();
        } else {
            self.stage_metrics.push(PassStageMetric {
                name,
                count: 1,
                duration_ns: duration.as_nanos(),
            });
        }
    }

    fn finish(self) -> LoweringResult {
        let mut result = self.lowering.finish();
        result.pass_metrics = self.stage_metrics;
        result
    }
}

fn build_unit_import_resolvers(
    module: &Module,
    item_units: Option<&[usize]>,
    unit_count_hint: usize,
) -> Option<Vec<ImportResolver>> {
    let unit_count = if unit_count_hint > 0 {
        unit_count_hint
    } else if let Some(units) = item_units {
        units.iter().copied().max().unwrap_or(0).saturating_add(1)
    } else {
        0
    };

    if unit_count <= 1 {
        return None;
    }

    let mut unit_items: Vec<Vec<Item>> = vec![Vec::new(); unit_count];
    match item_units {
        Some(units) => {
            for (idx, item) in module.items.iter().enumerate() {
                let unit = units.get(idx).copied().unwrap_or(0);
                if let Some(bucket) = unit_items.get_mut(unit) {
                    bucket.push(item.clone());
                }
            }
        }
        None => {
            unit_items[0] = module.items.clone();
        }
    }

    let mut resolvers = Vec::with_capacity(unit_items.len());
    for items in unit_items {
        let unit_module = Module::with_items(module.namespace.clone(), items);
        resolvers.push(ImportResolver::build(&unit_module));
    }
    Some(resolvers)
}

impl ModuleLowering {
    fn collect_type_layouts(&mut self, items: &[Item], namespace: Option<&str>) {
        for item in items {
            match item {
                Item::Struct(strct) => {
                    self.register_struct_layout(strct, namespace);
                    if !strct.nested_types.is_empty() {
                        let nested = qualify(namespace, &strct.name);
                        self.collect_type_layouts(&strct.nested_types, Some(&nested));
                    }
                }
                Item::Class(class) => {
                    self.register_class_layout(class, namespace);
                    if !class.nested_types.is_empty() {
                        let nested = qualify(namespace, &class.name);
                        self.collect_type_layouts(&class.nested_types, Some(&nested));
                    }
                }
                Item::Union(union_def) => self.register_union_layout(union_def, namespace),
                Item::Enum(enm) => self.register_enum_layout(enm, namespace),
                Item::Namespace(ns) => {
                    let nested = qualify(namespace, &ns.name);
                    self.collect_namespace_attributes(ns);
                    self.collect_type_layouts(&ns.items, Some(&nested));
                }
                Item::Static(static_item) => {
                    self.register_static_item(namespace, static_item);
                }
                Item::TypeAlias(alias) => self.register_type_alias(alias, namespace),
                Item::Extension(_) => {
                    // Extensions may reference types but don't introduce new layouts.
                }
                _ => {}
            }
        }
    }

    fn evaluate_constants(&mut self) {
        let context = ConstEvalContext::new(
            &mut self.symbol_index,
            &mut self.type_layouts,
            Some(&self.import_resolver),
        );
        let summary: ConstEvalSummary = context.evaluate_all();
        for error in summary.errors {
            self.diagnostics.push(super::LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }
        let metrics = summary.metrics;
        debug!(
            target: "const_eval",
            stage = "module_lowering",
            expressions_requested = metrics.expressions_requested,
            expressions_evaluated = metrics.expressions_evaluated,
            memo_hits = metrics.memo_hits,
            memo_misses = metrics.memo_misses,
            fn_cache_hits = metrics.fn_cache_hits,
            fn_cache_misses = metrics.fn_cache_misses,
            fuel_consumed = metrics.fuel_consumed,
            fuel_exhaustions = metrics.fuel_exhaustions,
            fuel_limit = metrics.fuel_limit,
            cache_entries = metrics.cache_entries,
            "const eval summary"
        );
    }

    fn prepare_default_arguments(&mut self) {
        let mut function_groups: HashMap<String, Vec<FunctionDeclSymbol>> = HashMap::new();
        for decls in self.symbol_index.function_decl_groups() {
            for decl in decls {
                function_groups
                    .entry(decl.internal_name.clone())
                    .or_default()
                    .push(decl.clone());
            }
        }
        for (internal, group) in function_groups {
            if group.is_empty() {
                continue;
            }
            self.build_function_default_arguments(internal, group);
        }

        let mut constructor_groups: HashMap<String, Vec<ConstructorDeclSymbol>> = HashMap::new();
        for decls in self.symbol_index.constructor_decl_groups() {
            for decl in decls {
                constructor_groups
                    .entry(decl.internal_name.clone())
                    .or_default()
                    .push(decl.clone());
            }
        }
        for (internal, group) in constructor_groups {
            if group.is_empty() {
                continue;
            }
            self.build_constructor_default_arguments(internal, group);
        }
    }

    fn finalise_static_members(&mut self) {
        let mut context = ConstEvalContext::new(
            &mut self.symbol_index,
            &mut self.type_layouts,
            Some(&self.import_resolver),
        );
        self.static_registry
            .finalise(&mut context, &mut self.diagnostics);
    }

    fn finish(&mut self) -> LoweringResult {
        if std::env::var("CHIC_DEBUG_INTERFACE_DISPATCH").is_ok() {
            let present = self
                .trait_decls
                .contains_key("Std::Platform::Thread::ThreadStart");
            eprintln!(
                "[trait-registry] ThreadStart present={present} total={}",
                self.trait_decls.len()
            );
        }
        self.synthesise_interface_trait_vtables();
        self.type_layouts.backfill_missing_offsets();
        self.type_layouts.finalize_type_flags();
        self.type_layouts.primitive_registry = self.primitive_registry.clone();
        let class_vtables = self.finalize_class_vtables();
        let mut module = MirModule {
            functions: take(&mut self.functions),
            test_cases: take(&mut self.test_cases),
            statics: self.static_registry.drain_vars(),
            type_layouts: take(&mut self.type_layouts),
            primitive_registry: take(&mut self.primitive_registry),
            interned_strs: self.string_interner.drain(),
            exports: take(&mut self.exports),
            attributes: take(&mut self.module_attributes),
            trait_vtables: take(&mut self.trait_vtables),
            class_vtables,
            interface_defaults: Vec::new(),
            default_arguments: take(&mut self.default_argument_records),
            type_variance: self.symbol_index.drain_type_variance(),
            async_plans: Vec::new(),
        };
        self.specialise_generic_functions(&mut module);
        let mut async_diagnostics = lower_async_functions(&mut module);
        self.diagnostics.append(&mut async_diagnostics);
        self.specialise_generic_type_layouts(&mut module);

        let result = LoweringResult {
            module,
            diagnostics: take(&mut self.diagnostics),
            constraints: take(&mut self.constraints),
            unit_slices: take(&mut self.unit_slices),
            pass_metrics: Vec::new(),
            cache_metrics: self.cache.metrics(),
            perf_metadata: take(&mut self.perf_metadata),
        };

        self.type_visibilities.clear();
        self.class_bases.clear();
        self.reset_class_vtable_state();
        self.operator_registry = OperatorRegistry::default();
        self.symbol_index = SymbolIndex::default();
        self.import_resolver = ImportResolver::default();
        self.item_units = None;
        self.root_item_index = 0;
        self.exported_symbols.clear();
        self.trait_decls.clear();
        self.static_registry = StaticRegistry::new();
        self.default_arguments = Rc::new(RefCell::new(DefaultArgumentMap::default()));
        self.test_cases.clear();
        self.perf_metadata = PerfMetadata::default();
        self.generic_specializations = Rc::new(RefCell::new(Vec::new()));
        self.extra_primitives.clear();
        self.registered_primitives.clear();
        self.extra_primitive_names.clear();
        self.primitive_registry = PrimitiveRegistry::with_builtins();

        result
    }

    fn specialise_generic_functions(&mut self, module: &mut MirModule) {
        let mut specs = self.generic_specializations.borrow().clone();
        if specs.is_empty() {
            return;
        }
        let mut existing: HashSet<String> =
            module.functions.iter().map(|f| f.name.clone()).collect();
        let mut materialized: HashSet<String> = HashSet::new();
        let mut index = 0usize;
        while index < specs.len() {
            let spec = specs[index].clone();
            index += 1;
            if existing.contains(&spec.specialized) {
                continue;
            }
            let Some(base_fn) = self.find_base_function(module, &spec.base) else {
                continue;
            };
            let canonical = spec.base.replace("__", "::");
            let mut param_names = self.method_generic_param_names(&canonical, spec.type_args.len());
            let mut owner_instantiation: Option<(String, Ty)> = None;
            if param_names.is_empty() && !spec.type_args.is_empty() {
                if let Some((owner, _)) = canonical.rsplit_once("::") {
                    let owner = owner.split('<').next().unwrap_or(owner);
                    let resolved = module
                        .type_layouts
                        .resolve_type_key(owner)
                        .unwrap_or(owner)
                        .to_string();
                    if let Some(params) = module.type_layouts.type_generic_params_for(&resolved) {
                        if params.len() == spec.type_args.len() {
                            param_names = params.to_vec();
                            let owner_ty = Ty::named_generic(
                                resolved.clone(),
                                spec.type_args
                                    .iter()
                                    .cloned()
                                    .map(GenericArg::Type)
                                    .collect(),
                            );
                            owner_instantiation = Some((resolved, owner_ty));
                        }
                    }
                }
            }
            if param_names.is_empty() {
                continue;
            }
            let map: HashMap<String, Ty> = param_names
                .into_iter()
                .zip(spec.type_args.clone())
                .collect();
            let mut clone = self.substitute_function_types(&base_fn, &map);
            if let Some((owner_name, owner_ty)) = owner_instantiation.as_ref() {
                if let Some(first) = clone.signature.params.first() {
                    let first_name = first.canonical_name();
                    let first_base = first_name.split('<').next().unwrap_or(first_name.as_str());
                    let owner_base = owner_name.split('<').next().unwrap_or(owner_name.as_str());
                    if first_base == owner_base {
                        if let Some(slot) = clone.signature.params.first_mut() {
                            *slot = owner_ty.clone();
                        }
                        for local in &mut clone.body.locals {
                            if matches!(local.kind, crate::mir::LocalKind::Arg(0)) {
                                local.ty = owner_ty.clone();
                                local.is_nullable = matches!(local.ty, Ty::Nullable(_));
                            }
                        }
                    }
                }
            }
            let new_specs = self.substitute_body_generics(&mut clone.body, module, &map);
            self.substitute_type_id_constants(&mut clone.body, module, &map);
            for new_spec in new_specs {
                if !specs.iter().any(|entry| {
                    entry.specialized == new_spec.specialized
                        || entry.base == new_spec.base && entry.type_args == new_spec.type_args
                }) {
                    specs.push(new_spec);
                }
            }
            clone.name = spec.specialized.clone();
            existing.insert(clone.name.clone());
            module.functions.push(clone);
            materialized.insert(spec.base.clone());
        }

        fn collect_fn_tys(ty: &Ty, out: &mut Vec<crate::mir::FnTy>) {
            match ty {
                Ty::Fn(fn_ty) => {
                    out.push(fn_ty.clone());
                    for param in &fn_ty.params {
                        collect_fn_tys(param, out);
                    }
                    collect_fn_tys(&fn_ty.ret, out);
                }
                Ty::Named(named) => {
                    for arg in named.args() {
                        if let Some(inner) = arg.as_type() {
                            collect_fn_tys(inner, out);
                        }
                    }
                }
                Ty::Array(array) => collect_fn_tys(&array.element, out),
                Ty::Vec(vec) => collect_fn_tys(&vec.element, out),
                Ty::Span(span) => collect_fn_tys(&span.element, out),
                Ty::ReadOnlySpan(span) => collect_fn_tys(&span.element, out),
                Ty::Rc(rc) => collect_fn_tys(&rc.element, out),
                Ty::Arc(arc) => collect_fn_tys(&arc.element, out),
                Ty::Tuple(tuple) => {
                    for element in &tuple.elements {
                        collect_fn_tys(element, out);
                    }
                }
                Ty::Pointer(pointer) => collect_fn_tys(&pointer.element, out),
                Ty::Ref(reference) => collect_fn_tys(&reference.element, out),
                Ty::Nullable(inner) => collect_fn_tys(inner, out),
                Ty::Vector(vector) => collect_fn_tys(&vector.element, out),
                Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => {}
            }
        }

        let mut fn_tys = Vec::new();
        for function in &module.functions {
            for param in &function.signature.params {
                collect_fn_tys(param, &mut fn_tys);
            }
            collect_fn_tys(&function.signature.ret, &mut fn_tys);
            for effect in &function.body.effects {
                collect_fn_tys(effect, &mut fn_tys);
            }
            if let Some(async_ret) = function.async_result.as_ref() {
                collect_fn_tys(async_ret, &mut fn_tys);
            }
            for local in &function.body.locals {
                collect_fn_tys(&local.ty, &mut fn_tys);
            }
        }
        for fn_ty in fn_tys {
            module.type_layouts.ensure_fn_layout(&fn_ty);
        }
    }

    fn specialise_generic_type_layouts(&mut self, module: &mut MirModule) {
        use crate::mir::GenericArg;
        use crate::mir::TypeLayout;

        let mut queue: Vec<Ty> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        fn is_unbound_generic_name(name: &str) -> bool {
            if name.contains("::") {
                return false;
            }
            let bytes = name.as_bytes();
            if bytes.len() == 1 && bytes[0].is_ascii_uppercase() {
                return true;
            }
            if name.starts_with('T')
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return true;
            }
            false
        }

        fn contains_unbound_generic(ty: &Ty) -> bool {
            match ty {
                Ty::Named(named) => {
                    if named.args().is_empty() {
                        return is_unbound_generic_name(named.as_str());
                    }
                    named.args().iter().any(|arg| match arg {
                        GenericArg::Type(inner) => contains_unbound_generic(inner),
                        _ => false,
                    })
                }
                Ty::Array(array) => contains_unbound_generic(&array.element),
                Ty::Vec(vec) => contains_unbound_generic(&vec.element),
                Ty::Span(span) => contains_unbound_generic(&span.element),
                Ty::ReadOnlySpan(span) => contains_unbound_generic(&span.element),
                Ty::Rc(rc) => contains_unbound_generic(&rc.element),
                Ty::Arc(arc) => contains_unbound_generic(&arc.element),
                Ty::Tuple(tuple) => tuple.elements.iter().any(contains_unbound_generic),
                Ty::Fn(fn_ty) => {
                    fn_ty.params.iter().any(contains_unbound_generic)
                        || contains_unbound_generic(&fn_ty.ret)
                }
                Ty::Pointer(pointer) => contains_unbound_generic(&pointer.element),
                Ty::Ref(reference) => contains_unbound_generic(&reference.element),
                Ty::Nullable(inner) => contains_unbound_generic(inner),
                Ty::Vector(vector) => contains_unbound_generic(&vector.element),
                Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => false,
            }
        }

        for function in &module.functions {
            for param in &function.signature.params {
                queue.push(param.clone());
            }
            queue.push(function.signature.ret.clone());
            for effect in &function.body.effects {
                queue.push(effect.clone());
            }
            if let Some(async_ret) = function.async_result.as_ref() {
                queue.push(async_ret.clone());
            }
            for local in &function.body.locals {
                queue.push(local.ty.clone());
            }
        }

        while let Some(ty) = queue.pop() {
            match &ty {
                Ty::Named(named) => {
                    for arg in named.args() {
                        if let GenericArg::Type(inner) = arg {
                            queue.push(inner.clone());
                        }
                    }
                }
                Ty::Array(array) => queue.push((*array.element).clone()),
                Ty::Vec(vec) => queue.push((*vec.element).clone()),
                Ty::Span(span) => queue.push((*span.element).clone()),
                Ty::ReadOnlySpan(span) => queue.push((*span.element).clone()),
                Ty::Rc(rc) => queue.push((*rc.element).clone()),
                Ty::Arc(arc) => queue.push((*arc.element).clone()),
                Ty::Tuple(tuple) => {
                    for element in &tuple.elements {
                        queue.push(element.clone());
                    }
                }
                Ty::Fn(fn_ty) => {
                    for param in &fn_ty.params {
                        queue.push(param.clone());
                    }
                    queue.push((*fn_ty.ret).clone());
                }
                Ty::Pointer(pointer) => queue.push(pointer.element.clone()),
                Ty::Ref(reference) => queue.push(reference.element.clone()),
                Ty::Nullable(inner) => queue.push((**inner).clone()),
                Ty::Vector(vector) => queue.push((*vector.element).clone()),
                Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => {}
            }

            let Ty::Named(named) = &ty else { continue };
            if named.args().is_empty() {
                continue;
            }
            let key = ty.canonical_name().replace('.', "::");
            if !seen.insert(key.clone()) {
                continue;
            }
            if module.type_layouts.types.contains_key(&key) {
                continue;
            }
            let base = key.split('<').next().unwrap_or(key.as_str()).to_string();
            let Some(param_names) = module.type_layouts.type_generic_params_for(&base) else {
                continue;
            };
            let type_args = named
                .args()
                .iter()
                .filter_map(|arg| match arg {
                    GenericArg::Type(inner) => Some(inner.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if type_args.len() != param_names.len() {
                continue;
            }
            if type_args.iter().any(|ty| contains_unbound_generic(ty)) {
                continue;
            }

            let Some(base_key) = module
                .type_layouts
                .resolve_type_key(&base)
                .map(str::to_string)
            else {
                continue;
            };
            let Some(base_layout) = module.type_layouts.types.get(&base_key).cloned() else {
                continue;
            };

            let TypeLayout::Struct(struct_layout) = base_layout else {
                continue;
            };

            let map: HashMap<String, Ty> = param_names
                .iter()
                .cloned()
                .zip(type_args.iter().cloned())
                .collect();
            let mut specialised = struct_layout.clone();
            specialised.name = key.clone();
            if let Some(dispose) = specialised.dispose.clone() {
                specialised.dispose = Some(specialised_function_name(&dispose, &type_args));
            }
            specialised.fields = specialised
                .fields
                .into_iter()
                .map(|mut field| {
                    field.ty = BodyBuilder::substitute_generics(&field.ty, &map);
                    field.is_nullable = matches!(field.ty, Ty::Nullable(_));
                    field.offset = None;
                    field
                })
                .collect();
            specialised.size = None;
            specialised.align = None;

            let base_flags = module.type_layouts.type_flags_for_name(base_key.as_str());
            module.type_layouts.add_type_flags(key.clone(), base_flags);
            module
                .type_layouts
                .types
                .insert(key.clone(), TypeLayout::Struct(specialised));

            if let Some(TypeLayout::Struct(inserted)) = module.type_layouts.types.get(&key) {
                for field in &inserted.fields {
                    queue.push(field.ty.clone());
                }
            }
        }

        module.type_layouts.backfill_missing_offsets();
    }

    fn find_base_function(&self, module: &MirModule, base: &str) -> Option<MirFunction> {
        let candidates = [
            base.to_string(),
            base.replace("::", "__"),
            base.replace("__", "::"),
        ];
        for name in candidates {
            if let Some(found) = module.functions.iter().find(|func| func.name == name) {
                return Some(found.clone());
            }
        }
        None
    }

    fn substitute_function_types(
        &self,
        func: &MirFunction,
        map: &HashMap<String, Ty>,
    ) -> MirFunction {
        let mut clone = func.clone();
        clone.signature.params = clone
            .signature
            .params
            .iter()
            .map(|ty| BodyBuilder::substitute_generics(ty, map))
            .collect();
        clone.signature.ret = BodyBuilder::substitute_generics(&clone.signature.ret, map);
        clone.body.effects = clone
            .body
            .effects
            .iter()
            .map(|ty| BodyBuilder::substitute_generics(ty, map))
            .collect();
        if let Some(async_ret) = clone.async_result.as_mut() {
            *async_ret = BodyBuilder::substitute_generics(async_ret, map);
        }
        for local in &mut clone.body.locals {
            local.ty = BodyBuilder::substitute_generics(&local.ty, map);
            local.is_nullable = matches!(local.ty, Ty::Nullable(_));
        }
        self.substitute_body_value_generics(&mut clone.body, map);
        clone
    }

    fn substitute_body_value_generics(&self, body: &mut MirBody, map: &HashMap<String, Ty>) {
        if map.is_empty() {
            return;
        }
        for block in &mut body.blocks {
            for stmt in &mut block.statements {
                self.substitute_statement_generics(stmt, map);
            }
            if let Some(term) = block.terminator.as_mut() {
                self.substitute_terminator_generics(term, map);
            }
        }
    }

    fn substitute_statement_generics(&self, stmt: &mut Statement, map: &HashMap<String, Ty>) {
        match &mut stmt.kind {
            StatementKind::Assign { value, .. } => self.substitute_rvalue_generics(value, map),
            StatementKind::ZeroInitRaw { pointer, length } => {
                self.substitute_operand_generics(pointer, map);
                self.substitute_operand_generics(length, map);
            }
            StatementKind::AtomicStore { value, .. } => {
                self.substitute_operand_generics(value, map)
            }
            StatementKind::MmioStore { target, value } => {
                target.ty = BodyBuilder::substitute_generics(&target.ty, map);
                self.substitute_operand_generics(value, map);
            }
            StatementKind::Assert { cond, .. } => self.substitute_operand_generics(cond, map),
            StatementKind::EnqueueKernel { kernel, args, .. } => {
                self.substitute_operand_generics(kernel, map);
                for arg in args {
                    self.substitute_operand_generics(arg, map);
                }
            }
            StatementKind::EnqueueCopy { bytes, .. } => {
                self.substitute_operand_generics(bytes, map)
            }
            StatementKind::Eval(_) => {}
            StatementKind::StaticStore { value, .. } => {
                self.substitute_operand_generics(value, map)
            }
            StatementKind::InlineAsm(asm) => self.substitute_inline_asm_generics(asm, map),
            StatementKind::Drop { .. }
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::MarkFallibleHandled { .. }
            | StatementKind::Deinit(_)
            | StatementKind::Borrow { .. }
            | StatementKind::Retag { .. }
            | StatementKind::DeferDrop { .. }
            | StatementKind::DefaultInit { .. }
            | StatementKind::ZeroInit { .. }
            | StatementKind::AtomicFence { .. }
            | StatementKind::EnterUnsafe
            | StatementKind::ExitUnsafe
            | StatementKind::RecordEvent { .. }
            | StatementKind::WaitEvent { .. }
            | StatementKind::Nop
            | StatementKind::Pending(_) => {}
        }
    }

    fn substitute_inline_asm_generics(&self, asm: &mut InlineAsm, map: &HashMap<String, Ty>) {
        for operand in &mut asm.operands {
            match &mut operand.kind {
                InlineAsmOperandKind::In { value } | InlineAsmOperandKind::Const { value } => {
                    self.substitute_operand_generics(value, map);
                }
                InlineAsmOperandKind::InOut { input, .. } => {
                    self.substitute_operand_generics(input, map);
                }
                InlineAsmOperandKind::Out { .. } | InlineAsmOperandKind::Sym { .. } => {}
            }
        }
    }

    fn substitute_terminator_generics(&self, term: &mut Terminator, map: &HashMap<String, Ty>) {
        match term {
            Terminator::SwitchInt { discr, .. } => self.substitute_operand_generics(discr, map),
            Terminator::Call { func, args, .. } => {
                self.substitute_operand_generics(func, map);
                for arg in args {
                    self.substitute_operand_generics(arg, map);
                }
            }
            Terminator::Yield { value, .. } => self.substitute_operand_generics(value, map),
            Terminator::Throw { exception, ty } => {
                if let Some(exception) = exception.as_mut() {
                    self.substitute_operand_generics(exception, map);
                }
                if let Some(ty) = ty.as_mut() {
                    *ty = BodyBuilder::substitute_generics(ty, map);
                }
            }
            Terminator::Match { arms, .. } => {
                for arm in arms {
                    self.substitute_pattern_generics(&mut arm.pattern, map);
                }
            }
            Terminator::Goto { .. }
            | Terminator::Return
            | Terminator::Await { .. }
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_) => {}
        }
    }

    fn substitute_pattern_generics(&self, pattern: &mut Pattern, map: &HashMap<String, Ty>) {
        match pattern {
            Pattern::Type(ty) => {
                *ty = BodyBuilder::substitute_generics(ty, map);
            }
            Pattern::Tuple(items) => {
                for item in items {
                    self.substitute_pattern_generics(item, map);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    self.substitute_pattern_generics(&mut field.pattern, map);
                }
            }
            Pattern::Enum { fields, .. } => match fields {
                VariantPatternFields::Unit => {}
                VariantPatternFields::Tuple(items) => {
                    for item in items {
                        self.substitute_pattern_generics(item, map);
                    }
                }
                VariantPatternFields::Struct(items) => {
                    for field in items {
                        self.substitute_pattern_generics(&mut field.pattern, map);
                    }
                }
            },
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Binding(_) => {}
        }
    }

    fn substitute_rvalue_generics(&self, value: &mut Rvalue, map: &HashMap<String, Ty>) {
        match value {
            Rvalue::Use(operand) => self.substitute_operand_generics(operand, map),
            Rvalue::Unary { operand, .. } => self.substitute_operand_generics(operand, map),
            Rvalue::Binary { lhs, rhs, .. } => {
                self.substitute_operand_generics(lhs, map);
                self.substitute_operand_generics(rhs, map);
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    self.substitute_operand_generics(field, map);
                }
            }
            Rvalue::SpanStackAlloc {
                element,
                length,
                source,
            } => {
                *element = BodyBuilder::substitute_generics(element, map);
                self.substitute_operand_generics(length, map);
                if let Some(source) = source.as_mut() {
                    self.substitute_operand_generics(source, map);
                }
            }
            Rvalue::Cast {
                operand,
                source,
                target,
                ..
            } => {
                self.substitute_operand_generics(operand, map);
                *source = BodyBuilder::substitute_generics(source, map);
                *target = BodyBuilder::substitute_generics(target, map);
            }
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        self.substitute_operand_generics(operand, map);
                    }
                }
            }
            Rvalue::NumericIntrinsic(NumericIntrinsic { operands, .. }) => {
                for operand in operands {
                    self.substitute_operand_generics(operand, map);
                }
            }
            Rvalue::DecimalIntrinsic(DecimalIntrinsic {
                lhs,
                rhs,
                addend,
                rounding,
                vectorize,
                ..
            }) => {
                self.substitute_operand_generics(lhs, map);
                self.substitute_operand_generics(rhs, map);
                if let Some(addend) = addend.as_mut() {
                    self.substitute_operand_generics(addend, map);
                }
                self.substitute_operand_generics(rounding, map);
                self.substitute_operand_generics(vectorize, map);
            }
            Rvalue::AtomicRmw { value, .. } => self.substitute_operand_generics(value, map),
            Rvalue::AtomicCompareExchange {
                expected, desired, ..
            } => {
                self.substitute_operand_generics(expected, map);
                self.substitute_operand_generics(desired, map);
            }
            Rvalue::AddressOf { .. }
            | Rvalue::Len(_)
            | Rvalue::AtomicLoad { .. }
            | Rvalue::Pending(_)
            | Rvalue::StaticLoad { .. }
            | Rvalue::StaticRef { .. } => {}
        }
    }

    fn substitute_operand_generics(&self, operand: &mut Operand, map: &HashMap<String, Ty>) {
        match operand {
            Operand::Mmio(mmio) => {
                mmio.ty = BodyBuilder::substitute_generics(&mmio.ty, map);
            }
            Operand::Pending(pending) => {
                if let Some(info) = pending.info.as_mut() {
                    match info.as_mut() {
                        PendingOperandInfo::FunctionGroup {
                            candidates,
                            receiver,
                            ..
                        } => {
                            if let Some(receiver) = receiver.as_mut() {
                                self.substitute_operand_generics(receiver, map);
                            }
                            self.substitute_pending_candidates_generics(candidates, map);
                        }
                    }
                }
            }
            Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_) | Operand::Const(_) => {}
        }
    }

    fn substitute_pending_candidates_generics(
        &self,
        candidates: &mut [PendingFunctionCandidate],
        map: &HashMap<String, Ty>,
    ) {
        for candidate in candidates {
            candidate.signature.params = candidate
                .signature
                .params
                .iter()
                .map(|ty| BodyBuilder::substitute_generics(ty, map))
                .collect();
            candidate.signature.ret = Box::new(BodyBuilder::substitute_generics(
                &candidate.signature.ret,
                map,
            ));
        }
    }

    fn substitute_type_id_constants(
        &self,
        body: &mut MirBody,
        module: &MirModule,
        map: &HashMap<String, Ty>,
    ) {
        if map.is_empty() {
            return;
        }
        let mut id_map: HashMap<u128, u128> = HashMap::new();
        for (name, replacement) in map {
            let from = u128::from(drop_type_identity(name));
            let to = u128::from(type_identity_for_name(
                &module.type_layouts,
                &replacement.canonical_name(),
            ));
            if from != to {
                id_map.insert(from, to);
            }
        }
        if id_map.is_empty() {
            return;
        }

        for block in &mut body.blocks {
            for stmt in &mut block.statements {
                Self::rewrite_statement_type_ids(stmt, &id_map);
            }
            if let Some(term) = block.terminator.as_mut() {
                Self::rewrite_terminator_type_ids(term, &id_map);
            }
        }
    }

    fn rewrite_statement_type_ids(stmt: &mut Statement, id_map: &HashMap<u128, u128>) {
        match &mut stmt.kind {
            StatementKind::Assign { value, .. } => Self::rewrite_rvalue_type_ids(value, id_map),
            StatementKind::ZeroInitRaw { pointer, length } => {
                Self::rewrite_operand_type_ids(pointer, id_map);
                Self::rewrite_operand_type_ids(length, id_map);
            }
            StatementKind::AtomicStore { value, .. } => {
                Self::rewrite_operand_type_ids(value, id_map);
            }
            StatementKind::MmioStore { value, .. } => {
                Self::rewrite_operand_type_ids(value, id_map);
            }
            StatementKind::Assert { cond, .. } => {
                Self::rewrite_operand_type_ids(cond, id_map);
            }
            StatementKind::EnqueueKernel { kernel, args, .. } => {
                Self::rewrite_operand_type_ids(kernel, id_map);
                for arg in args {
                    Self::rewrite_operand_type_ids(arg, id_map);
                }
            }
            StatementKind::EnqueueCopy { bytes, .. } => {
                Self::rewrite_operand_type_ids(bytes, id_map);
            }
            StatementKind::StaticStore { value, .. } => {
                Self::rewrite_operand_type_ids(value, id_map);
            }
            StatementKind::InlineAsm(asm) => Self::rewrite_inline_asm_type_ids(asm, id_map),
            StatementKind::Drop { .. }
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::MarkFallibleHandled { .. }
            | StatementKind::Deinit(_)
            | StatementKind::Borrow { .. }
            | StatementKind::Retag { .. }
            | StatementKind::DeferDrop { .. }
            | StatementKind::DefaultInit { .. }
            | StatementKind::ZeroInit { .. }
            | StatementKind::AtomicFence { .. }
            | StatementKind::EnterUnsafe
            | StatementKind::ExitUnsafe
            | StatementKind::RecordEvent { .. }
            | StatementKind::WaitEvent { .. }
            | StatementKind::Eval(_)
            | StatementKind::Nop
            | StatementKind::Pending(_) => {}
        }
    }

    fn rewrite_inline_asm_type_ids(asm: &mut InlineAsm, id_map: &HashMap<u128, u128>) {
        for operand in &mut asm.operands {
            match &mut operand.kind {
                InlineAsmOperandKind::In { value } | InlineAsmOperandKind::Const { value } => {
                    Self::rewrite_operand_type_ids(value, id_map);
                }
                InlineAsmOperandKind::InOut { input, .. } => {
                    Self::rewrite_operand_type_ids(input, id_map);
                }
                InlineAsmOperandKind::Out { .. } | InlineAsmOperandKind::Sym { .. } => {}
            }
        }
    }

    fn rewrite_terminator_type_ids(term: &mut Terminator, id_map: &HashMap<u128, u128>) {
        match term {
            Terminator::SwitchInt { discr, .. } => Self::rewrite_operand_type_ids(discr, id_map),
            Terminator::Call { func, args, .. } => {
                Self::rewrite_operand_type_ids(func, id_map);
                for arg in args {
                    Self::rewrite_operand_type_ids(arg, id_map);
                }
            }
            Terminator::Yield { value, .. } => Self::rewrite_operand_type_ids(value, id_map),
            Terminator::Throw { exception, .. } => {
                if let Some(exception) = exception.as_mut() {
                    Self::rewrite_operand_type_ids(exception, id_map);
                }
            }
            Terminator::Goto { .. }
            | Terminator::Match { .. }
            | Terminator::Return
            | Terminator::Await { .. }
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_) => {}
        }
    }

    fn rewrite_rvalue_type_ids(value: &mut Rvalue, id_map: &HashMap<u128, u128>) {
        match value {
            Rvalue::Use(operand) => Self::rewrite_operand_type_ids(operand, id_map),
            Rvalue::Unary { operand, .. } => Self::rewrite_operand_type_ids(operand, id_map),
            Rvalue::Binary { lhs, rhs, .. } => {
                Self::rewrite_operand_type_ids(lhs, id_map);
                Self::rewrite_operand_type_ids(rhs, id_map);
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    Self::rewrite_operand_type_ids(field, id_map);
                }
            }
            Rvalue::SpanStackAlloc { length, source, .. } => {
                Self::rewrite_operand_type_ids(length, id_map);
                if let Some(source) = source.as_mut() {
                    Self::rewrite_operand_type_ids(source, id_map);
                }
            }
            Rvalue::Cast { operand, .. } => Self::rewrite_operand_type_ids(operand, id_map),
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        Self::rewrite_operand_type_ids(operand, id_map);
                    }
                }
            }
            Rvalue::NumericIntrinsic(intrinsic) => {
                Self::rewrite_numeric_intrinsic_type_ids(intrinsic, id_map);
            }
            Rvalue::DecimalIntrinsic(intrinsic) => {
                Self::rewrite_decimal_intrinsic_type_ids(intrinsic, id_map);
            }
            Rvalue::AtomicRmw { value, .. } => Self::rewrite_operand_type_ids(value, id_map),
            Rvalue::AtomicCompareExchange {
                expected, desired, ..
            } => {
                Self::rewrite_operand_type_ids(expected, id_map);
                Self::rewrite_operand_type_ids(desired, id_map);
            }
            Rvalue::AddressOf { .. }
            | Rvalue::Len(_)
            | Rvalue::AtomicLoad { .. }
            | Rvalue::Pending(_)
            | Rvalue::StaticLoad { .. }
            | Rvalue::StaticRef { .. } => {}
        }
    }

    fn rewrite_numeric_intrinsic_type_ids(
        intrinsic: &mut NumericIntrinsic,
        id_map: &HashMap<u128, u128>,
    ) {
        for operand in &mut intrinsic.operands {
            Self::rewrite_operand_type_ids(operand, id_map);
        }
    }

    fn rewrite_decimal_intrinsic_type_ids(
        intrinsic: &mut DecimalIntrinsic,
        id_map: &HashMap<u128, u128>,
    ) {
        Self::rewrite_operand_type_ids(&mut intrinsic.lhs, id_map);
        Self::rewrite_operand_type_ids(&mut intrinsic.rhs, id_map);
        if let Some(addend) = intrinsic.addend.as_mut() {
            Self::rewrite_operand_type_ids(addend, id_map);
        }
        Self::rewrite_operand_type_ids(&mut intrinsic.rounding, id_map);
        Self::rewrite_operand_type_ids(&mut intrinsic.vectorize, id_map);
    }

    fn rewrite_operand_type_ids(operand: &mut Operand, id_map: &HashMap<u128, u128>) {
        match operand {
            Operand::Const(constant) => Self::rewrite_const_operand_type_ids(constant, id_map),
            Operand::Pending(pending) => {
                if let Some(info) = pending.info.as_mut() {
                    let crate::mir::PendingOperandInfo::FunctionGroup { receiver, .. } =
                        info.as_mut();
                    if let Some(receiver) = receiver.as_mut() {
                        Self::rewrite_operand_type_ids(receiver, id_map);
                    }
                }
            }
            Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_) | Operand::Mmio(_) => {}
        }
    }

    fn rewrite_const_operand_type_ids(constant: &mut ConstOperand, id_map: &HashMap<u128, u128>) {
        match &mut constant.value {
            ConstValue::UInt(value) => {
                if let Some(replacement) = id_map.get(value).copied() {
                    *value = replacement;
                }
            }
            ConstValue::Int(value) => {
                let Ok(signed) = i64::try_from(*value) else {
                    return;
                };
                let bits = u128::from(signed as u64);
                let Some(replacement) = id_map.get(&bits).copied() else {
                    return;
                };
                let Ok(replacement) = u64::try_from(replacement) else {
                    return;
                };
                *value = i128::from(replacement as i64);
            }
            ConstValue::Struct { fields, .. } => {
                for (_, field) in fields {
                    Self::rewrite_const_value_type_ids(field, id_map);
                }
            }
            ConstValue::Enum { .. }
            | ConstValue::Int32(_)
            | ConstValue::Float(_)
            | ConstValue::Decimal(_)
            | ConstValue::Bool(_)
            | ConstValue::Char(_)
            | ConstValue::Str { .. }
            | ConstValue::RawStr(_)
            | ConstValue::Symbol(_)
            | ConstValue::Null
            | ConstValue::Unit
            | ConstValue::Unknown => {}
        }
    }

    fn rewrite_const_value_type_ids(value: &mut ConstValue, id_map: &HashMap<u128, u128>) {
        match value {
            ConstValue::UInt(v) => {
                if let Some(replacement) = id_map.get(v).copied() {
                    *v = replacement;
                }
            }
            ConstValue::Int(v) => {
                let Ok(signed) = i64::try_from(*v) else {
                    return;
                };
                let bits = u128::from(signed as u64);
                let Some(replacement) = id_map.get(&bits).copied() else {
                    return;
                };
                let Ok(replacement) = u64::try_from(replacement) else {
                    return;
                };
                *v = i128::from(replacement as i64);
            }
            ConstValue::Struct { fields, .. } => {
                for (_, field) in fields {
                    Self::rewrite_const_value_type_ids(field, id_map);
                }
            }
            ConstValue::Enum { .. }
            | ConstValue::Int32(_)
            | ConstValue::Float(_)
            | ConstValue::Decimal(_)
            | ConstValue::Bool(_)
            | ConstValue::Char(_)
            | ConstValue::Str { .. }
            | ConstValue::RawStr(_)
            | ConstValue::Symbol(_)
            | ConstValue::Null
            | ConstValue::Unit
            | ConstValue::Unknown => {}
        }
    }

    fn substitute_body_generics(
        &self,
        body: &mut MirBody,
        module: &MirModule,
        map: &HashMap<String, Ty>,
    ) -> Vec<FunctionSpecialization> {
        let mut new_specs = Vec::new();
        let locals = body.locals.clone();
        for block in &mut body.blocks {
            if let Some(Terminator::Call {
                func,
                args,
                destination,
                dispatch,
                ..
            }) = block.terminator.as_mut()
            {
                if let Some(CallDispatch::Trait(TraitObjectDispatch { impl_type, .. })) =
                    dispatch.as_mut()
                {
                    if let Some(name) = impl_type.as_mut() {
                        if let Some(replacement) = map.get(name) {
                            *name = replacement.canonical_name();
                        }
                    }
                }

                let mut inferred_return_ty: Option<Ty> = None;
                if let Operand::Const(constant) = func {
                    if let ConstValue::Symbol(symbol) = &constant.value {
                        if let Some(callee) = self.find_base_function(module, symbol) {
                            inferred_return_ty = Some(callee.signature.ret);
                        }
                    }
                }

                if inferred_return_ty.is_none() {
                    if let Some(CallDispatch::Trait(trait_dispatch)) = dispatch.as_ref() {
                        let trait_label = trait_dispatch
                            .trait_name
                            .rsplit("::")
                            .next()
                            .unwrap_or(trait_dispatch.trait_name.as_str());
                        if let Some(impl_type) = trait_dispatch.impl_type.as_deref() {
                            let candidates = [
                                format!("{impl_type}::{trait_label}::{}", trait_dispatch.method),
                                format!("{impl_type}::{}", trait_dispatch.method),
                            ];
                            for candidate in candidates {
                                if let Some(callee) = self.find_base_function(module, &candidate) {
                                    inferred_return_ty = Some(callee.signature.ret);
                                    break;
                                }
                            }
                            if inferred_return_ty.is_none() {
                                let impl_segment = impl_type
                                    .rsplit("::")
                                    .next()
                                    .unwrap_or(impl_type)
                                    .split('<')
                                    .next()
                                    .unwrap_or(impl_type);
                                for callee in &module.functions {
                                    if callee.name.ends_with(&format!(
                                        "::{impl_segment}::{trait_label}::{}",
                                        trait_dispatch.method
                                    )) || callee.name.ends_with(&format!(
                                        "::{impl_segment}::{}",
                                        trait_dispatch.method
                                    )) {
                                        inferred_return_ty = Some(callee.signature.ret.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                if let (Some(ret_ty), Some(dest_place)) =
                    (inferred_return_ty.as_ref(), destination.as_ref())
                {
                    if dest_place.projection.is_empty() {
                        if let Some(local) = body.locals.get_mut(dest_place.local.0) {
                            if matches!(local.ty, Ty::Unknown) {
                                local.ty = ret_ty.clone();
                            }
                        }
                    }
                }
                let Operand::Const(constant) = func else {
                    continue;
                };
                let ConstValue::Symbol(symbol) = &mut constant.value else {
                    continue;
                };

                if !map.is_empty() {
                    if let Some(rewritten) = Self::substitute_specialised_symbol_args(symbol, map) {
                        if rewritten != *symbol {
                            *symbol = rewritten;
                        }
                    }
                }

                if let Some((base_name, type_args)) = Self::split_specialised_symbol(symbol) {
                    if !type_args.is_empty()
                        && type_args
                            .iter()
                            .all(|ty| !Self::ty_contains_unbound_generic(ty))
                    {
                        new_specs.push(FunctionSpecialization {
                            base: base_name,
                            specialized: symbol.clone(),
                            type_args,
                        });
                    }
                }

                let base = symbol.split('<').next().unwrap_or(symbol).to_string();
                let canonical = base.replace("__", "::");
                if !map.is_empty() && !symbol.contains('<') {
                    if let Some((owner, _)) = canonical.rsplit_once("::") {
                        let owner = owner.split('<').next().unwrap_or(owner);
                        let resolved_owner =
                            module.type_layouts.resolve_type_key(owner).unwrap_or(owner);
                        if let Some(owner_params) =
                            module.type_layouts.type_generic_params_for(resolved_owner)
                        {
                            if !owner_params.is_empty()
                                && owner_params.iter().all(|name| map.contains_key(name))
                            {
                                let type_args = owner_params
                                    .iter()
                                    .filter_map(|name| map.get(name).cloned())
                                    .collect::<Vec<_>>();
                                if !type_args.is_empty()
                                    && type_args
                                        .iter()
                                        .all(|ty| !Self::ty_contains_unbound_generic(ty))
                                {
                                    let specialised = specialised_function_name(&base, &type_args);
                                    if &specialised != symbol {
                                        *symbol = specialised.clone();
                                        new_specs.push(FunctionSpecialization {
                                            base: base.clone(),
                                            specialized: specialised,
                                            type_args,
                                        });
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                if args.len() > 0 {
                    if let Some((owner, _)) = canonical.rsplit_once("::") {
                        let owner = owner.split('<').next().unwrap_or(owner);
                        if let Some(param_names) =
                            module.type_layouts.type_generic_params_for(owner)
                        {
                            if !param_names.is_empty() {
                                let generic_names: HashSet<String> =
                                    param_names.iter().cloned().collect();
                                if let Some(mut receiver_ty) = Self::operand_ty(&locals, &args[0]) {
                                    if let Ty::Nullable(inner) = receiver_ty {
                                        receiver_ty = *inner;
                                    }
                                    if let Ty::Ref(reference) = receiver_ty {
                                        receiver_ty = reference.element;
                                    }
                                    if let Ty::Named(named) = receiver_ty {
                                        let owner_base = owner.rsplit("::").next().unwrap_or(owner);
                                        let recv_base =
                                            named.name.rsplit("::").next().unwrap_or(&named.name);
                                        if owner_base == recv_base {
                                            let type_args = named
                                                .args()
                                                .iter()
                                                .filter_map(|arg| arg.as_type().cloned())
                                                .collect::<Vec<_>>();
                                            if type_args.len() == param_names.len()
                                                && !type_args.iter().any(|ty| {
                                                    Self::ty_contains_generic(ty, &generic_names)
                                                })
                                            {
                                                let specialised =
                                                    specialised_function_name(&base, &type_args);
                                                if &specialised != symbol {
                                                    *symbol = specialised.clone();
                                                    new_specs.push(FunctionSpecialization {
                                                        base: base.clone(),
                                                        specialized: specialised,
                                                        type_args,
                                                    });
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let param_names = self.method_generic_param_names(&canonical, 0);
                if std::env::var_os("CHIC_DEBUG_SPECIALISE_HASHING").is_some()
                    && (canonical.contains("Std::Hashing::Hashing::HashValue")
                        || canonical.contains("Std::Hashing::Hashing::HashCodeOf")
                        || canonical.contains("Std::Collections::HashSetHelpers::HashValue"))
                {
                    static COUNTER: std::sync::atomic::AtomicUsize =
                        std::sync::atomic::AtomicUsize::new(0);
                    let idx = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if idx < 40 {
                        eprintln!(
                            "[specialise-infer] base={base} symbol={symbol} params={:?} args_len={}",
                            param_names,
                            args.len()
                        );
                    }
                }
                if param_names.is_empty() {
                    continue;
                }
                let Some(callee) = self.find_base_function(module, &base) else {
                    continue;
                };
                if callee.signature.params.len() != args.len() {
                    if std::env::var_os("CHIC_DEBUG_SPECIALISE_HASHING").is_some()
                        && (canonical.contains("Std::Hashing::Hashing::HashValue")
                            || canonical.contains("Std::Hashing::Hashing::HashCodeOf")
                            || canonical.contains("Std::Collections::HashSetHelpers::HashValue"))
                    {
                        eprintln!(
                            "[specialise-infer] skip len_mismatch base={base} callee_params={} args={}",
                            callee.signature.params.len(),
                            args.len()
                        );
                    }
                    continue;
                }
                let generic_names: HashSet<String> = param_names.iter().cloned().collect();
                let mut bindings: HashMap<String, Ty> = HashMap::new();
                for (param_ty, arg) in callee.signature.params.iter().zip(args.iter()) {
                    let Some(arg_ty) = Self::operand_ty_with_layouts(module, &locals, arg) else {
                        bindings.clear();
                        break;
                    };
                    Self::bind_generic_params(param_ty, &arg_ty, &generic_names, &mut bindings);
                }
                if bindings.is_empty()
                    || !param_names.iter().all(|name| bindings.contains_key(name))
                {
                    if std::env::var_os("CHIC_DEBUG_SPECIALISE_HASHING").is_some()
                        && (canonical.contains("Std::Hashing::Hashing::HashValue")
                            || canonical.contains("Std::Hashing::Hashing::HashCodeOf")
                            || canonical.contains("Std::Collections::HashSetHelpers::HashValue"))
                    {
                        eprintln!(
                            "[specialise-infer] skip missing_bindings base={base} bindings={:?}",
                            bindings
                                .iter()
                                .map(|(k, v)| format!("{k}={}", v.canonical_name()))
                                .collect::<Vec<_>>()
                        );
                    }
                    continue;
                }
                let type_args: Vec<Ty> = param_names
                    .iter()
                    .filter_map(|name| bindings.get(name).cloned())
                    .collect();
                if type_args.iter().any(|ty| matches!(ty, Ty::Unknown)) {
                    continue;
                }
                let is_identity_instantiation = param_names
                    .iter()
                    .zip(type_args.iter())
                    .all(|(param, ty)| matches!(ty, Ty::Named(named) if named.as_str() == param));
                if is_identity_instantiation {
                    if std::env::var_os("CHIC_DEBUG_SPECIALISE_HASHING").is_some()
                        && (canonical.contains("Std::Hashing::Hashing::HashValue")
                            || canonical.contains("Std::Hashing::Hashing::HashCodeOf")
                            || canonical.contains("Std::Collections::HashSetHelpers::HashValue"))
                    {
                        eprintln!(
                            "[specialise-infer] skip identity base={base} type_args={:?}",
                            type_args
                                .iter()
                                .map(|ty| ty.canonical_name())
                                .collect::<Vec<_>>()
                        );
                    }
                    continue;
                }
                let specialised = specialised_function_name(&base, &type_args);
                if &specialised == symbol {
                    continue;
                }
                *symbol = specialised.clone();
                if std::env::var_os("CHIC_DEBUG_SPECIALISE_HASHING").is_some()
                    && (canonical.contains("Std::Hashing::Hashing::HashValue")
                        || canonical.contains("Std::Hashing::Hashing::HashCodeOf")
                        || canonical.contains("Std::Collections::HashSetHelpers::HashValue"))
                {
                    eprintln!("[specialise-infer] rewrite base={base} -> {specialised}");
                }
                new_specs.push(FunctionSpecialization {
                    base,
                    specialized: specialised,
                    type_args,
                });
            }
        }
        new_specs
    }

    fn operand_ty_with_layouts(
        module: &MirModule,
        locals: &[LocalDecl],
        operand: &Operand,
    ) -> Option<Ty> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                Self::place_ty_with_layouts(module, locals, place)
            }
            Operand::Borrow(borrow) => Self::place_ty_with_layouts(module, locals, &borrow.place),
            Operand::Mmio(spec) => Some(spec.ty.clone()),
            Operand::Const(_) | Operand::Pending(_) => None,
        }
    }

    fn place_ty_with_layouts(
        module: &MirModule,
        locals: &[LocalDecl],
        place: &Place,
    ) -> Option<Ty> {
        let local = locals.get(place.local.0)?;
        let mut ty = local.ty.clone();
        for projection in &place.projection {
            match projection {
                ProjectionElem::Deref => match ty {
                    Ty::Pointer(ptr) => ty = ptr.element.clone(),
                    Ty::Ref(reference) => ty = reference.element.clone(),
                    _ => return None,
                },
                ProjectionElem::FieldNamed(field_name) => {
                    let layout_name = ty.canonical_name();
                    let layout = module.type_layouts.layout_for_name(&layout_name)?;
                    match layout {
                        TypeLayout::Struct(info) | TypeLayout::Class(info) => {
                            let field =
                                info.fields.iter().find(|field| field.name == *field_name)?;
                            ty = field.ty.clone();
                        }
                        TypeLayout::Enum(_) | TypeLayout::Union(_) => return None,
                    }
                }
                ProjectionElem::Field(field_index) => {
                    let layout_name = ty.canonical_name();
                    let layout = module.type_layouts.layout_for_name(&layout_name)?;
                    match layout {
                        TypeLayout::Struct(info) | TypeLayout::Class(info) => {
                            let field = info
                                .fields
                                .iter()
                                .find(|field| field.index == *field_index)?;
                            ty = field.ty.clone();
                        }
                        TypeLayout::Enum(_) | TypeLayout::Union(_) => return None,
                    }
                }
                _ => return None,
            }
        }
        Some(ty)
    }

    fn operand_ty(locals: &[LocalDecl], operand: &Operand) -> Option<Ty> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => Self::place_ty(locals, place),
            Operand::Borrow(borrow) => Self::place_ty(locals, &borrow.place),
            Operand::Mmio(spec) => Some(spec.ty.clone()),
            Operand::Const(_) | Operand::Pending(_) => None,
        }
    }

    fn place_ty(locals: &[LocalDecl], place: &Place) -> Option<Ty> {
        let local = locals.get(place.local.0)?;
        let mut ty = local.ty.clone();
        for projection in &place.projection {
            match projection {
                ProjectionElem::Deref => match ty {
                    Ty::Pointer(ptr) => ty = ptr.element.clone(),
                    Ty::Ref(reference) => ty = reference.element.clone(),
                    _ => return None,
                },
                _ => return None,
            }
        }
        Some(ty)
    }

    fn bind_generic_params(
        param_ty: &Ty,
        arg_ty: &Ty,
        generic_names: &HashSet<String>,
        bindings: &mut HashMap<String, Ty>,
    ) {
        match param_ty {
            Ty::Named(named) if generic_names.contains(named.as_str()) => {
                let mut bound = arg_ty.clone();
                if let Ty::Nullable(inner) = bound {
                    bound = *inner;
                }
                if let Ty::Ref(reference) = bound {
                    bound = reference.element;
                }
                bindings.entry(named.as_str().to_string()).or_insert(bound);
            }
            Ty::Named(named) => {
                if let Ty::Named(arg_named) = arg_ty {
                    if named.as_str() == arg_named.as_str() {
                        for (param_arg, arg_arg) in named.args().iter().zip(arg_named.args().iter())
                        {
                            if let (GenericArg::Type(param_ty), GenericArg::Type(arg_ty)) =
                                (param_arg, arg_arg)
                            {
                                Self::bind_generic_params(
                                    param_ty,
                                    arg_ty,
                                    generic_names,
                                    bindings,
                                );
                            }
                        }
                    }
                }
            }
            Ty::Array(array) => {
                if let Ty::Array(arg_array) = arg_ty {
                    Self::bind_generic_params(
                        &array.element,
                        &arg_array.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Vec(vec) => {
                if let Ty::Vec(arg_vec) = arg_ty {
                    Self::bind_generic_params(
                        &vec.element,
                        &arg_vec.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Span(span) => {
                if let Ty::Span(arg_span) = arg_ty {
                    Self::bind_generic_params(
                        &span.element,
                        &arg_span.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::ReadOnlySpan(span) => {
                if let Ty::ReadOnlySpan(arg_span) = arg_ty {
                    Self::bind_generic_params(
                        &span.element,
                        &arg_span.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Rc(rc) => {
                if let Ty::Rc(arg_rc) = arg_ty {
                    Self::bind_generic_params(
                        &rc.element,
                        &arg_rc.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Arc(arc) => {
                if let Ty::Arc(arg_arc) = arg_ty {
                    Self::bind_generic_params(
                        &arc.element,
                        &arg_arc.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Tuple(tuple) => {
                if let Ty::Tuple(arg_tuple) = arg_ty {
                    for (param_ty, arg_ty) in tuple.elements.iter().zip(arg_tuple.elements.iter()) {
                        Self::bind_generic_params(param_ty, arg_ty, generic_names, bindings);
                    }
                }
            }
            Ty::Pointer(ptr) => {
                if let Ty::Pointer(arg_ptr) = arg_ty {
                    Self::bind_generic_params(
                        &ptr.element,
                        &arg_ptr.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Ref(reference) => {
                if let Ty::Ref(arg_ref) = arg_ty {
                    Self::bind_generic_params(
                        &reference.element,
                        &arg_ref.element,
                        generic_names,
                        bindings,
                    );
                }
            }
            Ty::Nullable(inner) => {
                if let Ty::Nullable(arg_inner) = arg_ty {
                    Self::bind_generic_params(inner, arg_inner, generic_names, bindings);
                }
            }
            _ => {}
        }
    }

    fn ty_contains_generic(ty: &Ty, generic_names: &HashSet<String>) -> bool {
        match ty {
            Ty::Named(named) => {
                if generic_names.contains(named.as_str()) {
                    return true;
                }
                named.args().iter().any(|arg| match arg {
                    GenericArg::Type(inner) => Self::ty_contains_generic(inner, generic_names),
                    GenericArg::Const(_) => false,
                })
            }
            Ty::Array(array) => Self::ty_contains_generic(&array.element, generic_names),
            Ty::Vec(vec) => Self::ty_contains_generic(&vec.element, generic_names),
            Ty::Span(span) => Self::ty_contains_generic(&span.element, generic_names),
            Ty::ReadOnlySpan(span) => Self::ty_contains_generic(&span.element, generic_names),
            Ty::Rc(rc) => Self::ty_contains_generic(&rc.element, generic_names),
            Ty::Arc(arc) => Self::ty_contains_generic(&arc.element, generic_names),
            Ty::Tuple(tuple) => tuple
                .elements
                .iter()
                .any(|elem| Self::ty_contains_generic(elem, generic_names)),
            Ty::Fn(fn_ty) => {
                fn_ty
                    .params
                    .iter()
                    .any(|param| Self::ty_contains_generic(param, generic_names))
                    || Self::ty_contains_generic(&fn_ty.ret, generic_names)
            }
            Ty::Pointer(ptr) => Self::ty_contains_generic(&ptr.element, generic_names),
            Ty::Ref(reference) => Self::ty_contains_generic(&reference.element, generic_names),
            Ty::Nullable(inner) => Self::ty_contains_generic(inner, generic_names),
            _ => false,
        }
    }

    fn substitute_specialised_symbol_args(
        symbol: &str,
        map: &HashMap<String, Ty>,
    ) -> Option<String> {
        let (base, args) = Self::parse_specialised_symbol_args(symbol)?;
        if args.is_empty() {
            return None;
        }

        let mut changed = false;
        let mut rewritten_args = Vec::with_capacity(args.len());
        for arg in args {
            if let Some(replacement) = map.get(&arg) {
                changed = true;
                rewritten_args.push(replacement.canonical_name());
            } else {
                rewritten_args.push(arg);
            }
        }

        if !changed {
            return None;
        }

        let mut rebuilt = String::with_capacity(symbol.len() + 16);
        rebuilt.push_str(&base);
        rebuilt.push('<');
        for (idx, arg) in rewritten_args.into_iter().enumerate() {
            if idx > 0 {
                rebuilt.push(',');
            }
            rebuilt.push_str(&arg);
        }
        rebuilt.push('>');
        Some(rebuilt)
    }

    fn split_specialised_symbol(symbol: &str) -> Option<(String, Vec<Ty>)> {
        let (base, args) = Self::parse_specialised_symbol_args(symbol)?;
        if args.is_empty() {
            return None;
        }
        let mut type_args = Vec::with_capacity(args.len());
        for arg in args {
            let ty = Self::parse_type_arg_fragment(&arg)?;
            type_args.push(ty);
        }
        Some((base, type_args))
    }

    fn parse_specialised_symbol_args(symbol: &str) -> Option<(String, Vec<String>)> {
        if !symbol.ends_with('>') {
            return None;
        }
        let start = symbol.find('<')?;
        let base = symbol[..start].to_string();

        let mut args = Vec::new();
        let mut depth = 0usize;
        let mut current = String::new();

        for ch in symbol[start + 1..symbol.len() - 1].chars() {
            match ch {
                '<' => {
                    depth += 1;
                    current.push(ch);
                }
                '>' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    args.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            }
        }
        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }
        Some((base, args))
    }

    fn parse_type_arg_fragment(fragment: &str) -> Option<Ty> {
        if let Some(expr) = parse_type_expression_text(fragment) {
            return Some(Ty::from_type_expr(&expr));
        }
        if fragment.contains("::") {
            let substituted = fragment.replace("::", ".");
            if let Some(mut expr) = parse_type_expression_text(&substituted) {
                expr.name = fragment.to_string();
                return Some(Ty::from_type_expr(&expr));
            }
        }
        Some(Ty::named(fragment))
    }

    fn ty_contains_unbound_generic(ty: &Ty) -> bool {
        fn is_unbound_generic_name(name: &str) -> bool {
            if name.contains("::") {
                return false;
            }
            let bytes = name.as_bytes();
            if bytes.len() == 1 && bytes[0].is_ascii_uppercase() {
                return true;
            }
            if name.starts_with('T')
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return true;
            }
            false
        }

        match ty {
            Ty::Named(named) => {
                if named.args().is_empty() {
                    return is_unbound_generic_name(named.as_str());
                }
                named.args().iter().any(|arg| match arg {
                    GenericArg::Type(inner) => Self::ty_contains_unbound_generic(inner),
                    _ => false,
                })
            }
            Ty::Array(array) => Self::ty_contains_unbound_generic(&array.element),
            Ty::Vec(vec) => Self::ty_contains_unbound_generic(&vec.element),
            Ty::Span(span) => Self::ty_contains_unbound_generic(&span.element),
            Ty::ReadOnlySpan(span) => Self::ty_contains_unbound_generic(&span.element),
            Ty::Rc(rc) => Self::ty_contains_unbound_generic(&rc.element),
            Ty::Arc(arc) => Self::ty_contains_unbound_generic(&arc.element),
            Ty::Tuple(tuple) => tuple.elements.iter().any(Self::ty_contains_unbound_generic),
            Ty::Fn(fn_ty) => {
                fn_ty.params.iter().any(Self::ty_contains_unbound_generic)
                    || Self::ty_contains_unbound_generic(&fn_ty.ret)
            }
            Ty::Pointer(ptr) => Self::ty_contains_unbound_generic(&ptr.element),
            Ty::Ref(reference) => Self::ty_contains_unbound_generic(&reference.element),
            Ty::Nullable(inner) => Self::ty_contains_unbound_generic(inner),
            Ty::Vector(vector) => Self::ty_contains_unbound_generic(&vector.element),
            Ty::TraitObject(_) | Ty::String | Ty::Str | Ty::Unit | Ty::Unknown => false,
        }
    }

    fn method_generic_param_names(&self, canonical: &str, required_len: usize) -> Vec<String> {
        fn base_name(name: &str) -> &str {
            let without_generics = name.split('<').next().unwrap_or(name).trim();
            without_generics
                .split('#')
                .next()
                .unwrap_or(without_generics)
                .trim()
        }

        let canonical_base = base_name(canonical);
        let mut selected: Option<Vec<FunctionDeclSymbol>> = self
            .symbol_index
            .function_decls(canonical_base)
            .map(|decls| decls.to_vec());
        if selected.is_none() {
            let mut best_len = 0usize;
            for decls in self.symbol_index.function_decl_groups() {
                let Some(first) = decls.first() else { continue };
                let qualified = &first.qualified;
                let qualified_base = base_name(qualified);
                if qualified == canonical
                    || qualified.ends_with(canonical)
                    || canonical.ends_with(qualified)
                    || qualified_base == canonical_base
                    || qualified_base.ends_with(canonical_base)
                    || canonical_base.ends_with(qualified_base)
                {
                    let score = qualified.len();
                    if score >= best_len {
                        best_len = score;
                        selected = Some(decls.clone());
                    }
                }
            }
        }
        let Some(decls) = selected else {
            return Vec::new();
        };

        fn type_param_names(function: &crate::frontend::ast::FunctionDecl) -> Vec<String> {
            function
                .generics
                .as_ref()
                .map(|params| {
                    params
                        .params
                        .iter()
                        .filter_map(|param| {
                            if matches!(param.kind, GenericParamKind::Type(_)) {
                                Some(param.name.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        if required_len > 0 {
            if let Some(symbol) = decls.iter().find(|symbol| {
                let names = type_param_names(&symbol.function);
                names.len() == required_len
            }) {
                return type_param_names(&symbol.function);
            }
        }

        decls
            .iter()
            .find_map(|symbol| {
                let names = type_param_names(&symbol.function);
                if !names.is_empty() { Some(names) } else { None }
            })
            .or_else(|| {
                decls
                    .first()
                    .map(|symbol| type_param_names(&symbol.function))
            })
            .unwrap_or_default()
    }
}
