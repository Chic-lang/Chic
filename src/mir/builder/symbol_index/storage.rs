//! Backing storage for the symbol index plus the invariants enforced by each table.
//!
//! The symbol index owns a collection of HashMaps/Sets tracking every symbol discovered
//! during AST traversal. Each table has strict invariants (e.g. `types` always contains
//! fully-qualified names, field/property maps are namespaced by owner, etc.) so wrapper
//! helpers live here to keep the bookkeeping logic centralized.

use std::collections::{HashMap, HashSet};

use crate::frontend::ast::Expression;
use crate::frontend::ast::{
    ConstructorDecl, FunctionDecl, PropertyAccessorKind, TypeExpr, Visibility,
};
use crate::frontend::diagnostics::Span;
use crate::mir::data::{ConstValue, FnTy, ParamMode};
use crate::type_metadata::TypeVariance;

#[derive(Clone, Debug)]
pub struct PropertyAccessorMetadata {
    pub function: String,
}

#[derive(Clone, Debug)]
pub struct FieldMetadata {
    pub ty: TypeExpr,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_readonly: bool,
    pub is_required: bool,
    pub span: Option<Span>,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PropertyMetadata {
    pub visibility: Visibility,
    pub is_static: bool,
    pub has_setter: bool,
    pub has_init: bool,
    pub span: Option<Span>,
    pub namespace: Option<String>,
    pub is_required: bool,
}

#[derive(Clone, Debug)]
pub struct PropertySymbol {
    pub ty: String,
    pub is_static: bool,
    pub accessors: HashMap<PropertyAccessorKind, PropertyAccessorMetadata>,
    pub span: Option<Span>,
    pub is_required: bool,
    pub is_nullable: bool,
    pub visibility: Visibility,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FieldSymbol {
    pub ty: TypeExpr,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_readonly: bool,
    pub is_required: bool,
    pub span: Option<Span>,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FunctionParamSymbol {
    pub name: String,
    pub has_default: bool,
    pub mode: ParamMode,
    pub is_extension_this: bool,
}

impl FunctionParamSymbol {
    pub fn is_receiver(&self) -> bool {
        self.is_extension_this
            || self.name.eq_ignore_ascii_case("self")
            || self.name.eq_ignore_ascii_case("this")
    }
}

#[derive(Clone, Debug)]
pub struct FunctionSymbol {
    pub qualified: String,
    pub internal_name: String,
    pub signature: FnTy,
    pub params: Vec<FunctionParamSymbol>,
    pub is_unsafe: bool,
    pub is_static: bool,
    pub visibility: Visibility,
    pub namespace: Option<String>,
    pub owner: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PropertyAccessorLookup {
    pub owner: String,
    pub kind: PropertyAccessorKind,
    pub backing_field: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ConstSymbol {
    pub qualified: String,
    pub name: String,
    pub owner: Option<String>,
    pub namespace: Option<String>,
    pub ty: TypeExpr,
    pub initializer: Expression,
    pub visibility: Visibility,
    pub modifiers: Vec<String>,
    pub span: Option<Span>,
    pub value: Option<ConstValue>,
}

#[derive(Clone, Debug)]
pub struct FunctionDeclSymbol {
    pub qualified: String,
    pub function: FunctionDecl,
    pub owner: Option<String>,
    pub namespace: Option<String>,
    pub internal_name: String,
}

#[derive(Clone, Debug)]
pub struct ConstructorDeclSymbol {
    pub qualified: String,
    pub constructor: ConstructorDecl,
    pub owner: String,
    pub namespace: Option<String>,
    pub internal_name: String,
}

#[derive(Clone, Debug)]
pub struct TypeGenericParamEntry {
    pub name: String,
    pub variance: TypeVariance,
}

#[derive(Clone, Default)]
pub struct SymbolStorage {
    pub types: HashSet<String>,
    pub readonly_structs: HashSet<String>,
    pub functions: HashMap<String, Vec<FunctionSymbol>>,
    pub function_decls: HashMap<String, Vec<FunctionDeclSymbol>>,
    pub constructor_decls: HashMap<String, Vec<ConstructorDeclSymbol>>,
    pub delegate_signatures: HashMap<String, FnTy>,
    pub type_generics: HashMap<String, Vec<TypeGenericParamEntry>>,
    pub type_fields: HashMap<String, HashMap<String, FieldSymbol>>,
    pub type_methods: HashMap<String, HashMap<String, usize>>,
    pub type_properties: HashMap<String, HashMap<String, PropertySymbol>>,
    pub enum_variants: HashMap<String, HashSet<String>>,
    pub extension_placeholders: HashMap<String, HashSet<String>>,
    pub property_accessors: HashMap<String, PropertyAccessorLookup>,
    pub constants: HashMap<String, ConstSymbol>,
    pub type_constants: HashMap<String, HashMap<String, ConstSymbol>>,
    pub namespace_constants: HashMap<String, HashMap<String, ConstSymbol>>,
}

#[allow(dead_code)]
impl SymbolStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.types.clear();
        self.readonly_structs.clear();
        self.functions.clear();
        self.function_decls.clear();
        self.constructor_decls.clear();
        self.delegate_signatures.clear();
        self.type_generics.clear();
        self.type_fields.clear();
        self.type_methods.clear();
        self.type_properties.clear();
        self.enum_variants.clear();
        self.extension_placeholders.clear();
        self.property_accessors.clear();
        self.constants.clear();
        self.type_constants.clear();
        self.namespace_constants.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_storage_clear_resets_state() {
        let mut storage = SymbolStorage::new();
        storage.types.insert("Foo".into());
        storage.constants.insert(
            "Foo::CONST".into(),
            ConstSymbol {
                qualified: "Foo::CONST".into(),
                name: "CONST".into(),
                owner: Some("Foo".into()),
                namespace: Some("Foo".into()),
                ty: TypeExpr::simple("int"),
                initializer: Expression::new("1", None),
                visibility: Visibility::Public,
                modifiers: Vec::new(),
                span: None,
                value: None,
            },
        );
        storage.clear();
        assert!(storage.types.is_empty());
        assert!(storage.constants.is_empty());
    }
}
