use super::super::functions::{LoweredFunction, lower_function, lower_testcase};
use super::super::{FunctionKind, Item, Visibility, qualify};
use super::cache::CachedLowering;
use super::driver::{ModuleLowering, ModuleUnitSlice, TypeDeclInfo};
use crate::frontend::ast::items::UsingKind;
use crate::frontend::attributes::{
    AttributeError, collect_cost_attribute, collect_trace_attribute,
};
use crate::mir::StrLifetime;
use crate::mir::builder::symbol_index::canonical_method_owner;
use crate::perf::{CostModel, TraceLevel, Tracepoint, trace_id};
use blake3::hash;
use std::sync::OnceLock;
use tracing::{debug, info_span};

impl ModuleLowering {
    pub(super) fn record_decl_packages(&mut self, items: &[Item], namespace: Option<&str>) {
        for (index, item) in items.iter().enumerate() {
            let unit = self
                .item_units
                .as_ref()
                .and_then(|units| units.get(index).copied())
                .unwrap_or(0);
            let package = self.unit_packages.get(unit).and_then(|pkg| pkg.clone());
            self.record_item_package(item, namespace, unit, package.as_deref());
        }
    }

    fn record_decl_packages_with_unit(
        &mut self,
        items: &[Item],
        namespace: Option<&str>,
        unit: usize,
        package: Option<&str>,
    ) {
        for item in items {
            self.record_item_package(item, namespace, unit, package);
        }
    }

    fn record_item_package(
        &mut self,
        item: &Item,
        namespace: Option<&str>,
        unit: usize,
        package: Option<&str>,
    ) {
        match item {
            Item::Namespace(ns) => {
                let nested = qualify(namespace, &ns.name);
                self.record_decl_packages_with_unit(&ns.items, Some(&nested), unit, package);
            }
            Item::Function(func) => {
                let qualified = qualify(namespace, &func.name);
                self.record_function_package(&qualified, package);
            }
            Item::TestCase(test) => {
                let qualified = qualify(namespace, &test.name);
                self.record_function_package(&qualified, package);
            }
            Item::Struct(strct) => {
                let type_name = qualify(namespace, &strct.name);
                self.upsert_type_package(&type_name, strct.visibility, namespace, None, package);
                for method in &strct.methods {
                    let qualified = format!("{type_name}::{}", method.name);
                    self.record_function_package(&qualified, package);
                }
                for (index, _ctor) in strct.constructors.iter().enumerate() {
                    let qualified = format!("{type_name}::init#{index}");
                    self.record_function_package(&qualified, package);
                }
                if !strct.nested_types.is_empty() {
                    let nested = qualify(namespace, &strct.name);
                    self.record_decl_packages_with_unit(
                        &strct.nested_types,
                        Some(&nested),
                        unit,
                        package,
                    );
                }
            }
            Item::Union(union_def) => {
                let type_name = qualify(namespace, &union_def.name);
                self.upsert_type_package(
                    &type_name,
                    union_def.visibility,
                    namespace,
                    None,
                    package,
                );
            }
            Item::Enum(enm) => {
                let type_name = qualify(namespace, &enm.name);
                self.upsert_type_package(&type_name, enm.visibility, namespace, None, package);
            }
            Item::Class(class) => {
                let type_name = qualify(namespace, &class.name);
                self.upsert_type_package(&type_name, class.visibility, namespace, None, package);
                let mut ctor_index = 0usize;
                for member in &class.members {
                    match member {
                        crate::frontend::ast::ClassMember::Method(method) => {
                            let qualified = format!("{type_name}::{}", method.name);
                            self.record_function_package(&qualified, package);
                        }
                        crate::frontend::ast::ClassMember::Constructor(_) => {
                            let qualified = format!("{type_name}::init#{ctor_index}");
                            self.record_function_package(&qualified, package);
                            ctor_index += 1;
                        }
                        crate::frontend::ast::ClassMember::Field(_)
                        | crate::frontend::ast::ClassMember::Property(_)
                        | crate::frontend::ast::ClassMember::Const(_) => {}
                    }
                }
                if !class.nested_types.is_empty() {
                    let nested = qualify(namespace, &class.name);
                    self.record_decl_packages_with_unit(
                        &class.nested_types,
                        Some(&nested),
                        unit,
                        package,
                    );
                }
            }
            Item::Interface(iface) => {
                let type_name = qualify(namespace, &iface.name);
                self.upsert_type_package(&type_name, iface.visibility, namespace, None, package);
            }
            Item::Delegate(delegate) => {
                let type_name = qualify(namespace, &delegate.name);
                self.upsert_type_package(&type_name, delegate.visibility, namespace, None, package);
            }
            Item::Extension(ext) => {
                if ext.target.base.is_empty() {
                    return;
                }
                let base = ext.target.base.join("::");
                let mut candidates = Vec::new();
                candidates.push(base.clone());
                if let Some(ns) = namespace {
                    candidates.push(format!("{ns}::{base}"));
                }

                for member in &ext.members {
                    let crate::frontend::ast::ExtensionMember::Method(method) = member;
                    for candidate in &candidates {
                        let owner_key = canonical_method_owner(candidate);
                        let qualified = format!("{owner_key}::{}", method.function.name);
                        self.record_function_package(&qualified, package);
                    }
                }
            }
            Item::Trait(trait_decl) => {
                let type_name = qualify(namespace, &trait_decl.name);
                self.upsert_type_package(
                    &type_name,
                    trait_decl.visibility,
                    namespace,
                    None,
                    package,
                );
            }
            Item::Impl(_) => {}
            Item::Const(_) | Item::Static(_) | Item::Import(_) | Item::TypeAlias(_) => {}
        }
    }

    fn record_function_package(&mut self, qualified: &str, package: Option<&str>) {
        if let Some(pkg) = package {
            self.function_packages
                .entry(qualified.to_string())
                .or_insert_with(|| pkg.to_string());
        }
    }

    fn upsert_type_package(
        &mut self,
        type_name: &str,
        visibility: Visibility,
        namespace: Option<&str>,
        enclosing_type: Option<&str>,
        package: Option<&str>,
    ) {
        self.type_visibilities
            .entry(type_name.to_string())
            .and_modify(|info| {
                if info.package.is_none() {
                    info.package = package.map(str::to_string);
                }
                if info.namespace.is_none() {
                    info.namespace = namespace.map(str::to_string);
                }
                if info.enclosing_type.is_none() {
                    info.enclosing_type = enclosing_type.map(str::to_string);
                }
            })
            .or_insert(TypeDeclInfo {
                visibility,
                namespace: namespace.map(str::to_string),
                enclosing_type: enclosing_type.map(str::to_string),
                package: package.map(str::to_string),
            });
    }

    // queue_mgmt NOTE: Root traversal and translation-unit tagging reside here to
    // make queue-focused testing possible.
    pub(super) fn lower_root_items(&mut self, items: &[Item], namespace: Option<&str>) {
        for item in items {
            let unit = if let Some(units) = &self.item_units {
                let unit = units.get(self.root_item_index).copied().unwrap_or(0);
                self.root_item_index += 1;
                unit
            } else {
                self.root_item_index += 1;
                0
            };
            self.lower_item(item, namespace, unit);
        }
    }

    pub(super) fn lower_items<'a>(
        &mut self,
        items: impl Iterator<Item = &'a Item>,
        namespace: Option<&str>,
        unit: usize,
    ) {
        for item in items {
            self.lower_item(item, namespace, unit);
        }
    }

    pub(super) fn record_lowered_function(&mut self, lowered: LoweredFunction) -> String {
        let LoweredFunction {
            function,
            diagnostics,
            constraints,
            nested_functions,
            method_metadata,
            test_metadata,
        } = lowered;
        let mut function = function;
        function.name = self.allocate_internal_name(&function.name);
        let final_name = function.name.clone();
        let function_index = self.functions.len();
        self.functions.push(function);
        self.functions.extend(nested_functions);
        self.diagnostics.extend(diagnostics);
        self.constraints.extend(constraints);
        if let Some(meta) = method_metadata {
            self.register_virtual_method(meta, &final_name);
        }
        if matches!(
            self.functions.get(function_index).map(|func| func.kind),
            Some(FunctionKind::Testcase)
        ) {
            let (namespace_from_name, short_name) =
                crate::mir::TestCaseMetadata::split_namespace(&final_name);
            let resolved_namespace = test_metadata
                .as_ref()
                .and_then(|meta| meta.namespace.clone())
                .or(namespace_from_name);
            let categories = test_metadata
                .as_ref()
                .map(|meta| meta.categories.clone())
                .unwrap_or_default();
            let parameters = test_metadata
                .as_ref()
                .map(|meta| meta.parameters.clone())
                .unwrap_or_default();
            let explicit_id = test_metadata
                .as_ref()
                .and_then(|meta| meta.explicit_id.clone());
            let id = crate::mir::TestCaseMetadata::stable_id(&final_name, explicit_id.as_deref());
            self.test_cases.push(crate::mir::TestCaseMetadata {
                function_index,
                id,
                qualified_name: final_name.clone(),
                name: short_name,
                namespace: resolved_namespace,
                categories,
                parameters,
                is_async: self
                    .functions
                    .get(function_index)
                    .map(|func| func.is_async)
                    .unwrap_or(false),
                span: test_metadata.as_ref().and_then(|meta| meta.span),
            });
        }
        final_name
    }

    pub(super) fn record_perf_attributes(
        &mut self,
        qualified_name: &str,
        attrs: &[crate::frontend::ast::Attribute],
        span: Option<crate::frontend::diagnostics::Span>,
    ) -> Vec<crate::mir::LoweringDiagnostic> {
        let mut diagnostics = Vec::new();
        let (trace_attr, trace_errors) = collect_trace_attribute(attrs);
        diagnostics.extend(convert_attribute_errors(trace_errors));
        let (cost_attr, cost_errors) = collect_cost_attribute(attrs);
        diagnostics.extend(convert_attribute_errors(cost_errors));

        let cost_model = cost_attr.map(|attr| CostModel {
            function: qualified_name.to_string(),
            cpu_budget_us: attr.cpu_budget_us,
            gpu_budget_us: attr.gpu_budget_us,
            mem_budget_bytes: attr.mem_budget_bytes,
            span: attr.span.or(span),
        });
        if let Some(cost) = &cost_model {
            self.perf_metadata.costs.push(cost.clone());
        }

        let should_trace =
            force_profile_tracepoints() || trace_attr.is_some() || cost_model.is_some();
        if should_trace {
            let trace_attr = trace_attr.unwrap_or(crate::frontend::attributes::TraceAttr {
                label: None,
                level: None,
                span,
            });
            let has_label = trace_attr.label.is_some();
            let user_label = trace_attr
                .label
                .clone()
                .unwrap_or_else(|| qualified_name.to_string());
            let label = if has_label {
                format!("{qualified_name}::{user_label}")
            } else {
                user_label
            };
            let level = trace_attr
                .level
                .as_deref()
                .and_then(TraceLevel::from_str)
                .unwrap_or(TraceLevel::Perf);
            let label_id = Some(self.string_interner.intern(
                &label,
                StrLifetime::Static,
                trace_attr.span.or(span),
            ));
            let tracepoint = Tracepoint {
                function: qualified_name.to_string(),
                label: label.clone(),
                label_id,
                level,
                trace_id: trace_id(qualified_name, &label),
                span: trace_attr.span.or(span),
                budget: cost_model,
            };
            self.perf_metadata.tracepoints.push(tracepoint);
        }

        diagnostics
    }

    // queue_mgmt NOTE: Item dispatch now lives alongside the queue utilities so
    // the future pipeline can call into a single abstraction.
    pub(super) fn lower_item(&mut self, item: &Item, namespace: Option<&str>, unit: usize) {
        let previous_package = self.current_package.clone();
        self.current_package = self.unit_packages.get(unit).and_then(|pkg| pkg.clone());

        if matches!(item, Item::Namespace(_)) {
            let nested = if let Item::Namespace(ns) = item {
                qualify(namespace, &ns.name)
            } else {
                unreachable!()
            };
            if let Item::Namespace(ns) = item {
                self.lower_items(ns.items.iter(), Some(&nested), unit);
            }
            return;
        }

        let label = item_label(item, namespace);
        let span = info_span!("lower_item", kind = item_kind(item), item = %label, unit);
        let _guard = span.enter();

        let cache_key = cache_key(item, namespace, unit);
        if let Some(entry) = self.cache.lookup(&cache_key) {
            debug!(
                target: "lowering::cache",
                event = "hit",
                item = %label,
                unit,
                functions = entry.functions.len()
            );
            self.apply_cached_lowering(&entry, unit);
            return;
        }

        debug!(
            target: "lowering::cache",
            event = "miss",
            item = %label,
            unit
        );

        let fn_start = self.functions.len();
        let diag_start = self.diagnostics.len();
        let constraint_start = self.constraints.len();
        let interner_start = self.string_interner.len();

        match item {
            Item::Function(func) => {
                let name = qualify(namespace, &func.name);
                self.collect_exports_for(&name, &func.attributes);
                self.collect_link_library(func.link_library.as_deref());
                let perf_diags = self.record_perf_attributes(&name, &func.attributes, None);
                self.diagnostics.extend(perf_diags);
                self.check_signature(&func.signature, namespace, None, &name);
                let import_resolver = self.import_resolver_for_unit(unit) as *const _;
                let lowered = lower_function(
                    func,
                    &name,
                    FunctionKind::Function,
                    namespace,
                    self.current_package.as_deref(),
                    None,
                    &mut self.type_layouts,
                    &self.type_visibilities,
                    &self.primitive_registry,
                    self.default_arguments.clone(),
                    &self.function_packages,
                    &self.operator_registry,
                    &mut self.string_interner,
                    &self.symbol_index,
                    // SAFETY: The resolver is stored on `self` and we don't mutate the resolver
                    // storage during lowering, so the pointer remains valid for the duration of
                    // this call.
                    unsafe { &*import_resolver },
                    &self.static_registry,
                    &self.class_bases,
                    &self.class_virtual_slots,
                    &self.trait_decls,
                    self.generic_specializations.clone(),
                );
                let _ = self.record_lowered_function(lowered);
            }
            Item::TestCase(test) => {
                let name = qualify(namespace, &test.name);
                if let Some(signature) = &test.signature {
                    self.check_signature(signature, namespace, None, &name);
                }
                let perf_diags = self.record_perf_attributes(&name, &test.attributes, None);
                self.diagnostics.extend(perf_diags);
                let import_resolver = self.import_resolver_for_unit(unit) as *const _;
                let lowered = lower_testcase(
                    test,
                    &name,
                    namespace,
                    self.current_package.as_deref(),
                    &mut self.type_layouts,
                    &self.type_visibilities,
                    &self.primitive_registry,
                    self.default_arguments.clone(),
                    &self.function_packages,
                    &self.operator_registry,
                    &mut self.string_interner,
                    &self.symbol_index,
                    // SAFETY: The resolver is stored on `self` and we don't mutate the resolver
                    // storage during lowering, so the pointer remains valid for the duration of
                    // this call.
                    unsafe { &*import_resolver },
                    &self.static_registry,
                    &self.class_bases,
                    &self.class_virtual_slots,
                    &self.trait_decls,
                    self.generic_specializations.clone(),
                );
                let _ = self.record_lowered_function(lowered);
            }
            Item::Struct(strct) => {
                self.register_struct_layout(strct, namespace);
                self.lower_struct(strct, namespace);
                if !strct.nested_types.is_empty() {
                    let nested_ns = qualify(namespace, &strct.name);
                    self.lower_items(strct.nested_types.iter(), Some(&nested_ns), unit);
                }
            }
            Item::Union(union_def) => self.register_union_layout(union_def, namespace),
            Item::Enum(enm) => self.register_enum_layout(enm, namespace),
            Item::Class(class) => {
                self.register_class_layout(class, namespace);
                self.lower_class(class, namespace);
                if !class.nested_types.is_empty() {
                    let nested_ns = qualify(namespace, &class.name);
                    self.lower_items(class.nested_types.iter(), Some(&nested_ns), unit);
                }
            }
            Item::Interface(iface) => self.lower_interface(iface, namespace),
            Item::Delegate(delegate) => {
                let name = qualify(namespace, &delegate.name);
                if let Some(signature) = self.symbol_index.delegate_signature(&name) {
                    self.type_layouts
                        .record_delegate_signature(name.clone(), signature.clone());
                }
                self.type_layouts.ensure_delegate_layout(&name);
                let enclosing_type = namespace.and_then(|ns| {
                    if self.symbol_index.types.contains(ns) {
                        Some(ns.to_string())
                    } else {
                        None
                    }
                });
                self.type_visibilities.insert(
                    name,
                    TypeDeclInfo {
                        visibility: delegate.visibility,
                        namespace: namespace.map(str::to_string),
                        enclosing_type,
                        package: self.current_package.clone(),
                    },
                );
            }
            Item::Extension(ext) => self.lower_extension(ext, namespace),
            Item::Trait(_) => {}
            Item::Impl(impl_decl) => self.lower_impl(impl_decl, namespace),
            Item::Const(_) | Item::Static(_) => {}
            Item::Import(_) => {}
            Item::TypeAlias(_) => {}
            Item::Namespace(_) => unreachable!(),
        }

        self.current_package = previous_package;

        let fn_end = self.functions.len();
        let diag_end = self.diagnostics.len();
        let constraint_end = self.constraints.len();
        let interner_end = self.string_interner.len();

        if fn_end > fn_start {
            self.unit_slices.push(ModuleUnitSlice {
                unit,
                range: fn_start..fn_end,
            });
        }

        let cached = CachedLowering {
            functions: self.functions[fn_start..fn_end].to_vec(),
            diagnostics: self.diagnostics[diag_start..diag_end].to_vec(),
            constraints: self.constraints[constraint_start..constraint_end].to_vec(),
            interned: self.string_interner.segments()[interner_start..interner_end].to_vec(),
        };
        debug!(
            target: "lowering::cache",
            event = "store",
            item = %label,
            unit,
            functions = cached.functions.len(),
            diagnostics = cached.diagnostics.len(),
            constraints = cached.constraints.len(),
            interned = cached.interned.len()
        );
        self.cache.insert(cache_key, cached);
    }

    fn apply_cached_lowering(&mut self, cached: &CachedLowering, unit: usize) {
        self.string_interner.install_snapshot(&cached.interned);
        let start = self.functions.len();
        if !cached.functions.is_empty() {
            self.functions.extend(cached.functions.iter().cloned());
            self.unit_slices.push(ModuleUnitSlice {
                unit,
                range: start..self.functions.len(),
            });
        }
        self.diagnostics.extend(cached.diagnostics.iter().cloned());
        self.constraints.extend(cached.constraints.iter().cloned());
    }
}

fn convert_attribute_errors(errors: Vec<AttributeError>) -> Vec<crate::mir::LoweringDiagnostic> {
    errors
        .into_iter()
        .map(|error| crate::mir::LoweringDiagnostic {
            message: error.message,
            span: error.span,
        })
        .collect()
}

fn force_profile_tracepoints() -> bool {
    static FORCE: OnceLock<bool> = OnceLock::new();
    *FORCE.get_or_init(|| {
        std::env::var("CHIC_PROFILE_AUTO_TRACE")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    })
}

fn cache_key(item: &Item, namespace: Option<&str>, unit: usize) -> String {
    let digest = hash(format!("{:?}", item).as_bytes());
    format!(
        "ns:{}|unit:{}|{}",
        namespace.unwrap_or_default(),
        unit,
        digest.to_hex()
    )
}

fn item_kind(item: &Item) -> &'static str {
    match item {
        Item::Function(_) => "function",
        Item::Struct(_) => "struct",
        Item::Union(_) => "union",
        Item::Enum(_) => "enum",
        Item::Class(_) => "class",
        Item::Interface(_) => "interface",
        Item::Delegate(_) => "delegate",
        Item::Extension(_) => "extension",
        Item::Trait(_) => "trait",
        Item::Impl(_) => "impl",
        Item::TestCase(_) => "testcase",
        Item::Static(_) => "static",
        Item::Const(_) => "const",
        Item::Import(_) => "import",
        Item::TypeAlias(_) => "typealias",
        Item::Namespace(_) => "namespace",
    }
}

fn item_label(item: &Item, namespace: Option<&str>) -> String {
    match item {
        Item::Function(func) => qualify(namespace, &func.name),
        Item::Struct(strct) => qualify(namespace, &strct.name),
        Item::Union(union_def) => qualify(namespace, &union_def.name),
        Item::Enum(enm) => qualify(namespace, &enm.name),
        Item::Class(class) => qualify(namespace, &class.name),
        Item::Interface(iface) => qualify(namespace, &iface.name),
        Item::Delegate(delegate) => qualify(namespace, &delegate.name),
        Item::Extension(ext) => format!(
            "{}::extension<{:?}>",
            namespace.unwrap_or_default(),
            ext.target
        ),
        Item::Trait(trait_decl) => qualify(namespace, &trait_decl.name),
        Item::Static(item) => {
            let name = item
                .declaration
                .declarators
                .first()
                .map(|decl| decl.name.as_str())
                .unwrap_or("static");
            qualify(namespace, name)
        }
        Item::Impl(impl_decl) => {
            let base = impl_decl
                .trait_ref
                .as_ref()
                .map(|ty| ty.name.as_str())
                .unwrap_or("impl");
            format!("{}::{base}", namespace.unwrap_or_default())
        }
        Item::TestCase(test) => qualify(namespace, &test.name),
        Item::Const(const_decl) => {
            let declarator = const_decl
                .declaration
                .declarators
                .first()
                .map(|decl| decl.name.as_str())
                .unwrap_or("<const>");
            qualify(namespace, declarator)
        }
        Item::TypeAlias(alias) => qualify(namespace, &alias.name),
        Item::Import(using) => match &using.kind {
            UsingKind::Namespace { path } => {
                let prefix = if using.is_global { "global " } else { "" };
                format!("{prefix}import namespace {path}")
            }
            UsingKind::Alias { alias, target } => {
                let prefix = if using.is_global { "global " } else { "" };
                format!("{prefix}import {alias} = {target}")
            }
            UsingKind::Static { target } => {
                let prefix = if using.is_global { "global " } else { "" };
                format!("{prefix}import static {target}")
            }
            UsingKind::CImport { header } => {
                format!("import cimport {header}")
            }
        },
        Item::Namespace(ns) => qualify(namespace, &ns.name),
    }
}
