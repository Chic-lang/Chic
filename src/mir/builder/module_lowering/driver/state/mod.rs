mod defaults;
mod helpers;
mod vtable;

use super::{executor, planner};
use crate::frontend::ast::{
    Attribute, BindingModifier, ClassDecl, ClassMember, Expression, FieldDecl, FnTypeExpr,
    GenericArgument, Parameter, PropertyAccessorKind, PropertyDecl, StaticItemDecl,
    StaticMutability, TraitObjectTypeExpr, TypeAliasDecl, TypeExpr, TypeSuffix,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::{ImportResolver, Resolution as ImportResolution};
use crate::frontend::type_alias::{TypeAlias, TypeAliasRegistry};
use crate::mir::TypeLayout;
use crate::mir::builder::FunctionSpecialization;
use crate::mir::builder::body_builder::BodyBuilder;
use crate::mir::builder::body_builder::opaque_return_info_from_ty;
use crate::mir::builder::const_eval::ConstEvalContext;
use crate::mir::builder::default_arguments::{
    DefaultArgumentMap, DefaultArgumentStore, DefaultArgumentValue,
};
use crate::mir::builder::functions::LoweredMethodMetadata;
use crate::mir::builder::module_lowering::cache::{LoweringCache, LoweringCacheMetrics};
use crate::mir::builder::module_lowering::traits::TraitLoweringInfo;
use crate::mir::builder::qualify;
use crate::mir::builder::static_registry::StaticRegistry;
use crate::mir::builder::symbol_index::{ConstructorDeclSymbol, FunctionDeclSymbol, SymbolIndex};
use crate::mir::builder::{
    Abi, FnSig, FunctionKind, LocalId, MirFunction, MirModule, MirStatement, MirStatementKind,
    Module, Place, Rvalue, Signature, StringInterner, Terminator, TraitVTable, Ty, TypeConstraint,
    TypeLayoutTable, Visibility,
};
use crate::mir::module_metadata::{Export, ModuleAttributes};
use crate::mir::operators::OperatorRegistry;
use crate::mir::passes::budget;
use crate::mir::{
    ClassVTable, ClassVTableSlot, ConstValue, DefaultArgumentKind, DefaultArgumentRecord,
    TestCaseMetadata, VTableSlot, trait_vtable_symbol_name,
};
use crate::perf::PerfMetadata;
use crate::primitives::{PrimitiveDescriptor, PrimitiveRegistry, normalize_name};
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub(crate) use helpers::{
    dispatch_participates, expect_u32_index, expr_path_segments, is_power_of_two,
};
use vtable::ClassVTablePlan;

/// Result of lowering a module to MIR.
#[derive(Debug, Default)]
pub struct LoweringResult {
    pub module: MirModule,
    pub diagnostics: Vec<LoweringDiagnostic>,
    pub constraints: Vec<TypeConstraint>,
    pub unit_slices: Vec<ModuleUnitSlice>,
    pub pass_metrics: Vec<PassStageMetric>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) cache_metrics: LoweringCacheMetrics,
    pub perf_metadata: PerfMetadata,
}

/// Captures non-fatal issues encountered during lowering.
#[derive(Debug, Clone)]
pub struct LoweringDiagnostic {
    pub message: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct ModuleUnitSlice {
    pub unit: usize,
    pub range: std::ops::Range<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PassStageMetric {
    pub name: &'static str,
    pub count: u64,
    pub duration_ns: u128,
}

/// Lower an AST module into MIR.
#[must_use]
pub fn lower_module(module: &Module) -> LoweringResult {
    ModuleLoweringDriver::new().lower_with_units(module, None, None)
}

/// Lower an AST module with optional per-item translation unit metadata.
#[must_use]
pub fn lower_module_with_units(
    module: &Module,
    item_units: Option<&[usize]>,
    unit_packages: Option<&[Option<String>]>,
) -> LoweringResult {
    ModuleLoweringDriver::new().lower_with_units(module, item_units, unit_packages)
}

/// Lower an AST module with an optional primitive hook and per-item unit metadata.
#[must_use]
pub fn lower_module_with_units_and_hook(
    module: &Module,
    item_units: Option<&[usize]>,
    unit_packages: Option<&[Option<String>]>,
    extra_primitives_hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
) -> LoweringResult {
    ModuleLoweringDriver::new()
        .with_extra_primitives_hook(extra_primitives_hook)
        .lower_with_units(module, item_units, unit_packages)
}

pub struct ModuleLoweringDriver {
    lowering: ModuleLowering,
    extra_primitives_hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
}

impl Default for ModuleLoweringDriver {
    fn default() -> Self {
        Self {
            lowering: ModuleLowering::default(),
            extra_primitives_hook: None,
        }
    }
}

impl ModuleLoweringDriver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_extra_primitives_hook(
        mut self,
        hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
    ) -> Self {
        self.extra_primitives_hook = hook;
        self
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn lower(&mut self, module: &Module) -> LoweringResult {
        self.lower_with_units(module, None, None)
    }

    #[must_use]
    pub fn lower_with_units(
        &mut self,
        module: &Module,
        item_units: Option<&[usize]>,
        unit_packages: Option<&[Option<String>]>,
    ) -> LoweringResult {
        self.lowering
            .apply_extra_primitives_hook(self.extra_primitives_hook);
        self.lowering.unit_packages = unit_packages.map(|pkgs| pkgs.to_vec()).unwrap_or_default();
        let plan = planner::LowerPlanner::new().plan(module, item_units);
        let executor = executor::LowerExecutor::new(&mut self.lowering);
        let result = executor.run(module, &plan);
        budget::record_pass_metrics(&result.pass_metrics);
        if std::env::var_os("DUMP_LOWER").is_some() {
            for func in &result.module.functions {
                eprintln!("function {}:", func.name);
                eprintln!("  locals: {:#?}", func.body.locals);
                for (index, block) in func.body.blocks.iter().enumerate() {
                    eprintln!("  block {index}: {:#?}", block.statements);
                    eprintln!("  terminator: {:#?}", block.terminator);
                }
            }
        }
        result
    }
}

pub(crate) struct ModuleLowering {
    pub(crate) functions: Vec<MirFunction>,
    pub(crate) diagnostics: Vec<LoweringDiagnostic>,
    pub(crate) constraints: Vec<TypeConstraint>,
    pub(crate) type_layouts: TypeLayoutTable,
    pub(crate) primitive_registry: PrimitiveRegistry,
    pub(crate) string_interner: StringInterner,
    pub(crate) exports: Vec<Export>,
    pub(crate) type_visibilities: HashMap<String, TypeDeclInfo>,
    pub(crate) class_bases: HashMap<String, Vec<String>>,
    pub(crate) operator_registry: OperatorRegistry,
    pub(crate) symbol_index: SymbolIndex,
    pub(crate) import_resolver: ImportResolver,
    pub(crate) unit_import_resolvers: Option<Vec<ImportResolver>>,
    pub(crate) type_aliases: TypeAliasRegistry,
    pub(crate) item_units: Option<Vec<usize>>,
    pub(crate) unit_packages: Vec<Option<String>>,
    pub(crate) root_item_index: usize,
    pub(crate) current_package: Option<String>,
    pub(crate) unit_slices: Vec<ModuleUnitSlice>,
    pub(crate) exported_symbols: HashSet<String>,
    pub(crate) linked_libraries: HashSet<String>,
    pub(crate) module_attributes: ModuleAttributes,
    pub(crate) cache: LoweringCache,
    pub(crate) trait_decls: HashMap<String, TraitLoweringInfo>,
    pub(crate) trait_vtables: Vec<TraitVTable>,
    pub(crate) class_decls: HashMap<String, ClassDecl>,
    class_vtable_plans: HashMap<String, ClassVTablePlan>,
    class_vtable_in_progress: HashSet<String>,
    pub(crate) class_virtual_slots: HashMap<String, HashMap<String, u32>>,
    pub(crate) test_cases: Vec<TestCaseMetadata>,
    pub(crate) static_registry: StaticRegistry,
    pub(crate) function_ordinals: HashMap<String, usize>,
    pub(crate) default_arguments: DefaultArgumentStore,
    pub(crate) default_argument_records: Vec<DefaultArgumentRecord>,
    pub(crate) perf_metadata: PerfMetadata,
    pub(crate) generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
    pub(crate) extra_primitives: Vec<PrimitiveDescriptor>,
    pub(crate) registered_primitives: Vec<PrimitiveDescriptor>,
    pub(crate) extra_primitive_names: HashSet<String>,
    pub(crate) function_packages: HashMap<String, String>,
}

impl Default for ModuleLowering {
    fn default() -> Self {
        Self {
            functions: Vec::new(),
            diagnostics: Vec::new(),
            constraints: Vec::new(),
            type_layouts: TypeLayoutTable::default(),
            primitive_registry: PrimitiveRegistry::with_builtins(),
            string_interner: StringInterner::new(),
            exports: Vec::new(),
            type_visibilities: HashMap::new(),
            class_bases: HashMap::new(),
            operator_registry: OperatorRegistry::default(),
            symbol_index: SymbolIndex::default(),
            import_resolver: ImportResolver::default(),
            unit_import_resolvers: None,
            type_aliases: TypeAliasRegistry::default(),
            item_units: None,
            unit_packages: Vec::new(),
            root_item_index: 0,
            current_package: None,
            unit_slices: Vec::new(),
            exported_symbols: HashSet::new(),
            linked_libraries: HashSet::new(),
            module_attributes: ModuleAttributes::default(),
            cache: LoweringCache::new(),
            trait_decls: HashMap::new(),
            trait_vtables: Vec::new(),
            class_decls: HashMap::new(),
            class_vtable_plans: HashMap::new(),
            class_vtable_in_progress: HashSet::new(),
            class_virtual_slots: HashMap::new(),
            test_cases: Vec::new(),
            static_registry: StaticRegistry::new(),
            function_ordinals: HashMap::new(),
            default_arguments: Rc::new(RefCell::new(DefaultArgumentMap::default())),
            default_argument_records: Vec::new(),
            perf_metadata: PerfMetadata::default(),
            generic_specializations: Rc::new(RefCell::new(Vec::new())),
            extra_primitives: Vec::new(),
            registered_primitives: Vec::new(),
            extra_primitive_names: HashSet::new(),
            function_packages: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TypeDeclInfo {
    pub(crate) visibility: Visibility,
    pub(crate) namespace: Option<String>,
    pub(crate) enclosing_type: Option<String>,
    pub(crate) package: Option<String>,
}

pub(crate) fn visibility_keyword(vis: Visibility) -> &'static str {
    match vis {
        Visibility::Public => "public",
        Visibility::Internal => "internal",
        Visibility::Protected => "protected",
        Visibility::Private => "private",
        Visibility::ProtectedInternal => "protected internal",
        Visibility::PrivateProtected => "private protected",
    }
}

fn normalized_attribute_name(name: &str) -> String {
    let mut name = name.to_ascii_lowercase();
    name.retain(|ch| ch != '_' && ch != '-');
    name
}

fn has_attribute(attributes: &[Attribute], target: &str) -> bool {
    attributes
        .iter()
        .any(|attr| normalized_attribute_name(&attr.name) == target)
}

fn has_threadlocal_attribute(attributes: &[Attribute]) -> bool {
    has_attribute(attributes, "threadlocal")
}

fn has_weak_attribute(attributes: &[Attribute]) -> bool {
    has_attribute(attributes, "weak")
}

fn has_weak_import_attribute(attributes: &[Attribute]) -> bool {
    has_attribute(attributes, "weakimport")
}

impl ModuleLowering {
    pub(crate) fn import_resolver_for_unit(&self, unit: usize) -> &ImportResolver {
        if let Some(resolvers) = self.unit_import_resolvers.as_ref()
            && let Some(resolver) = resolvers.get(unit)
        {
            return resolver;
        }
        &self.import_resolver
    }

    pub(crate) fn apply_extra_primitives_hook(
        &mut self,
        hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
    ) {
        self.extra_primitives.clear();
        self.extra_primitive_names.clear();
        let Some(hook_fn) = hook else {
            return;
        };

        let mut descriptors: Vec<(usize, PrimitiveDescriptor, Option<String>)> = hook_fn()
            .into_iter()
            .enumerate()
            .map(|(idx, desc)| {
                let normalized = normalize_name(&desc.primitive_name);
                (idx, desc, normalized)
            })
            .collect();
        descriptors.sort_by(|a, b| match (&a.2, &b.2) {
            (Some(an), Some(bn)) => {
                let ord = an.cmp(bn);
                if ord == Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    ord
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.0.cmp(&b.0),
        });

        for (_, desc, normalized) in descriptors {
            let Some(primary) = normalized else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "primitive name must not be empty in extra primitive hook".to_string(),
                    span: desc.span,
                });
                continue;
            };

            let aliases = Self::normalized_aliases(&desc, &primary);
            if let Some(conflict) = self.conflicts_with_builtin(&aliases) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "primitive `{primary}` from extra primitive hook conflicts with built-in primitive `{conflict}`"
                    ),
                    span: desc.span,
                });
                continue;
            }

            match self.register_descriptor_pair(desc) {
                Ok(registered) => {
                    let primary = normalize_name(&registered.primitive_name).unwrap_or_default();
                    for alias in Self::normalized_aliases(&registered, &primary) {
                        self.extra_primitive_names.insert(alias);
                    }
                    self.extra_primitives.push(registered);
                }
                Err(_) => {
                    // Diagnostics already recorded.
                }
            }
        }
    }

    pub(crate) fn reset_class_vtable_state(&mut self) {
        self.class_decls.clear();
        self.class_vtable_plans.clear();
        self.class_vtable_in_progress.clear();
        self.class_virtual_slots.clear();
    }

    pub(crate) fn normalized_aliases(desc: &PrimitiveDescriptor, primary: &str) -> HashSet<String> {
        let mut aliases = HashSet::new();
        aliases.insert(primary.to_string());
        for alias in &desc.aliases {
            if let Some(normalized) = normalize_name(alias) {
                aliases.insert(normalized);
            }
        }
        aliases
    }

    pub(crate) fn conflicts_with_builtin(&self, aliases: &HashSet<String>) -> Option<String> {
        for alias in aliases {
            if self.extra_primitive_names.contains(alias) {
                continue;
            }
            if let Some(id) = self.primitive_registry.lookup_by_name(alias) {
                if let Some(existing) = self.primitive_registry.descriptor(id) {
                    return Some(existing.primitive_name.clone());
                }
                return Some(alias.clone());
            }
        }
        None
    }

    pub(crate) fn prune_conflicting_extras(
        &mut self,
        aliases: &HashSet<String>,
        span: Option<Span>,
    ) -> bool {
        if !aliases
            .iter()
            .any(|alias| self.extra_primitive_names.contains(alias))
        {
            return false;
        }
        let winner = aliases.iter().next().cloned().unwrap_or_default();
        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "primitive `{winner}` from `@primitive` conflicts with an extra primitive hook; using the Chic definition"
            ),
            span,
        });

        let mut retained = Vec::new();
        for desc in self.extra_primitives.drain(..) {
            let primary = normalize_name(&desc.primitive_name).unwrap_or_default();
            let desc_aliases = Self::normalized_aliases(&desc, &primary);
            if desc_aliases.iter().any(|alias| aliases.contains(alias)) {
                continue;
            }
            retained.push(desc);
        }
        self.extra_primitives = retained;
        self.extra_primitive_names.clear();
        for desc in &self.extra_primitives {
            let primary = normalize_name(&desc.primitive_name).unwrap_or_default();
            for alias in Self::normalized_aliases(desc, &primary) {
                self.extra_primitive_names.insert(alias);
            }
        }
        true
    }

    pub(crate) fn register_descriptor_pair(
        &mut self,
        desc: PrimitiveDescriptor,
    ) -> Result<PrimitiveDescriptor, ()> {
        let main_result = self.primitive_registry.register(desc.clone());
        let Ok(id) = main_result else {
            if let Err(err) = main_result {
                self.push_primitive_error(err);
            }
            return Err(());
        };

        if let Err(err) = self.type_layouts.primitive_registry.register(desc) {
            self.push_primitive_error(err);
            self.rebuild_primitive_registries();
            return Err(());
        }

        if let Some(registered) = self.primitive_registry.descriptor(id).cloned() {
            return Ok(registered);
        }
        Err(())
    }

    pub(crate) fn rebuild_primitive_registries(&mut self) {
        let extras = self.extra_primitives.clone();
        let chic = self.registered_primitives.clone();
        self.primitive_registry = PrimitiveRegistry::with_builtins();
        self.type_layouts.primitive_registry = PrimitiveRegistry::with_builtins();
        self.extra_primitive_names.clear();

        let mut rebuilt_extras = Vec::new();
        for desc in extras {
            if let Ok(registered) = self.register_descriptor_pair(desc) {
                let primary = normalize_name(&registered.primitive_name).unwrap_or_default();
                for alias in Self::normalized_aliases(&registered, &primary) {
                    self.extra_primitive_names.insert(alias);
                }
                rebuilt_extras.push(registered);
            }
        }
        self.extra_primitives = rebuilt_extras;

        self.registered_primitives.clear();
        for desc in chic {
            if let Ok(registered) = self.register_descriptor_pair(desc) {
                self.registered_primitives.push(registered);
            }
        }
    }

    pub(crate) fn allocate_internal_name(&mut self, display_name: &str) -> String {
        let counter = self
            .function_ordinals
            .entry(display_name.to_string())
            .or_insert(0);
        let ordinal = *counter;
        *counter += 1;
        if ordinal == 0 {
            display_name.to_string()
        } else {
            format!("{display_name}#{ordinal}")
        }
    }

    pub(crate) fn register_static_field_decl(
        &mut self,
        type_name: &str,
        namespace: Option<&str>,
        owner_visibility: Visibility,
        field: &FieldDecl,
    ) {
        if !field.is_static {
            return;
        }
        let qualified = format!("{type_name}::{}", field.name);
        let threadlocal = has_threadlocal_attribute(&field.attributes);
        let is_weak = has_weak_attribute(&field.attributes);
        let visibility =
            if owner_visibility != Visibility::Public && field.visibility == Visibility::Public {
                owner_visibility
            } else {
                field.visibility
            };
        self.static_registry.register(
            qualified,
            Some(type_name.to_string()),
            namespace.map(str::to_string),
            field.ty.clone(),
            field.initializer.clone(),
            visibility,
            field.is_readonly,
            threadlocal,
            is_weak,
            false,
            None,
            None,
            None,
            false,
            None,
        );
    }

    pub(crate) fn register_static_property_backing(
        &mut self,
        type_name: &str,
        namespace: Option<&str>,
        property: &PropertyDecl,
    ) {
        if !property.is_static || !property.is_auto() {
            return;
        }
        let qualified = format!("{type_name}::{}", property.backing_field_name());
        let has_setter = property.accessor(PropertyAccessorKind::Set).is_some();
        let has_init = property.accessor(PropertyAccessorKind::Init).is_some();
        let threadlocal = has_threadlocal_attribute(&property.attributes);
        let is_weak = has_weak_attribute(&property.attributes);
        self.static_registry.register(
            qualified,
            Some(type_name.to_string()),
            namespace.map(str::to_string),
            property.ty.clone(),
            property.initializer.clone(),
            Visibility::Private,
            !(has_setter || has_init),
            threadlocal,
            is_weak,
            false,
            None,
            None,
            None,
            false,
            property.span,
        );
    }

    pub(crate) fn register_static_item(&mut self, namespace: Option<&str>, item: &StaticItemDecl) {
        let is_readonly = matches!(item.declaration.mutability, StaticMutability::Const);
        let threadlocal = has_threadlocal_attribute(&item.declaration.attributes);
        let is_weak = has_weak_attribute(&item.declaration.attributes);
        let is_weak_import = item.declaration.is_weak_import
            || has_weak_import_attribute(&item.declaration.attributes);
        for decl in &item.declaration.declarators {
            let qualified = qualify(namespace, &decl.name);
            self.static_registry.register(
                qualified,
                None,
                namespace.map(str::to_string),
                item.declaration.ty.clone(),
                decl.initializer.clone(),
                item.visibility,
                is_readonly,
                threadlocal,
                is_weak,
                item.declaration.is_extern,
                item.declaration.extern_abi.clone(),
                item.declaration.extern_options.clone(),
                item.declaration.link_library.clone(),
                is_weak_import,
                decl.span.or(item.declaration.span),
            );
        }
        if item.declaration.is_extern {
            self.collect_link_library(item.declaration.link_library.as_deref());
        }
    }

    pub(crate) fn register_type_alias(&mut self, _alias: &TypeAliasDecl, _namespace: Option<&str>) {
        // Aliases are tracked via the collected registry; no layout registration needed here.
    }

    pub(crate) fn resolve_type_for_expr(
        &mut self,
        ty: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> ImportResolution {
        let mut alias_stack = Vec::new();
        if let Some(expanded) = self.try_expand_alias(ty, namespace, context_type, &mut alias_stack)
        {
            return self.resolve_type_for_expr(&expanded, namespace, context_type);
        }
        if ty.tuple_elements.is_some() || ty.fn_signature.is_some() {
            return ImportResolution::NotFound;
        }
        let primitive_registry = &self.primitive_registry;
        let canonicalize = |candidate: &str| -> Option<String> {
            if self.type_visibilities.contains_key(candidate) {
                return Some(candidate.to_string());
            }
            primitive_registry
                .descriptor_for_name(candidate)
                .map(|desc| {
                    desc.std_wrapper_type
                        .clone()
                        .unwrap_or_else(|| desc.primitive_name.clone())
                })
        };
        if ty.pointer_depth() > 0 {
            let mut element = ty.clone();
            element
                .suffixes
                .retain(|suffix| !matches!(suffix, TypeSuffix::Pointer { .. }));
            if element.pointer_depth() < ty.pointer_depth() {
                return self.resolve_type_for_expr(&element, namespace, context_type);
            }
        }
        let Some(base_segments) = self.alias_base_segments(ty) else {
            return ImportResolution::NotFound;
        };

        match self
            .import_resolver
            .resolve_type(&base_segments, namespace, context_type, |name| {
                canonicalize(name).is_some()
            }) {
            ImportResolution::Found(name) => canonicalize(&name)
                .map(ImportResolution::Found)
                .unwrap_or(ImportResolution::NotFound),
            ImportResolution::Ambiguous(candidates) => {
                let mut resolved: Vec<String> = Vec::new();
                for candidate in candidates {
                    if let Some(actual) = canonicalize(&candidate) {
                        let actual_norm = normalize_name(&actual);
                        let is_duplicate = resolved.iter().any(|existing| {
                            if let (Some(a), Some(b)) =
                                (normalize_name(existing), actual_norm.clone())
                            {
                                a == b
                            } else {
                                existing == &actual
                            }
                        });
                        if !is_duplicate {
                            resolved.push(actual);
                        }
                    }
                }
                match resolved.len() {
                    0 => ImportResolution::NotFound,
                    1 => ImportResolution::Found(resolved.remove(0)),
                    _ => ImportResolution::Ambiguous(resolved),
                }
            }
            ImportResolution::NotFound => ImportResolution::NotFound,
        }
    }

    fn alias_base_segments(&self, expr: &TypeExpr) -> Option<Vec<String>> {
        if !expr.base.is_empty() {
            return Some(expr.base.clone());
        }
        if expr.name.is_empty() {
            return None;
        }
        Some(
            expr.name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    pub(crate) fn try_expand_alias(
        &mut self,
        expr: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
        stack: &mut Vec<String>,
    ) -> Option<TypeExpr> {
        if expr.tuple_elements.is_some() || expr.fn_signature.is_some() {
            return None;
        }
        let Some(base_segments) = self.alias_base_segments(expr) else {
            return None;
        };
        let resolution = self.import_resolver.resolve_type(
            &base_segments,
            namespace,
            context_type,
            |candidate| self.type_aliases.get(candidate).is_some(),
        );
        match resolution {
            ImportResolution::Found(name) => {
                let Some(alias) = self.type_aliases.get(&name).cloned() else {
                    return None;
                };
                if stack.contains(&name) {
                    let span = expr.span.or(alias.span);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("type alias `{name}` forms a cycle"),
                        span,
                    });
                    return None;
                }
                stack.push(name.clone());
                let Some(mut expanded) = self.instantiate_alias(&alias, expr) else {
                    stack.pop();
                    return None;
                };
                if let Some(nested) =
                    self.try_expand_alias(&expanded, namespace, context_type, stack)
                {
                    expanded = nested;
                }
                stack.pop();
                Some(expanded)
            }
            ImportResolution::Ambiguous(candidates) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "type `{}` resolves to multiple aliases: {}",
                        expr.name,
                        candidates.join(", ")
                    ),
                    span: expr.span,
                });
                None
            }
            ImportResolution::NotFound => None,
        }
    }

    fn instantiate_alias(&mut self, alias: &TypeAlias, expr: &TypeExpr) -> Option<TypeExpr> {
        let args = expr
            .generic_arguments()
            .map(|args| args.to_vec())
            .unwrap_or_default();
        let params = alias
            .generics
            .as_ref()
            .map(|list| list.params.as_slice())
            .unwrap_or_default();
        if params.is_empty() && !args.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("type alias `{}` is not generic", alias.name),
                span: expr.generic_span.or(expr.span),
            });
            return None;
        }
        if !params.is_empty() && params.len() != args.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "type alias `{}` expects {} type argument{}, but {} {} supplied",
                    alias.name,
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
                span: expr.generic_span.or(expr.span),
            });
            return None;
        }

        let mut map = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            if param.as_const().is_some() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "const generic parameters are not supported on type aliases".into(),
                    span: param.span.or(expr.span),
                });
                continue;
            }
            let Some(arg_ty) = arg.ty() else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "type alias `{}` expects a type argument for `{}`, but a const expression was supplied",
                        alias.name, param.name
                    ),
                    span: expr.generic_span.or(expr.span),
                });
                continue;
            };
            map.insert(param.name.clone(), arg_ty.clone());
        }

        if map.len() != params.len() {
            return None;
        }

        let mut expanded = self.substitute_alias_params(&alias.target, &map);
        let extra_suffixes: Vec<_> = expr
            .suffixes
            .iter()
            .filter(|suffix| !matches!(suffix, TypeSuffix::GenericArgs(_)))
            .cloned()
            .collect();
        expanded.suffixes.extend(extra_suffixes);
        if expr.ref_kind.is_some() {
            expanded.ref_kind = expr.ref_kind;
        }
        expanded.is_view |= expr.is_view;
        expanded.span = expr.span.or(alias.span);
        Some(expanded)
    }

    fn substitute_alias_params(
        &self,
        expr: &TypeExpr,
        map: &HashMap<String, TypeExpr>,
    ) -> TypeExpr {
        if expr.base.len() == 1 {
            if let Some(replacement) = map.get(&expr.base[0]) {
                let mut substituted = replacement.clone();
                substituted.suffixes.extend(
                    expr.suffixes
                        .iter()
                        .cloned()
                        .filter(|suffix| !matches!(suffix, TypeSuffix::GenericArgs(_))),
                );
                if expr.ref_kind.is_some() {
                    substituted.ref_kind = expr.ref_kind;
                }
                substituted.is_view |= expr.is_view;
                substituted.span = expr.span.or(substituted.span);
                return substituted;
            }
        }

        let mut cloned = expr.clone();
        if let Some(elements) = &expr.tuple_elements {
            cloned.tuple_elements = Some(
                elements
                    .iter()
                    .map(|element| self.substitute_alias_params(element, map))
                    .collect(),
            );
        }
        if let Some(names) = &expr.tuple_element_names {
            cloned.tuple_element_names = Some(names.clone());
        }
        if let Some(signature) = &expr.fn_signature {
            cloned.fn_signature = Some(FnTypeExpr {
                abi: signature.abi.clone(),
                params: signature
                    .params
                    .iter()
                    .map(|param| self.substitute_alias_params(param, map))
                    .collect(),
                return_type: Box::new(
                    self.substitute_alias_params(signature.return_type.as_ref(), map),
                ),
                variadic: signature.variadic,
            });
        }
        if let Some(object) = &expr.trait_object {
            cloned.trait_object = Some(TraitObjectTypeExpr {
                bounds: object
                    .bounds
                    .iter()
                    .map(|bound| self.substitute_alias_params(bound, map))
                    .collect(),
                opaque_impl: object.opaque_impl,
            });
        }

        cloned.suffixes = expr
            .suffixes
            .iter()
            .map(|suffix| match suffix {
                TypeSuffix::GenericArgs(args) => TypeSuffix::GenericArgs(
                    args.iter()
                        .map(|arg| {
                            if let Some(ty) = arg.ty() {
                                GenericArgument::from_type_expr(
                                    self.substitute_alias_params(ty, map),
                                )
                            } else {
                                arg.clone()
                            }
                        })
                        .collect(),
                ),
                TypeSuffix::Array(spec) => TypeSuffix::Array(*spec),
                TypeSuffix::Nullable => TypeSuffix::Nullable,
                TypeSuffix::Pointer { mutable, modifiers } => TypeSuffix::Pointer {
                    mutable: *mutable,
                    modifiers: modifiers.clone(),
                },
                TypeSuffix::Qualifier(name) => TypeSuffix::Qualifier(name.clone()),
            })
            .collect();

        cloned
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Enum layout registration must walk each variant and compute aggregate metadata; splitting would obscure the sequential flow."
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "Union layout setup needs to consider view/field combinations in one pass."
    )]

    pub(crate) fn check_signature(
        &mut self,
        signature: &Signature,
        namespace: Option<&str>,
        enclosing_type: Option<&str>,
        context_name: &str,
    ) {
        self.ensure_type_expr_accessible(
            &signature.return_type,
            namespace,
            enclosing_type,
            &format!("return type of `{context_name}`"),
            None,
        );
        let _ = self.ty_from_type_expr(&signature.return_type, namespace, enclosing_type);
        for param in &signature.parameters {
            self.ensure_type_expr_accessible(
                &param.ty,
                namespace,
                enclosing_type,
                &format!("parameter `{}` of `{context_name}`", param.name),
                None,
            );
            let _ = self.ty_from_type_expr(&param.ty, namespace, enclosing_type);
        }
    }

    pub(crate) fn synthesise_interface_trait_vtables(&mut self) {
        let mut existing: HashSet<(String, String)> = HashSet::new();
        for vtable in &self.trait_vtables {
            existing.insert((vtable.trait_name.clone(), vtable.impl_type.clone()));
        }
        for (class, bases) in self.class_bases.clone() {
            for base in bases {
                if !self.trait_decls.contains_key(&base) {
                    continue;
                }
                let is_intrinsic = self
                    .type_layouts
                    .types
                    .get(&class)
                    .map(|layout| match layout {
                        TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
                            layout.is_intrinsic
                        }
                        _ => false,
                    })
                    .unwrap_or(false);
                if is_intrinsic && !self.class_decls.contains_key(&class) {
                    // Intrinsic structs/interfaces rely on compiler-provided glue; skip vtable synthesis.
                    continue;
                }
                if !existing.insert((base.clone(), class.clone())) {
                    continue;
                }
                let Some(info) = self.trait_decls.get(&base) else {
                    continue;
                };
                let mut slots = Vec::new();
                for method in &info.methods {
                    let impl_method = self.class_decls.get(&class).and_then(|decl| {
                        decl.members.iter().find_map(|member| match member {
                            ClassMember::Method(func)
                                if func.name == method.name
                                    && !func.modifiers.iter().any(|modifier| {
                                        modifier.eq_ignore_ascii_case("static")
                                    }) =>
                            {
                                Some(func)
                            }
                            _ => None,
                        })
                    });
                    if let Some(func) = impl_method {
                        if func.is_async != method.is_async {
                            let expectation = if method.is_async {
                                "must be marked async to satisfy the interface method"
                            } else {
                                "must not be async to satisfy the interface method"
                            };
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "class `{class}` mismatches asyncness of `{}`: {expectation}",
                                    method.name
                                ),
                                span: None,
                            });
                        }
                    }
                    let qualified = format!("{class}::{}", method.name);
                    let symbol = self
                        .symbol_index
                        .function_overloads(&qualified)
                        .and_then(|candidates| candidates.first().cloned())
                        .map(|entry| entry.internal_name.clone());
                    if let Some(symbol) = symbol {
                        slots.push(VTableSlot {
                            method: method.name.clone(),
                            symbol,
                        });
                    } else {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "class `{class}` does not implement interface `{base}` method `{}`",
                                method.name
                            ),
                            span: None,
                        });
                    }
                }
                if slots.is_empty() {
                    continue;
                }
                let vt_symbol = trait_vtable_symbol_name(&base, &class);
                self.trait_vtables.push(TraitVTable {
                    symbol: vt_symbol,
                    trait_name: base.clone(),
                    impl_type: class.clone(),
                    slots,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
