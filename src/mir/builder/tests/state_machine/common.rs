pub(super) use crate::frontend::ast::{
    Expression, PropertyAccessorKind, Statement, StatementKind as AstStatementKind,
    TraitObjectTypeExpr, TypeExpr,
};
pub(super) use crate::frontend::import_resolver::ImportResolver;
pub(super) use crate::frontend::parser::parse_module;
pub(super) use crate::mir::AtomicOrdering;
pub(super) use crate::mir::builder::SymbolIndex;
pub(super) use crate::mir::builder::body_builder::{
    AssignmentSourceKind, BodyBuilder, impl_trait_bounds_from_type_expr,
};
pub(super) use crate::mir::builder::default_arguments::{DefaultArgumentMap, DefaultArgumentStore};
pub(super) use crate::mir::builder::module_lowering::driver::{TypeDeclInfo, lower_module};
pub(super) use crate::mir::builder::module_lowering::traits::TraitLoweringInfo;
pub(super) use crate::mir::builder::static_registry::StaticRegistry;
pub(super) use crate::mir::builder::string_interner::StringInterner;
pub(super) use crate::mir::builder::symbol_index::{FieldSymbol, PropertySymbol};
pub(super) use crate::mir::builder::tests::common::RequireExt;
pub(super) use crate::mir::builder::{FunctionSpecialization, Span, Visibility};
pub(super) use crate::mir::data::{
    Abi, ConstOperand, ConstValue, FnSig, LocalDecl, LocalId, LocalKind, Operand, ParamMode,
    PendingOperand, PendingStatementKind, Place, ProjectionElem, Rvalue, Statement as MirStatement,
    StatementKind as MirStatementKind, Terminator, Ty, ValueCategory,
};
pub(super) use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutInfo, ClassLayoutKind, FieldLayout, StructLayout,
    TypeLayout, TypeLayoutTable, TypeRepr,
};
pub(super) use crate::mir::operators::OperatorRegistry;
pub(super) use crate::mir::{BlockId, FunctionKind};
pub(super) use crate::primitives::PrimitiveRegistry;
pub(super) use crate::syntax::expr::{AssignOp, ExprNode};
pub(super) use crate::threading::thread_runtime_mode;
pub(super) use crate::typeck::ConstraintKind;
pub(super) use std::cell::RefCell;
pub(super) use std::collections::HashMap;
pub(super) use std::rc::Rc;

pub(super) fn default_argument_store() -> DefaultArgumentStore {
    Rc::new(RefCell::new(DefaultArgumentMap::default()))
}

pub(super) fn simple_signature() -> FnSig {
    FnSig {
        params: Vec::new(),
        ret: Ty::named("int"),
        abi: Abi::Chic,
        effects: Vec::new(),
        lends_to_return: None,
        variadic: false,
    }
}

pub(super) fn with_state_builder<C, F>(function_kind: FunctionKind, configure: C, test: F)
where
    C: FnOnce(&mut TypeLayoutTable),
    F: FnOnce(BodyBuilder<'_>),
{
    with_named_state_builder("State::Fixture", function_kind, configure, test);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_named_state_builder<C, F>(
    function_name: &str,
    function_kind: FunctionKind,
    configure: C,
    test: F,
) where
    C: FnOnce(&mut TypeLayoutTable),
    F: FnOnce(BodyBuilder<'_>),
{
    with_named_state_builder_with_index(
        function_name,
        function_kind,
        configure,
        SymbolIndex::default(),
        test,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_named_state_builder_with_index<C, F>(
    function_name: &str,
    function_kind: FunctionKind,
    configure: C,
    symbol_index: SymbolIndex,
    test: F,
) where
    C: FnOnce(&mut TypeLayoutTable),
    F: FnOnce(BodyBuilder<'_>),
{
    let mut type_layouts = TypeLayoutTable::default();
    configure(&mut type_layouts);
    let mut interner = StringInterner::new();
    let operator_registry = OperatorRegistry::default();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let static_registry = StaticRegistry::new();
    let class_bases: HashMap<String, Vec<String>> = HashMap::new();
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let function_packages: HashMap<String, String> = HashMap::new();
    let signature = simple_signature();
    let generic_specializations = Rc::new(RefCell::new(Vec::<FunctionSpecialization>::new()));

    let builder = BodyBuilder::new(
        &signature,
        Some(Span::new(0, 1)),
        function_name,
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        None,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        function_kind,
        false,
        thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );
    test(builder);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_custom_state_builder<C, B, F>(
    function_name: &str,
    function_kind: FunctionKind,
    namespace: Option<&str>,
    configure_layouts: C,
    configure_bases: B,
    mut symbol_index: SymbolIndex,
    test: F,
) where
    C: FnOnce(&mut TypeLayoutTable),
    B: FnOnce(&mut HashMap<String, Vec<String>>),
    F: FnOnce(BodyBuilder<'_>),
{
    let mut type_layouts = TypeLayoutTable::default();
    configure_layouts(&mut type_layouts);
    let mut interner = StringInterner::new();
    let operator_registry = OperatorRegistry::default();
    let import_resolver = ImportResolver::default();
    let default_arguments = default_argument_store();
    let static_registry = StaticRegistry::new();
    let mut class_bases: HashMap<String, Vec<String>> = HashMap::new();
    configure_bases(&mut class_bases);
    let class_virtual_slots: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let trait_registry: HashMap<String, TraitLoweringInfo> = HashMap::new();
    let type_visibilities: HashMap<String, TypeDeclInfo> = HashMap::new();
    let primitive_registry = PrimitiveRegistry::with_builtins();
    let signature = simple_signature();
    let generic_specializations = Rc::new(RefCell::new(Vec::<FunctionSpecialization>::new()));
    let function_packages: HashMap<String, String> = HashMap::new();

    symbol_index.types.extend(class_bases.keys().cloned());

    let builder = BodyBuilder::new(
        &signature,
        Some(Span::new(0, 1)),
        function_name,
        false,
        false,
        Vec::new(),
        &mut type_layouts,
        &type_visibilities,
        &primitive_registry,
        default_arguments.clone(),
        namespace,
        None,
        &function_packages,
        &operator_registry,
        &mut interner,
        &symbol_index,
        &import_resolver,
        &static_registry,
        &class_bases,
        &class_virtual_slots,
        &trait_registry,
        function_kind,
        false,
        thread_runtime_mode(),
        None,
        None,
        generic_specializations.clone(),
    );
    test(builder);
}

pub(super) fn empty_layout(name: &str) -> TypeLayout {
    TypeLayout::Struct(StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    })
}

pub(super) fn required_field(name: &str, index: u32, display_name: Option<&str>) -> FieldLayout {
    FieldLayout {
        name: name.to_string(),
        ty: Ty::named("int"),
        index,
        offset: None,
        span: Some(Span::new(index as usize, index as usize + 1)),
        mmio: None,
        display_name: display_name.map(ToString::to_string),
        is_required: true,
        is_nullable: false,
        is_readonly: false,
        view_of: None,
    }
}

pub(super) fn required_layout(
    name: &str,
    bases: Vec<String>,
    fields: Vec<FieldLayout>,
) -> TypeLayout {
    TypeLayout::Struct(StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases,
            vtable_offset: None,
        }),
    })
}

pub(super) fn insert_readonly_layout(table: &mut TypeLayoutTable) {
    let field = FieldLayout {
        name: "Value".into(),
        ty: Ty::named("int"),
        index: 0,
        offset: Some(0),
        span: None,
        mmio: None,
        display_name: None,
        is_required: false,
        is_nullable: false,
        is_readonly: true,
        view_of: None,
    };
    let layout = StructLayout {
        name: "State::Readonly".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![field],
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride {
            thread_safe: None,
            shareable: None,
            copy: None,
        },
        mmio: None,
        dispose: None,
        class: None,
    };
    table
        .types
        .insert("State::Readonly".into(), TypeLayout::Struct(layout));
}
