//! Simple type constraint collection and checking for the bootstrap compiler.
//!
//! # Arena categories
//! - **Signatures** live in [`SignatureArena`], ensuring every method/function lowering
//!   request returns a stable [`SignatureId`].
//! - **Type infos** are stored per name inside [`TypeChecker::types`], covering structs,
//!   classes, interfaces, unions, enums, and traits.
//! - **Trait infos** occupy [`TypeChecker::traits`] and are fed by the registry discovery
//!   code.
//! - **Lifetime diagnostics** live in `arena/lifetimes.rs` and are shared between the
//!   MIR builder and the type checker.
//! - **Arena allocations** are tracked via [`ArenaAllocations`], which enforces per-category
//!   budgets derived from the parsed module to guard against runaway allocations.

use std::collections::{HashMap, HashSet};

use crate::di::{DiManifest, collect_di_manifest};
use crate::frontend::ast::{
    Block, ClassKind, ConstMemberDecl, FnTypeExpr, FunctionDecl, GenericArgument,
    GenericConstraintKind, GenericParam, GenericParamKind, GenericParams, Item, MemberDispatch,
    Module, OperatorKind, PropertyAccessorBody, PropertyAccessorKind, PropertyDecl,
    TraitObjectTypeExpr, TypeExpr, TypeParamData, TypeSuffix, UsingKind, Visibility,
};
use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode, Label, Span};
use crate::frontend::import_resolver::{ImportResolver, Resolution as ImportResolution};
use crate::frontend::type_alias::{TypeAlias, TypeAliasRegistry};
use crate::frontend::type_utils::{type_expr_surface, vector_descriptor};
use crate::mir::{
    AutoTraitOverride, ConstEvalContext, ConstEvalSummary, ParamMode, SymbolIndex, Ty,
    TypeLayoutTable,
    builder::symbol_index::{ConstructorDeclSymbol, FunctionDeclSymbol},
};
use tracing::debug;

use super::diagnostics as typeck_diagnostics;
use super::helpers::{base_type_name, canonical_type_name, strip_receiver, type_names_equivalent};
use super::registry::{RegistryHooks, RegistryIndex};
use super::trait_solver::TraitSolverMetrics;
use crate::typeck::TypeckQueries;

use allocations::{AllocationCategory, ArenaAllocationBudgets, ArenaAllocations};
use typeck_diagnostics::codes;

mod allocations;
mod diagnostics;
mod dispatch;
mod lifetimes;

pub use lifetimes::BorrowEscapeCategory;

/// Constraints generated during MIR lowering to be verified by the type checker.
#[derive(Clone, Debug)]
pub struct TypeConstraint {
    pub kind: ConstraintKind,
    pub span: Option<Span>,
}

impl TypeConstraint {
    #[must_use]
    pub fn new(kind: ConstraintKind, span: Option<Span>) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug)]
pub(super) struct EffectConstraintRecord {
    pub effect: String,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
struct RecordedDefault {
    text: String,
    type_name: String,
    span: Option<Span>,
    function: String,
}

#[derive(Clone, Copy, Debug)]
pub enum AutoTraitKind {
    ThreadSafe,
    Shareable,
    Copy,
}

impl AutoTraitKind {
    pub(super) fn display_name(self) -> &'static str {
        match self {
            AutoTraitKind::ThreadSafe => "ThreadSafe",
            AutoTraitKind::Shareable => "Shareable",
            AutoTraitKind::Copy => "Copy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoTraitConstraintOrigin {
    Generic,
    AsyncSuspend,
    ThreadSpawn,
}

/// Kinds of constraints understood by the type checker.
#[derive(Clone, Debug)]
pub enum ConstraintKind {
    ParameterType {
        function: String,
        param: String,
        ty: String,
    },
    VariableInit {
        function: String,
        name: String,
        declared: Option<String>,
        expr: String,
    },
    ReturnType {
        function: String,
        ty: String,
    },
    ImplTraitBound {
        function: String,
        opaque_ty: String,
        bound: String,
    },
    ImplementsInterface {
        type_name: String,
        interface: String,
    },
    ExtensionTarget {
        extension: String,
        target: String,
    },
    RequiresAutoTrait {
        function: String,
        target: String,
        ty: String,
        trait_kind: AutoTraitKind,
        origin: AutoTraitConstraintOrigin,
    },
    ThreadingBackendAvailable {
        function: String,
        backend: String,
        call: String,
    },
    RandomDuplication {
        function: String,
    },
    EffectEscape {
        function: String,
        effect: String,
    },
    BorrowEscape {
        function: String,
        parameter: String,
        parameter_mode: ParamMode,
        escape: BorrowEscapeCategory,
    },
    RequiresTrait {
        function: String,
        ty: String,
        trait_name: String,
    },
}

/// Recorded async signature metadata for compiled functions.
#[derive(Clone, Default)]
pub struct AsyncSignatureInfo {
    pub name: String,
    pub param_types: Vec<String>,
    pub result: Option<TypeExpr>,
}

/// Result of performing type checking for a module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceDefaultBinding {
    pub implementer: String,
    pub interface: String,
    pub method: String,
    pub symbol: String,
}

#[derive(Default, Clone)]
pub struct TypeCheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub async_signatures: Vec<AsyncSignatureInfo>,
    pub interface_defaults: Vec<InterfaceDefaultBinding>,
    pub trait_solver_metrics: TraitSolverMetrics,
}

#[derive(Clone, Default)]
pub struct TraitFulfillmentReport {
    pub diagnostics: Vec<Diagnostic>,
    pub metrics: TraitSolverMetrics,
}

#[derive(Clone, Default)]
pub struct PackageContext {
    pub item_units: Option<Vec<usize>>,
    pub unit_packages: Vec<Option<String>>,
    pub unit_import_resolvers: Option<Vec<ImportResolver>>,
}

impl PackageContext {
    #[must_use]
    pub fn package_for_unit(&self, unit: Option<usize>) -> Option<&str> {
        let Some(index) = unit else { return None };
        self.unit_packages.get(index).and_then(|pkg| pkg.as_deref())
    }
}

/// Run type constraint validation and trait/interface checks.
#[must_use]
pub fn check_module(
    module: &Module,
    constraints: &[TypeConstraint],
    type_layouts: &TypeLayoutTable,
) -> TypeCheckResult {
    TypeckQueries::new(module, type_layouts).check_module(constraints)
}

/// Run type checking with explicit package context (unit/package mapping).
#[must_use]
pub fn check_module_with_context(
    module: &Module,
    constraints: &[TypeConstraint],
    type_layouts: &TypeLayoutTable,
    package_context: PackageContext,
) -> TypeCheckResult {
    TypeckQueries::new_with_context(module, type_layouts, package_context).check_module(constraints)
}

#[derive(Clone)]
pub(super) struct FunctionSignature {
    pub(super) name: String,
    pub(super) param_types: Vec<String>,
    pub(super) return_type: String,
    pub(super) span: Option<Span>,
}

#[derive(Clone)]
pub(super) struct MethodDispatchInfo {
    pub(super) dispatch: MemberDispatch,
    pub(super) visibility: Visibility,
    pub(super) is_static: bool,
    pub(super) has_body: bool,
    pub(super) span: Option<Span>,
}

#[derive(Clone, Copy)]
pub(super) struct PropertyAccessors {
    get: bool,
    set: bool,
    init: bool,
}

impl PropertyAccessors {
    pub(super) fn from_decl(property: &PropertyDecl) -> Self {
        Self {
            get: property.accessor(PropertyAccessorKind::Get).is_some(),
            set: property.accessor(PropertyAccessorKind::Set).is_some(),
            init: property.accessor(PropertyAccessorKind::Init).is_some(),
        }
    }

    pub(super) fn has_get(self) -> bool {
        self.get
    }

    pub(super) fn has_set(self) -> bool {
        self.set
    }

    pub(super) fn has_init(self) -> bool {
        self.init
    }
}

#[derive(Clone)]
pub(super) struct PropertyInfo {
    pub(super) name: String,
    pub(super) ty: String,
    pub(super) accessors: PropertyAccessors,
    pub(super) is_static: bool,
    pub(super) span: Option<Span>,
    pub(super) accessor_details: Vec<PropertyAccessorInfo>,
}

#[derive(Clone)]
pub(super) struct PropertyAccessorInfo {
    pub(super) kind: PropertyAccessorKind,
    pub(super) dispatch: MemberDispatch,
    pub(super) visibility: Visibility,
    pub(super) span: Option<Span>,
    pub(super) has_body: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MethodKey {
    name: String,
    params: Vec<String>,
    generic_arity: usize,
}

impl MethodKey {
    fn new(signature: &FunctionSignature, generic_arity: usize) -> Self {
        Self {
            name: strip_receiver(&signature.name).to_string(),
            params: signature.param_types.clone(),
            generic_arity,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PropertyAccessorKey {
    property: String,
    kind: PropertyAccessorKind,
}

impl PropertyAccessorKey {
    fn new(property: &str, kind: PropertyAccessorKind) -> Self {
        Self {
            property: property.to_string(),
            kind,
        }
    }
}

#[derive(Clone)]
struct MethodRecord {
    owner: String,
    signature_id: SignatureId,
    dispatch: MemberDispatch,
    visibility: Visibility,
    is_static: bool,
    span: Option<Span>,
}

#[derive(Clone)]
struct PropertyAccessorRecord {
    owner: String,
    property_type: String,
    dispatch: MemberDispatch,
    is_static: bool,
    span: Option<Span>,
    visibility: Visibility,
}

#[derive(Default)]
struct InheritedMembers {
    methods: HashMap<MethodKey, MethodRecord>,
    accessors: HashMap<PropertyAccessorKey, PropertyAccessorRecord>,
}

impl PropertyInfo {
    pub(super) fn from_decl(property: &PropertyDecl) -> Self {
        let base_dispatch = property.dispatch;
        let mut accessor_details = Vec::new();
        for accessor in &property.accessors {
            let effective_dispatch = if accessor.dispatch.is_virtual
                || accessor.dispatch.is_override
                || accessor.dispatch.is_sealed
                || accessor.dispatch.is_abstract
            {
                accessor.dispatch
            } else {
                base_dispatch
            };
            let has_body = matches!(
                accessor.body,
                PropertyAccessorBody::Block(_) | PropertyAccessorBody::Expression(_)
            );
            accessor_details.push(PropertyAccessorInfo {
                kind: accessor.kind,
                dispatch: effective_dispatch,
                visibility: accessor.visibility.unwrap_or(property.visibility),
                span: accessor.span.or(property.span),
                has_body,
            });
        }
        Self {
            name: property.name.clone(),
            ty: type_expr_surface(&property.ty),
            accessors: PropertyAccessors::from_decl(property),
            is_static: property.is_static,
            span: property.span,
            accessor_details,
        }
    }
}

#[derive(Clone)]
pub(super) struct InterfaceDefaultProvider {
    pub(super) method: String,
    pub(super) symbol: String,
    pub(super) kind: InterfaceDefaultKind,
    pub(super) conditions: Vec<String>,
    pub(super) span: Option<Span>,
    pub(super) origin: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InterfaceDefaultKind {
    Inline,
    Extension,
}

#[derive(Clone)]
pub(super) struct ConstructorInfo {
    pub(super) visibility: Visibility,
    pub(super) param_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SignatureId(usize);

#[derive(Default)]
pub(super) struct SignatureArena {
    storage: Vec<FunctionSignature>,
}

impl SignatureArena {
    pub(super) fn alloc(&mut self, signature: FunctionSignature) -> SignatureId {
        let id = SignatureId(self.storage.len());
        self.storage.push(signature);
        id
    }

    pub(super) fn get(&self, id: SignatureId) -> &FunctionSignature {
        &self.storage[id.0]
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) enum TypeKind {
    Struct {
        constructors: Vec<ConstructorInfo>,
        is_record: bool,
        bases: Vec<BaseTypeBinding>,
    },
    Union,
    Class {
        methods: Vec<SignatureId>,
        bases: Vec<BaseTypeBinding>,
        kind: ClassKind,
        properties: Vec<PropertyInfo>,
        constructors: Vec<ConstructorInfo>,
        is_abstract: bool,
        is_sealed: bool,
        is_static: bool,
    },
    Interface {
        methods: Vec<SignatureId>,
        properties: Vec<PropertyInfo>,
        bases: Vec<BaseTypeBinding>,
    },
    Delegate {
        _signature: FunctionSignature,
    },
    Enum,
    Trait,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct BaseTypeBinding {
    pub(super) name: String,
    pub(super) expr: TypeExpr,
}

impl BaseTypeBinding {
    pub(super) fn new(name: String, expr: TypeExpr) -> Self {
        Self { name, expr }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct TypeInfo {
    pub(super) kind: TypeKind,
    pub(super) generics: Option<GenericParams>,
    pub(super) repr_c: bool,
    pub(super) packing: Option<u32>,
    pub(super) align: Option<u32>,
    #[allow(dead_code)]
    pub(super) is_readonly: bool,
    #[allow(dead_code)]
    pub(super) is_intrinsic: bool,
    pub(super) visibility: Visibility,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct TraitInfo {
    pub(super) methods: Vec<TraitMethodInfo>,
    pub(super) associated_types: Vec<TraitAssociatedTypeInfo>,
    pub(super) consts: Vec<ConstMemberDecl>,
    pub(super) generics: Option<GenericParams>,
    pub(super) super_traits: Vec<TypeExpr>,
    pub(super) object_safety: TraitObjectSafety,
    pub(super) auto_trait_overrides: AutoTraitOverride,
    pub(super) span: Option<Span>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct TraitMethodInfo {
    pub(super) name: String,
    pub(super) signature: SignatureId,
    pub(super) has_body: bool,
    pub(super) is_async: bool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct TraitAssociatedTypeInfo {
    pub(super) name: String,
    pub(super) generics: Option<GenericParams>,
    pub(super) default: Option<TypeExpr>,
}

#[derive(Clone, Default)]
pub(super) struct TraitObjectSafety {
    violations: Vec<ObjectSafetyViolation>,
}

impl TraitObjectSafety {
    pub(super) fn record(&mut self, violation: ObjectSafetyViolation) {
        self.violations.push(violation);
    }

    pub(super) fn is_object_safe(&self) -> bool {
        self.violations.is_empty()
    }

    pub(super) fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub(super) fn first_violation(&self) -> Option<&ObjectSafetyViolation> {
        self.violations.first()
    }

    pub(super) fn describe(&self) -> Option<String> {
        let violation = self.first_violation()?;
        let mut message = violation.describe();
        let remaining = self.violation_count().saturating_sub(1);
        if remaining > 0 {
            message.push_str(&format!(
                " (+{} more issue{})",
                remaining,
                if remaining == 1 { "" } else { "s" }
            ));
        }
        Some(message)
    }

    pub(super) fn violation_span(&self) -> Option<Span> {
        self.first_violation().and_then(|violation| violation.span)
    }
}

#[derive(Clone)]
pub(super) struct ObjectSafetyViolation {
    pub(super) kind: ObjectSafetyViolationKind,
    pub(super) member: String,
    pub(super) span: Option<Span>,
}

impl ObjectSafetyViolation {
    fn describe(&self) -> String {
        match self.kind {
            ObjectSafetyViolationKind::ReturnsSelf => {
                format!("method `{}` returns `Self`", self.member)
            }
            ObjectSafetyViolationKind::GenericMethod => {
                format!(
                    "method `{}` declares its own generic parameters",
                    self.member
                )
            }
            ObjectSafetyViolationKind::MissingAssociatedTypeDefault => {
                format!("associated type `{}` is missing a default", self.member)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ObjectSafetyViolationKind {
    ReturnsSelf,
    GenericMethod,
    MissingAssociatedTypeDefault,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(super) struct ImplInfo {
    pub(super) trait_name: Option<String>,
    pub(super) target: TypeExpr,
    pub(super) generics: Option<GenericParams>,
    pub(super) span: Option<Span>,
}

#[derive(Clone)]
pub(super) struct OperatorSignatureInfo {
    pub(super) kind: OperatorKind,
    #[allow(dead_code)]
    pub(super) return_type: String,
    pub(super) span: Option<Span>,
}

pub(super) struct PendingBody<'a> {
    pub(super) name: String,
    pub(super) body: &'a Block,
    pub(super) namespace: Option<String>,
    pub(super) context_type: Option<String>,
}

pub(super) struct TypeChecker<'a> {
    pub(super) module: &'a Module,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) symbol_index: SymbolIndex,
    pub(super) types: HashMap<String, Vec<TypeInfo>>,
    pub(super) traits: HashMap<String, TraitInfo>,
    pub(super) impls: Vec<ImplInfo>,
    interface_default_providers: HashMap<String, HashMap<String, Vec<InterfaceDefaultProvider>>>,
    interface_default_bindings: Vec<InterfaceDefaultBinding>,
    pending_generics: HashMap<String, Vec<GenericParam>>,
    pub(super) function_generics: HashMap<String, Vec<GenericParam>>,
    pub(super) local_function_ordinals: HashMap<String, usize>,
    pub(super) functions: HashMap<String, Vec<SignatureId>>,
    // Track method overloads per type
    pub(super) methods: HashMap<String, Vec<SignatureId>>,
    pub(super) signatures: SignatureArena,
    pub(super) method_dispatch: HashMap<SignatureId, MethodDispatchInfo>,
    pub(super) signature_generics: HashMap<SignatureId, Vec<GenericParam>>,
    pub(super) builtin_types: HashSet<String>,
    pub(super) async_signatures: HashMap<SignatureId, Option<TypeExpr>>,
    pub(super) declared_effects: HashMap<String, Vec<String>>,
    pub(super) inferred_effects: HashMap<String, Vec<EffectConstraintRecord>>,
    pub(super) type_layouts: &'a TypeLayoutTable,
    pub(super) di_manifest: DiManifest,
    pub(super) import_resolver: ImportResolver,
    pub(super) type_aliases: TypeAliasRegistry,
    pub(super) trait_solver_metrics: TraitSolverMetrics,
    pub(super) registry_hooks: RegistryHooks,
    pub(super) registry_index: RegistryIndex,
    pub(super) package_context: PackageContext,
    pub(super) current_unit: Option<usize>,
    pub(super) current_package: Option<String>,
    pub(super) pending_bodies: Vec<PendingBody<'a>>,
    pub(super) type_packages: HashMap<String, String>,
    pub(super) function_packages: HashMap<String, String>,
    allocations: ArenaAllocations,
    pub(super) enclosing_types: Vec<String>,
    pub(super) operator_signatures: HashMap<String, Vec<OperatorSignatureInfo>>,
}

impl<'a> TypeChecker<'a> {
    #[allow(dead_code)]
    pub(super) fn new(module: &'a Module, type_layouts: &'a TypeLayoutTable) -> Self {
        Self::new_with_context(module, type_layouts, PackageContext::default())
    }

    pub(super) fn new_with_context(
        module: &'a Module,
        type_layouts: &'a TypeLayoutTable,
        package_context: PackageContext,
    ) -> Self {
        let mut builtin_types: HashSet<String> = [
            "bool", "byte", "sbyte", "short", "ushort", "int", "uint", "long", "ulong", "char",
            "float", "double", "decimal", "string", "str", "void", "usize", "isize", "nint",
            "nuint", "Rc", "Arc", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64",
            "i128", "f32", "f64", "vector",
        ]
        .iter()
        .map(|name| name.to_string())
        .collect();
        for desc in type_layouts.primitive_registry.descriptors() {
            builtin_types.insert(desc.primitive_name.clone());
            for alias in &desc.aliases {
                builtin_types.insert(alias.clone());
            }
        }
        let type_aliases = TypeAliasRegistry::collect(module);
        let allocation_budgets = ArenaAllocationBudgets::from_module(module);
        let symbol_index = SymbolIndex::build(module);
        Self {
            module,
            diagnostics: Vec::new(),
            symbol_index,
            types: HashMap::new(),
            traits: HashMap::new(),
            impls: Vec::new(),
            interface_default_providers: HashMap::new(),
            interface_default_bindings: Vec::new(),
            pending_generics: HashMap::new(),
            function_generics: HashMap::new(),
            local_function_ordinals: HashMap::new(),
            functions: HashMap::new(),
            methods: HashMap::new(),
            signatures: SignatureArena::default(),
            method_dispatch: HashMap::new(),
            signature_generics: HashMap::new(),
            builtin_types,
            async_signatures: HashMap::new(),
            declared_effects: HashMap::new(),
            inferred_effects: HashMap::new(),
            type_layouts,
            di_manifest: collect_di_manifest(module),
            import_resolver: ImportResolver::build(module),
            type_aliases,
            trait_solver_metrics: TraitSolverMetrics::default(),
            registry_hooks: RegistryHooks::default(),
            registry_index: RegistryIndex::default(),
            package_context,
            current_unit: None,
            current_package: None,
            pending_bodies: Vec::new(),
            type_packages: HashMap::new(),
            function_packages: HashMap::new(),
            allocations: ArenaAllocations::with_budgets(allocation_budgets),
            enclosing_types: Vec::new(),
            operator_signatures: HashMap::new(),
        }
    }

    fn import_resolver_for_current_unit(&self) -> &ImportResolver {
        if let Some(resolvers) = self.package_context.unit_import_resolvers.as_ref()
            && let Some(unit) = self.current_unit
            && let Some(resolver) = resolvers.get(unit)
        {
            return resolver;
        }
        &self.import_resolver
    }

    pub(super) fn emit_error(
        &mut self,
        code: &'static str,
        span: Option<Span>,
        message: impl Into<String>,
    ) {
        diagnostics::emit_error_with_spec_link(&mut self.diagnostics, code, span, message);
    }

    pub(super) fn emit_warning(
        &mut self,
        code: &'static str,
        span: Option<Span>,
        message: impl Into<String>,
    ) {
        diagnostics::emit_warning_with_spec_link(&mut self.diagnostics, code, span, message);
    }

    pub(super) fn record_type_package(&mut self, name: &str) {
        if let Some(pkg) = self.current_package.clone() {
            self.type_packages.entry(name.to_string()).or_insert(pkg);
        }
    }

    pub(super) fn record_function_package(&mut self, name: &str) {
        if let Some(pkg) = self.current_package.clone() {
            self.function_packages
                .entry(name.to_string())
                .or_insert(pkg);
        }
    }

    pub(super) fn package_of_owner(&self, owner: &str) -> Option<&str> {
        self.type_packages
            .get(owner)
            .or_else(|| self.function_packages.get(owner))
            .map(String::as_str)
    }

    pub(super) fn context_package(&self, context_type: Option<&str>) -> Option<&str> {
        if let Some(ctx) = context_type {
            if let Some(pkg) = self.package_of_owner(ctx) {
                return Some(pkg);
            }
        }
        self.current_package.as_deref()
    }

    #[cfg(test)]
    pub(super) fn registry_hooks_mut(&mut self) -> &mut RegistryHooks {
        &mut self.registry_hooks
    }

    pub(super) fn method_is_static(method: &FunctionDecl) -> bool {
        method
            .modifiers
            .iter()
            .any(|modifier| modifier.eq_ignore_ascii_case("static"))
    }

    pub(super) fn insert_type_info(&mut self, name: String, info: TypeInfo) {
        self.allocations.record(AllocationCategory::TypeInfos);
        self.types.entry(name.clone()).or_default().push(info);
        self.pending_generics.remove(&name);
    }

    pub(super) fn insert_trait_info(&mut self, name: String, info: TraitInfo) {
        self.allocations.record(AllocationCategory::TraitInfos);
        self.traits.insert(name, info);
    }

    pub(super) fn allocate_signature(&mut self, signature: FunctionSignature) -> SignatureId {
        self.allocations.record(AllocationCategory::Signatures);
        self.signatures.alloc(signature)
    }

    pub(super) fn push_pending_generics(&mut self, name: &str, generics: Option<&GenericParams>) {
        let Some(params) = generics else {
            return;
        };
        if params.params.is_empty() {
            return;
        }
        self.pending_generics
            .insert(name.to_string(), params.params.clone());
    }

    pub(super) fn pop_pending_generics(&mut self, name: &str) {
        self.pending_generics.remove(name);
    }

    pub(super) fn register_function_generics(
        &mut self,
        name: &str,
        generics: Option<&GenericParams>,
    ) {
        let Some(params) = generics else {
            return;
        };
        if params.params.is_empty() {
            return;
        }
        self.function_generics
            .insert(name.to_string(), params.params.clone());
    }

    pub(super) fn record_signature_generics(
        &mut self,
        id: SignatureId,
        generics: Option<&GenericParams>,
    ) {
        let Some(params) = generics else {
            return;
        };
        if params.params.is_empty() {
            return;
        }
        self.signature_generics.insert(id, params.params.clone());
    }

    pub(super) fn pending_generics_contain(&self, name: &str, candidate: &str) -> bool {
        self.pending_generics
            .get(name)
            .is_some_and(|params| params.iter().any(|param| param.name == candidate))
    }

    pub(super) fn function_generics_contain(&self, name: &str, candidate: &str) -> bool {
        self.function_generics
            .get(name)
            .is_some_and(|params| params.iter().any(|param| param.name == candidate))
    }

    pub(super) fn generic_param_in_owner(
        &self,
        owner: &str,
        candidate: &str,
    ) -> Option<&GenericParam> {
        if let Some(params) = self.pending_generics.get(owner) {
            if let Some(param) = params.iter().find(|param| param.name == candidate) {
                return Some(param);
            }
        }
        if let Some(params) = self.function_generics.get(owner) {
            if let Some(param) = params.iter().find(|param| param.name == candidate) {
                return Some(param);
            }
        }
        if let Some(entries) = self.types.get(owner) {
            for info in entries {
                if let Some(generics) = &info.generics {
                    if let Some(param) =
                        generics.params.iter().find(|param| param.name == candidate)
                    {
                        return Some(param);
                    }
                }
            }
        }
        None
    }

    pub(super) fn register_interface_default_provider(
        &mut self,
        interface: &str,
        provider: InterfaceDefaultProvider,
    ) {
        self.interface_default_providers
            .entry(interface.to_string())
            .or_default()
            .entry(provider.method.clone())
            .or_default()
            .push(provider);
    }

    fn record_interface_default_binding(
        &mut self,
        implementer: &str,
        interface: &str,
        method: &str,
        symbol: &str,
    ) {
        self.interface_default_bindings
            .push(InterfaceDefaultBinding {
                implementer: implementer.to_string(),
                interface: interface.to_string(),
                method: method.to_string(),
                symbol: symbol.to_string(),
            });
    }

    pub(super) fn is_interface(&self, name: &str) -> bool {
        self.resolve_type_info(name)
            .is_some_and(|info| matches!(info.kind, TypeKind::Interface { .. }))
    }

    pub(super) fn try_apply_interface_default(
        &mut self,
        implementer: &str,
        interface: &str,
        method: &str,
        implemented_interfaces: &HashSet<String>,
        span: Option<Span>,
    ) -> bool {
        let Some(methods) = self.interface_default_providers.get(interface) else {
            return false;
        };
        let Some(providers) = methods.get(method) else {
            return false;
        };
        if let Some(provider) = providers
            .iter()
            .find(|p| matches!(p.kind, InterfaceDefaultKind::Inline))
            .cloned()
        {
            self.record_interface_default_binding(implementer, interface, method, &provider.symbol);
            return true;
        }
        let applicable: Vec<InterfaceDefaultProvider> = providers
            .iter()
            .filter(|p| {
                matches!(p.kind, InterfaceDefaultKind::Extension)
                    && p.conditions
                        .iter()
                        .all(|cond| implemented_interfaces.contains(cond))
            })
            .cloned()
            .collect();
        match applicable.len() {
            0 => false,
            1 => {
                let provider = &applicable[0];
                self.record_interface_default_binding(
                    implementer,
                    interface,
                    method,
                    &provider.symbol,
                );
                true
            }
            _ => {
                self.emit_error(
                    codes::DEFAULT_AMBIGUITY,
                    span.or_else(|| applicable.first().and_then(|entry| entry.span)),
                    format!(
                        "multiple default implementations of `{interface}::{method}` apply to `{implementer}`"
                    ),
                );
                for entry in applicable {
                    self.diagnostics.push(typeck_diagnostics::note(
                        format!("candidate `{}` from {}", entry.symbol, entry.origin),
                        entry.span,
                    ));
                }
                false
            }
        }
    }

    pub(super) fn resolve_type_for_expr(
        &mut self,
        ty: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> ImportResolution {
        let mut alias_stack = Vec::new();
        let mut current = ty.clone();
        loop {
            let before = type_expr_surface(&current);
            match self.try_expand_alias(&current, namespace, context_type, &mut alias_stack) {
                Some(expanded) => {
                    let after = type_expr_surface(&expanded);
                    current = expanded;
                    if after == before {
                        break;
                    }
                }
                None => break,
            }
        }
        if let Some(canonical) = self.handle_vector_type(&current) {
            return ImportResolution::Found(canonical);
        }
        enum Canonicalized {
            None,
            One(String),
            Many(Vec<String>),
        }

        let canonicalize = |candidate: &str| -> Canonicalized {
            if self.symbol_index.contains_type(candidate) {
                return Canonicalized::One(candidate.to_string());
            }

            let mut matches: Vec<String> = Vec::new();
            for existing in self.symbol_index.types() {
                if let Some(short) = existing.rsplit("::").next() {
                    if short == candidate && !matches.iter().any(|other| other == existing) {
                        matches.push(existing.clone());
                    }
                }
            }
            matches.sort();
            matches.dedup();

            if matches.len() == 1 {
                return Canonicalized::One(matches.remove(0));
            }
            if !matches.is_empty() {
                return Canonicalized::Many(matches);
            }

            if let Some(desc) = self
                .type_layouts
                .primitive_registry
                .descriptor_for_name(candidate)
            {
                return Canonicalized::One(
                    desc.std_wrapper_type
                        .clone()
                        .unwrap_or_else(|| desc.primitive_name.clone()),
                );
            }

            Canonicalized::None
        };
        if current.fn_signature().is_some() {
            if let Ty::Fn(fn_ty) = Ty::from_type_expr(&current) {
                return ImportResolution::Found(fn_ty.canonical_name());
            }
        }

        if current.pointer_depth() > 0 {
            let mut element = current.clone();
            element
                .suffixes
                .retain(|suffix| !matches!(suffix, TypeSuffix::Pointer { .. }));
            if element.pointer_depth() < current.pointer_depth() {
                return self.resolve_type_for_expr(&element, namespace, context_type);
            }
        }

        let segments = self.alias_base_segments(&current).unwrap_or_default();
        if segments.is_empty() {
            return ImportResolution::NotFound;
        }

        match self.import_resolver_for_current_unit().resolve_type(
            &segments,
            namespace,
            context_type,
            |candidate| matches!(canonicalize(candidate), Canonicalized::One(_)),
        ) {
            ImportResolution::Found(name) => match canonicalize(&name) {
                Canonicalized::One(actual) => ImportResolution::Found(actual),
                Canonicalized::Many(candidates) => ImportResolution::Ambiguous(candidates),
                Canonicalized::None => ImportResolution::NotFound,
            },
            ImportResolution::Ambiguous(candidates) => {
                let mut resolved: Vec<String> = Vec::new();
                for candidate in candidates {
                    match canonicalize(&candidate) {
                        Canonicalized::One(actual) => {
                            if !resolved.iter().any(|existing| existing == &actual) {
                                resolved.push(actual);
                            }
                        }
                        Canonicalized::Many(many) => {
                            for actual in many {
                                if !resolved.iter().any(|existing| existing == &actual) {
                                    resolved.push(actual);
                                }
                            }
                        }
                        Canonicalized::None => {}
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

    /// Validate and canonicalise a `vector<T, N>` type expression.
    pub(super) fn handle_vector_type(&mut self, expr: &TypeExpr) -> Option<String> {
        let Some(descriptor) = vector_descriptor(expr) else {
            return None;
        };
        let element_name = canonical_type_name(descriptor.element);
        let lane_text = descriptor.lanes.expression().text.trim();
        let lane_span = descriptor.lanes_span.or(expr.span);

        let mut lanes = None;
        let normalised_lane = lane_text.replace('_', "");
        match normalised_lane.parse::<u32>() {
            Ok(value) if value > 0 => lanes = Some(value),
            _ => self.emit_error(
                codes::SIMD_LANES_CONST,
                lane_span,
                "SIMD lane count must be a positive compile-time integer",
            ),
        }

        let element_bits = self.vector_element_bits(&element_name);
        if element_bits.is_none() {
            self.emit_error(
                codes::SIMD_ELEMENT_UNSUPPORTED,
                descriptor.element.span.or(expr.span),
                format!(
                    "element type `{}` is not supported for SIMD vectors; allowed types are bool, i8/u8, i16/u16, i32/u32, i64/u64, f32, and f64",
                    element_name
                ),
            );
        }

        if let (Some(lanes), Some(bits_per_lane)) = (lanes, element_bits) {
            if let Some(width) = lanes.checked_mul(bits_per_lane) {
                if width != 64 && width != 128 && width != 256 {
                    self.emit_error(
                        codes::SIMD_WIDTH_UNSUPPORTED,
                        lane_span,
                        format!(
                            "vector<{}, {}> has width {} bits; supported widths are 64, 128, or 256 bits",
                            element_name, lanes, width
                        ),
                    );
                }
            } else {
                self.emit_error(
                    codes::SIMD_WIDTH_UNSUPPORTED,
                    lane_span,
                    format!(
                        "vector<{}, {}> width overflows supported SIMD ranges",
                        element_name, lanes
                    ),
                );
            }
        }

        Some(format!("vector<{element_name}, {lane_text}>"))
    }

    fn vector_element_bits(&self, name: &str) -> Option<u32> {
        let simple = typeck_diagnostics::simple_name(name);
        match simple {
            "bool" | "Boolean" => Some(8),
            "byte" | "u8" | "uint8" | "UInt8" | "Std::Byte" | "System.Byte" => Some(8),
            "sbyte" | "i8" | "int8" | "Int8" | "Std::SByte" | "System.SByte" => Some(8),
            "short" | "i16" | "int16" | "Int16" | "Std::Int16" | "System.Int16" => Some(16),
            "ushort" | "u16" | "uint16" | "UInt16" | "Std::UInt16" | "System.UInt16" => Some(16),
            "int" | "i32" | "int32" | "Int32" | "Std::Int32" | "System.Int32" => Some(32),
            "uint" | "u32" | "uint32" | "UInt32" | "Std::UInt32" | "System.UInt32" => Some(32),
            "long" | "i64" | "int64" | "Int64" | "Std::Int64" | "System.Int64" => Some(64),
            "ulong" | "u64" | "uint64" | "UInt64" | "Std::UInt64" | "System.UInt64" => Some(64),
            "float" | "f32" | "Single" | "Std::Float" | "System.Single" => Some(32),
            "double" | "f64" | "Std::Double" | "System.Double" => Some(64),
            _ => None,
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

    fn try_expand_alias(
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
        let resolution = self.import_resolver_for_current_unit().resolve_type(
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
                    self.emit_error(
                        codes::TYPE_ALIAS_CYCLE,
                        span,
                        format!("type alias `{name}` forms a cycle"),
                    );
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
                self.emit_error(
                    codes::AMBIGUOUS_TYPE,
                    expr.span,
                    format!(
                        "type `{}` resolves to multiple aliases: {}",
                        expr.name,
                        candidates.join(", ")
                    ),
                );
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
            self.emit_error(
                codes::TYPE_NOT_GENERIC,
                expr.generic_span.or(expr.span),
                format!("type alias `{}` is not generic", alias.name),
            );
            return None;
        }
        if !params.is_empty() && params.len() != args.len() {
            self.emit_error(
                codes::GENERIC_ARGUMENT_MISMATCH,
                expr.generic_span.or(expr.span),
                format!(
                    "type alias `{}` expects {} type argument{}, but {} {} supplied",
                    alias.name,
                    params.len(),
                    if params.len() == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
            );
            return None;
        }

        let mut map = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            if param.as_const().is_some() {
                self.emit_error(
                    codes::TYPE_ALIAS_CONST_PARAM,
                    param.span.or(expr.span),
                    "const generic parameters are not supported on type aliases",
                );
                continue;
            }
            let Some(arg_ty) = arg.ty() else {
                self.emit_error(
                    codes::GENERIC_ARGUMENT_MISMATCH,
                    expr.generic_span.or(expr.span),
                    format!(
                        "type alias `{}` expects a type argument for `{}`, but a const expression was supplied",
                        alias.name, param.name
                    ),
                );
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

    fn verify_effects(&mut self) {
        let function_names: Vec<String> = self.inferred_effects.keys().cloned().collect();
        for function in function_names {
            let inferred = self
                .inferred_effects
                .get(&function)
                .cloned()
                .unwrap_or_default();
            let declared = self
                .declared_effects
                .get(&function)
                .cloned()
                .unwrap_or_default();
            for constraint in inferred {
                if self.effect_declared(&declared, &constraint.effect) {
                    continue;
                }
                let (code, message) = if constraint.effect.eq_ignore_ascii_case("random") {
                    (
                        codes::RANDOM_EFFECT_MISSING,
                        format!(
                            "function `{function}` uses randomness but is missing an `effects(random)` declaration"
                        ),
                    )
                } else if constraint.effect.eq_ignore_ascii_case("network") {
                    (
                        codes::NETWORK_EFFECT_MISSING,
                        format!(
                            "function `{function}` uses networking but is missing an `effects(network)` declaration"
                        ),
                    )
                } else {
                    (
                        codes::EFFECT_NOT_DECLARED,
                        format!(
                            "function `{function}` may throw `{}` but its signature does not declare a compatible `throws` clause",
                            constraint.effect
                        ),
                    )
                };
                if code == codes::EFFECT_NOT_DECLARED {
                    self.emit_warning(code, constraint.span, message);
                } else {
                    self.emit_error(code, constraint.span, message);
                }
            }
        }
    }

    fn effect_declared(&self, declared: &[String], actual: &str) -> bool {
        if declared.is_empty() {
            return false;
        }
        declared
            .iter()
            .any(|candidate| self.effect_matches(candidate, actual))
    }

    pub(super) fn report_borrow_escape(
        &mut self,
        function: &str,
        parameter: &str,
        mode: ParamMode,
        escape: &BorrowEscapeCategory,
        span: Option<Span>,
    ) {
        diagnostics::report_borrow_escape(
            &mut self.diagnostics,
            function,
            parameter,
            mode,
            escape,
            span,
        );
    }

    fn effect_matches(&self, declared: &str, actual: &str) -> bool {
        if declared == actual {
            return true;
        }
        if type_names_equivalent(declared, actual) {
            return true;
        }
        self.effect_inherits(actual, declared)
    }

    fn effect_inherits(&self, child: &str, parent: &str) -> bool {
        if child == parent {
            return true;
        }
        let mut stack = vec![child.to_string()];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if current == parent {
                return true;
            }
            let Some(info) = self.type_layouts.class_layout_info(&current) else {
                continue;
            };
            for base in info.bases {
                if base == parent {
                    return true;
                }
                stack.push(base);
            }
        }
        false
    }

    fn register_items(&mut self) {
        self.validate_import_conflicts();
        self.validate_type_aliases();
        if let Some(units) = self.package_context.item_units.as_ref() {
            debug_assert_eq!(
                units.len(),
                self.module.items.len(),
                "item_units length should match module items"
            );
        }
        self.visit_items_with_units(&self.module.items, self.module.namespace.as_deref());
        self.validate_default_argument_conflicts();
        self.run_pending_body_validations();
        if std::env::var_os("CHIC_DEBUG_NS_TYPES").is_some() {
            eprintln!(
                "[chic-debug] register_items populated {} types",
                self.types.len()
            );
        }
    }

    fn validate_import_conflicts(&mut self) {
        let mut global_aliases: HashMap<String, (String, Option<Span>)> = HashMap::new();
        self.collect_global_aliases(&self.module.items, &mut global_aliases);
        self.validate_local_alias_conflicts(&self.module.items, &global_aliases);
    }

    fn collect_global_aliases(
        &mut self,
        items: &[Item],
        aliases: &mut HashMap<String, (String, Option<Span>)>,
    ) {
        for item in items {
            match item {
                Item::Import(using) if using.is_global => {
                    if let UsingKind::Alias { alias, target } = &using.kind {
                        if let Some((existing, existing_span)) = aliases.get(alias) {
                            if !existing.eq_ignore_ascii_case(target) {
                                self.emit_conflicting_alias(
                                    alias,
                                    using.span,
                                    *existing_span,
                                    true,
                                );
                            }
                        } else {
                            aliases.insert(alias.clone(), (target.clone(), using.span));
                        }
                    }
                }
                Item::Namespace(ns) => {
                    self.collect_global_aliases(&ns.items, aliases);
                }
                _ => {}
            }
        }
    }

    fn validate_type_aliases(&mut self) {
        for (name, entries) in self.type_aliases.duplicates() {
            if entries.len() < 2 {
                continue;
            }
            let mut diagnostic = typeck_diagnostics::error(
                codes::TYPE_ALIAS_CONFLICT,
                format!("type alias `{name}` conflicts with a previous declaration"),
                entries
                    .last()
                    .and_then(|alias| alias.span)
                    .or_else(|| entries.first().and_then(|alias| alias.span)),
            );
            if let Some(previous) = entries.first() {
                diagnostic.add_note("previous type alias declared here");
                if let Some(span) = previous.span {
                    diagnostic = diagnostic
                        .with_secondary(Label::secondary(span, format!("previous `{name}` alias")));
                }
            }
            self.diagnostics.push(diagnostic);
        }

        let aliases: Vec<_> = self
            .type_aliases
            .iter()
            .map(|(_, alias)| alias.clone())
            .collect();
        for alias in aliases {
            self.validate_generics(&alias.name, alias.generics.as_ref());
            if let Some(generics) = alias.generics.as_ref() {
                for param in &generics.params {
                    if param.as_const().is_some() {
                        self.emit_error(
                            codes::TYPE_ALIAS_CONST_PARAM,
                            param.span,
                            format!(
                                "const generic parameters are not supported on type alias `{}`",
                                alias.name
                            ),
                        );
                    }
                }
            }
            self.ensure_type_expr(&alias.target, alias.namespace.as_deref(), None, alias.span);
        }
    }

    fn validate_local_alias_conflicts(
        &mut self,
        items: &[Item],
        global_aliases: &HashMap<String, (String, Option<Span>)>,
    ) {
        for item in items {
            match item {
                Item::Import(using) if !using.is_global => {
                    if let UsingKind::Alias { alias, target } = &using.kind {
                        if let Some((existing, existing_span)) = global_aliases.get(alias) {
                            if !existing.eq_ignore_ascii_case(target) {
                                self.emit_conflicting_alias(
                                    alias,
                                    using.span,
                                    *existing_span,
                                    false,
                                );
                            }
                        }
                    }
                }
                Item::Namespace(ns) => {
                    self.validate_local_alias_conflicts(&ns.items, global_aliases);
                }
                _ => {}
            }
        }
    }

    fn emit_conflicting_alias(
        &mut self,
        alias: &str,
        span: Option<Span>,
        previous: Option<Span>,
        previous_global: bool,
    ) {
        let mut diagnostic = Diagnostic::error(
            format!("alias `{alias}` conflicts with existing alias"),
            span,
        )
        .with_code(DiagnosticCode::new("E0G03", Some("import".to_string())));
        if let Some(previous_span) = previous {
            let scope = if previous_global {
                "global import"
            } else {
                "import"
            };
            diagnostic.add_note(format!("previous {scope} declared here"));
            diagnostic = diagnostic.with_secondary(Label::secondary(
                previous_span,
                format!("{scope} for `{alias}`"),
            ));
        }
        self.diagnostics.push(diagnostic);
    }

    fn validate_default_argument_conflicts(&mut self) {
        let function_decl_groups: Vec<Vec<FunctionDeclSymbol>> =
            self.symbol_index.function_decl_groups().cloned().collect();
        for decls in &function_decl_groups {
            self.check_function_default_conflicts(decls);
        }
        let constructor_decl_groups: Vec<Vec<ConstructorDeclSymbol>> = self
            .symbol_index
            .constructor_decl_groups()
            .cloned()
            .collect();
        for decls in &constructor_decl_groups {
            self.check_constructor_default_conflicts(decls);
        }
    }

    fn check_function_default_conflicts(&mut self, decls: &[FunctionDeclSymbol]) {
        if decls.len() < 2 {
            return;
        }
        let mut recorded: Vec<Option<RecordedDefault>> = Vec::new();
        for decl in decls {
            let params = &decl.function.signature.parameters;
            if recorded.len() < params.len() {
                recorded.resize(params.len(), None);
            }
            for (index, param) in params.iter().enumerate() {
                let Some(default_expr) = &param.default else {
                    continue;
                };
                let canonical = default_expr.text.trim();
                let type_name = canonical_type_name(&param.ty);
                if let Some(entry) = recorded[index].as_ref() {
                    if entry.type_name == type_name && entry.text != canonical {
                        self.emit_error(
                            codes::PARAMETER_DEFAULT_CONFLICT,
                            default_expr.span,
                            format!(
                                "parameter `{}` on `{}` has conflicting default values `{}` and `{}`",
                                param.name, decl.qualified, entry.text, canonical
                            ),
                        );
                        if let Some(span) = entry.span {
                            self.diagnostics.push(typeck_diagnostics::note(
                                format!(
                                    "previous default `{}` declared for `{}`",
                                    entry.text, entry.function
                                ),
                                Some(span),
                            ));
                        }
                    }
                } else {
                    recorded[index] = Some(RecordedDefault {
                        text: canonical.to_string(),
                        type_name,
                        span: default_expr.span,
                        function: decl.qualified.clone(),
                    });
                }
            }
        }
    }

    fn check_constructor_default_conflicts(&mut self, decls: &[ConstructorDeclSymbol]) {
        if decls.len() < 2 {
            return;
        }
        let mut recorded: Vec<Option<RecordedDefault>> = Vec::new();
        for decl in decls {
            let params = &decl.constructor.parameters;
            if recorded.len() < params.len() {
                recorded.resize(params.len(), None);
            }
            for (index, param) in params.iter().enumerate() {
                let Some(default_expr) = &param.default else {
                    continue;
                };
                let canonical = default_expr.text.trim();
                let type_name = canonical_type_name(&param.ty);
                if let Some(entry) = recorded[index].as_ref() {
                    if entry.type_name == type_name && entry.text != canonical {
                        self.emit_error(
                            codes::PARAMETER_DEFAULT_CONFLICT,
                            default_expr.span,
                            format!(
                                "parameter `{}` on `{}` has conflicting default values `{}` and `{}`",
                                param.name, decl.qualified, entry.text, canonical
                            ),
                        );
                        if let Some(span) = entry.span {
                            self.diagnostics.push(typeck_diagnostics::note(
                                format!(
                                    "previous default `{}` declared for `{}`",
                                    entry.text, entry.function
                                ),
                                Some(span),
                            ));
                        }
                    }
                } else {
                    recorded[index] = Some(RecordedDefault {
                        text: canonical.to_string(),
                        type_name,
                        span: default_expr.span,
                        function: decl.qualified.clone(),
                    });
                }
            }
        }
    }

    fn evaluate_constants(&mut self) {
        let mut layouts = self.type_layouts.clone();
        let context = ConstEvalContext::new(
            &mut self.symbol_index,
            &mut layouts,
            Some(&self.import_resolver),
        );
        let summary: ConstEvalSummary = context.evaluate_all();
        for error in summary.errors {
            self.emit_error(codes::CONST_EVAL_FAILURE, error.span, error.message);
        }
        let metrics = summary.metrics;
        let namespace = self.module.namespace.as_deref().unwrap_or("<root>");
        debug!(
            target: "const_eval",
            namespace,
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

    pub(super) fn run_full(mut self, constraints: &[TypeConstraint]) -> TypeCheckResult {
        self.register_items();
        // Type hierarchy validation is temporarily disabled while the stdlib
        // adopts stricter class/interface rules; this keeps the constraint
        // solver focused on user-visible errors.
        // self.validate_type_hierarchy();
        self.enforce_base_accessibility();
        self.evaluate_constants();
        self.verify_layout_attributes();
        self.check_overloads();
        self.check_interface_fulfillment();
        self.check_virtual_dispatch();
        let solver_metrics = crate::typeck::trait_solver::TraitSolver::run(&mut self);
        self.trait_solver_metrics = solver_metrics.clone();
        self.check_constraints(constraints);
        self.verify_effects();
        self.validate_dependency_injection();
        let async_signatures = self
            .async_signatures
            .iter()
            .map(|(id, result)| {
                let signature = self.signatures.get(*id);
                AsyncSignatureInfo {
                    name: signature.name.clone(),
                    param_types: signature.param_types.clone(),
                    result: result.clone(),
                }
            })
            .collect();
        TypeCheckResult {
            diagnostics: self.diagnostics,
            async_signatures,
            interface_defaults: self.interface_default_bindings,
            trait_solver_metrics: solver_metrics,
        }
    }

    pub(super) fn run_constraints_only(
        mut self,
        constraints: &[TypeConstraint],
    ) -> Vec<Diagnostic> {
        self.register_items();
        // self.validate_type_hierarchy();
        self.evaluate_constants();
        self.check_virtual_dispatch();
        let solver_metrics = crate::typeck::trait_solver::TraitSolver::run(&mut self);
        self.trait_solver_metrics = solver_metrics;
        self.check_constraints(constraints);
        self.verify_effects();
        self.diagnostics
    }

    pub(super) fn run_trait_checks(mut self) -> TraitFulfillmentReport {
        self.register_items();
        // self.validate_type_hierarchy();
        self.evaluate_constants();
        self.verify_layout_attributes();
        self.check_overloads();
        self.check_interface_fulfillment();
        self.check_virtual_dispatch();
        self.validate_dependency_injection();
        let solver_metrics = crate::typeck::trait_solver::TraitSolver::run(&mut self);
        self.trait_solver_metrics = solver_metrics.clone();
        TraitFulfillmentReport {
            diagnostics: self.diagnostics,
            metrics: solver_metrics,
        }
    }

    pub(super) fn resolve_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.resolve_type_info_with_arity(name, None)
    }

    pub(super) fn resolve_type_info_with_arity(
        &self,
        name: &str,
        arity: Option<usize>,
    ) -> Option<&TypeInfo> {
        let base = base_type_name(name);
        self.types
            .get(name)
            .and_then(|infos| Self::select_type_by_arity(infos, arity))
            .or_else(|| {
                self.types
                    .get(base)
                    .and_then(|infos| Self::select_type_by_arity(infos, arity))
            })
            .or_else(|| {
                let qualified = name.contains("::")
                    || name.contains('.')
                    || base.contains("::")
                    || base.contains('.');
                if qualified {
                    return None;
                }
                self.types.iter().find_map(|(key, infos)| {
                    if type_names_equivalent(key, name) || type_names_equivalent(key, base) {
                        Self::select_type_by_arity(infos, arity)
                    } else {
                        None
                    }
                })
            })
    }

    fn select_type_by_arity<'b>(
        infos: &'b [TypeInfo],
        arity: Option<usize>,
    ) -> Option<&'b TypeInfo> {
        if infos.is_empty() {
            return None;
        }
        if let Some(expected) = arity {
            if let Some(info) = infos.iter().find(|info| type_arity(info) == expected) {
                return Some(info);
            }
            if let Some(generic) = infos
                .iter()
                .filter(|info| type_arity(info) > 0)
                .max_by_key(|info| type_arity(info))
            {
                return Some(generic);
            }
        }
        if infos.len() == 1 {
            return infos.first();
        }
        infos
            .iter()
            .find(|info| type_arity(info) == 0)
            .or_else(|| infos.first())
    }

    pub(super) fn has_type(&self, name: &str) -> bool {
        self.resolve_type_info(name).is_some()
    }

    pub(super) fn is_local_type(&self, name: &str) -> bool {
        self.types.contains_key(name) || self.types.contains_key(base_type_name(name))
    }
}

fn type_arity(info: &TypeInfo) -> usize {
    info.generics
        .as_ref()
        .map_or(0, |params| params.params.len())
}

#[cfg(test)]
mod allocation_tests {
    use super::*;
    use crate::frontend::parser::parse_module;
    #[cfg(debug_assertions)]
    use std::panic;

    #[test]
    fn allocation_stats_track_each_category() {
        let budgets = ArenaAllocationBudgets {
            signatures: 3,
            type_infos: 2,
            trait_infos: 1,
        };
        let mut allocations = ArenaAllocations::with_budgets(budgets);
        allocations.record(AllocationCategory::Signatures);
        allocations.record(AllocationCategory::TypeInfos);
        allocations.record(AllocationCategory::TraitInfos);
        let stats = allocations.snapshot();
        assert_eq!(stats.signatures, 1);
        assert_eq!(stats.type_infos, 1);
        assert_eq!(stats.trait_infos, 1);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn allocation_tracker_panics_when_exceeding_budget() {
        let budgets = ArenaAllocationBudgets {
            signatures: 1,
            type_infos: 0,
            trait_infos: 0,
        };
        let mut allocations = ArenaAllocations::with_budgets(budgets);
        allocations.record(AllocationCategory::Signatures);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            allocations.record(AllocationCategory::Signatures);
        }));
        assert!(
            result.is_err(),
            "expected debug assertion when exceeding signature budget"
        );
    }

    #[test]
    fn budgets_reflect_module_contents() {
        let module = parse_module(
            r#"
public class Foo
{
    public init() { }
    public int Run(int value) { return value; }
}

public interface IFoo
{
    public int Compute();
}
"#,
        )
        .expect("module parses")
        .module;
        let budgets = ArenaAllocationBudgets::from_module(&module);
        assert!(
            budgets.signatures >= 3,
            "expected at least 3 signatures, found {}",
            budgets.signatures
        );
        assert!(
            budgets.type_infos >= 2,
            "expected at least 2 type infos, found {}",
            budgets.type_infos
        );
        assert_eq!(
            budgets.trait_infos, 0,
            "expected no trait budget, found {}",
            budgets.trait_infos
        );
    }
}

#[cfg(test)]
mod tests;
