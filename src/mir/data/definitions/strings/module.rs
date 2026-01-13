use crate::frontend::ast::Visibility;
use crate::frontend::diagnostics::Span;
use crate::mir::layout::TypeLayoutTable;
use crate::primitives::PrimitiveRegistry;
use crate::type_metadata::TypeVariance;
use std::collections::HashMap;

use super::basic_blocks::ConstValue;
use super::types::Ty;

use super::interning::InternedStr;
use super::module_metadata::{Export, ModuleAttributes};
use super::{ClassVTable, MirExternSpec, MirFunction, TraitVTable};
use crate::mir::AsyncLoweringArtifact;

/// A lowered Chic module containing MIR functions.
#[derive(Debug, Clone)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub test_cases: Vec<crate::mir::TestCaseMetadata>,
    pub statics: Vec<StaticVar>,
    pub type_layouts: TypeLayoutTable,
    pub primitive_registry: PrimitiveRegistry,
    pub interned_strs: Vec<InternedStr>,
    pub exports: Vec<Export>,
    pub attributes: ModuleAttributes,
    pub trait_vtables: Vec<TraitVTable>,
    pub class_vtables: Vec<ClassVTable>,
    pub interface_defaults: Vec<InterfaceDefaultImpl>,
    pub default_arguments: Vec<DefaultArgumentRecord>,
    pub type_variance: HashMap<String, Vec<TypeVariance>>,
    pub async_plans: Vec<AsyncLoweringArtifact>,
}

impl Default for MirModule {
    fn default() -> Self {
        Self {
            functions: Vec::new(),
            test_cases: Vec::new(),
            statics: Vec::new(),
            type_layouts: TypeLayoutTable::default(),
            primitive_registry: PrimitiveRegistry::with_builtins(),
            interned_strs: Vec::new(),
            exports: Vec::new(),
            attributes: ModuleAttributes::default(),
            trait_vtables: Vec::new(),
            class_vtables: Vec::new(),
            interface_defaults: Vec::new(),
            default_arguments: Vec::new(),
            type_variance: HashMap::new(),
            async_plans: Vec::new(),
        }
    }
}

/// Identifier for a module-level static variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticId(pub usize);

/// Chic static storage definition emitted alongside MIR.
#[derive(Debug, Clone)]
pub struct StaticVar {
    pub id: StaticId,
    pub qualified: String,
    pub owner: Option<String>,
    pub namespace: Option<String>,
    pub ty: Ty,
    pub visibility: Visibility,
    pub is_readonly: bool,
    pub threadlocal: bool,
    pub is_weak: bool,
    pub is_extern: bool,
    pub is_import: bool,
    pub is_weak_import: bool,
    pub link_library: Option<String>,
    pub extern_spec: Option<MirExternSpec>,
    pub span: Option<Span>,
    pub initializer: Option<ConstValue>,
}

#[derive(Debug, Clone)]
pub struct InterfaceDefaultImpl {
    pub implementer: String,
    pub interface: String,
    pub method: String,
    pub symbol: String,
}

#[derive(Debug, Clone)]
pub struct DefaultArgumentRecord {
    pub function: String,
    pub internal: String,
    pub param_name: String,
    pub param_index: usize,
    pub span: Option<Span>,
    pub value: DefaultArgumentKind,
}

#[derive(Debug, Clone)]
pub enum DefaultArgumentKind {
    Const(ConstValue),
    Thunk {
        symbol: String,
        metadata_count: usize,
    },
}
