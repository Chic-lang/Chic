//! Reflection descriptor generation for Chic `public` items.
//!
//! This module walks the parsed AST and emits lightweight descriptor tables that can be
//! serialised into metadata sidecars. The descriptors intentionally avoid depending on type
//! checking results so they can be produced during lowering, cached in metadata, and used by
//! compile-time features such as `reflect<T>()` and macro quasiquotes.

use serde::{Deserialize, Serialize};

use crate::frontend::ast::Module;
use crate::frontend::ast::items::{BindingModifier, Visibility};

mod emit;
mod query;

use emit::ReflectionEmitter;
use query::DescriptorQuery;

/// Top-level reflection tables produced for a module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReflectionTables {
    #[serde(default = "reflection_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub types: Vec<TypeDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<TypeAliasDescriptor>,
}

/// Handle referencing a type descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeHandle {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_id: Option<u64>,
}

/// Descriptor for a public type-level item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_id: Option<u64>,
    pub kind: TypeKind,
    pub visibility: VisibilityDescriptor,
    #[serde(default)]
    pub is_generic: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_arguments: Vec<TypeHandle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bases: Vec<TypeHandle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlying_type: Option<TypeHandle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<TypeLayoutDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_hints: Option<LayoutDescriptor>,
    #[serde(default)]
    pub readonly: bool,
}

/// Descriptor for a type alias mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeAliasDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
    pub full_name: String,
    pub target: TypeHandle,
    pub visibility: VisibilityDescriptor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutDescriptor {
    pub repr_c: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeLayoutDescriptor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldLayoutDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldLayoutDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ty: Option<TypeHandle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeArgument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributeDescriptor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positional_args: Vec<AttributeArgument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub named_args: Vec<AttributeArgument>,
}

/// Descriptor for a public member of a type or function surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberDescriptor {
    pub name: String,
    pub kind: MemberKind,
    pub visibility: VisibilityDescriptor,
    pub declaring_type: TypeHandle,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<PropertyDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<MethodDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constructor: Option<ConstructorDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<MemberDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub field_type: TypeHandle,
    #[serde(default)]
    pub is_static: bool,
    #[serde(default)]
    pub is_readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PropertyDescriptor {
    pub property_type: TypeHandle,
    #[serde(default)]
    pub has_getter: bool,
    #[serde(default)]
    pub has_setter: bool,
    #[serde(default)]
    pub has_init: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub getter: Option<MethodDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setter: Option<MethodDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<MethodDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MethodDescriptor {
    pub return_type: TypeHandle,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterDescriptor>,
    #[serde(default)]
    pub is_static: bool,
    #[serde(default)]
    pub is_virtual: bool,
    #[serde(default)]
    pub is_override: bool,
    #[serde(default)]
    pub is_abstract: bool,
    #[serde(default)]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub throws: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extern_abi: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstructorDescriptor {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterDescriptor>,
    #[serde(default)]
    pub is_designated: bool,
    #[serde(default)]
    pub is_convenience: bool,
}

/// Descriptor for a member or function parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParameterDescriptor {
    pub name: String,
    pub parameter_type: TypeHandle,
    pub mode: ParameterMode,
    #[serde(default)]
    pub has_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<AttributeDescriptor>,
}

/// High-level classification for public types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TypeKind {
    Struct,
    Record,
    Class,
    Enum,
    Interface,
    Union,
    Extension,
    Trait,
    Delegate,
    Impl,
    Function,
    Const,
    Static,
}

/// Kinds of members captured in descriptors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MemberKind {
    Field,
    Property,
    Method,
    Constructor,
    Const,
    EnumVariant,
    UnionField,
    UnionView,
    AssociatedType,
    ExtensionMethod,
    TraitMethod,
}

/// Visibility value recorded in descriptors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityDescriptor {
    Public,
    Internal,
    Protected,
    Private,
    ProtectedInternal,
    PrivateProtected,
}

/// Parameter binding mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ParameterMode {
    In,
    Ref,
    Out,
    Value,
}

impl From<Visibility> for VisibilityDescriptor {
    fn from(value: Visibility) -> Self {
        match value {
            Visibility::Public => Self::Public,
            Visibility::Internal => Self::Internal,
            Visibility::Protected => Self::Protected,
            Visibility::Private => Self::Private,
            Visibility::ProtectedInternal => Self::ProtectedInternal,
            Visibility::PrivateProtected => Self::PrivateProtected,
        }
    }
}

impl From<BindingModifier> for ParameterMode {
    fn from(value: BindingModifier) -> Self {
        match value {
            BindingModifier::In => Self::In,
            BindingModifier::Ref => Self::Ref,
            BindingModifier::Out => Self::Out,
            BindingModifier::Value => Self::Value,
        }
    }
}

/// Collect public reflection descriptors for the provided module.
pub fn collect_reflection_tables(module: &Module) -> ReflectionTables {
    DescriptorQuery::collect(module)
}

/// Serialize reflection tables into a stable, pretty-printed JSON string.
pub fn serialize_reflection_tables(tables: &ReflectionTables) -> Result<String, serde_json::Error> {
    ReflectionEmitter::to_pretty_json(tables)
}

/// Parse reflection tables from a serialized JSON string.
pub fn deserialize_reflection_tables(input: &str) -> Result<ReflectionTables, serde_json::Error> {
    ReflectionEmitter::from_str(input)
}

/// Convenience helper that collects and serializes metadata in one step.
pub fn collect_and_serialize_reflection(module: &Module) -> Result<String, serde_json::Error> {
    let tables = collect_reflection_tables(module);
    serialize_reflection_tables(&tables)
}

const fn reflection_schema_version() -> u32 {
    2
}

impl Default for ReflectionTables {
    fn default() -> Self {
        Self {
            version: reflection_schema_version(),
            types: Vec::new(),
            aliases: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::parse_module;

    #[test]
    fn collects_alias_descriptors() {
        let parsed = parse_module(
            r#"
namespace Audio;

public typealias Sample = ushort;
"#,
        )
        .expect("parse alias");
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        let tables = collect_reflection_tables(&parsed.module);
        assert_eq!(tables.aliases.len(), 1);
        let alias = &tables.aliases[0];
        assert_eq!(alias.full_name, "Audio::Sample");
        assert_eq!(alias.namespace.as_deref(), Some("Audio"));
        assert_eq!(alias.target.name, "ushort");
        assert_eq!(alias.visibility, VisibilityDescriptor::Public);
    }
}
