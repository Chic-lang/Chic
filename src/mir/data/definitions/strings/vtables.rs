//! Trait and class vtable metadata emitted alongside MIR for dynamic dispatch.

use crate::frontend::ast::PropertyAccessorKind;

/// Describes a single vtable generated for a `(Trait, ImplType)` pairing.
#[derive(Debug, Clone)]
pub struct TraitVTable {
    pub symbol: String,
    pub trait_name: String,
    pub impl_type: String,
    pub slots: Vec<VTableSlot>,
}

/// Entry for an object-safe method exposed through a trait vtable.
#[derive(Debug, Clone)]
pub struct VTableSlot {
    pub method: String,
    pub symbol: String,
}

/// Describes the virtual dispatch table for a concrete class type.
#[derive(Debug, Clone)]
pub struct ClassVTable {
    pub type_name: String,
    pub symbol: String,
    pub version: u64,
    pub slots: Vec<ClassVTableSlot>,
}

/// Entry for a class vtable slot (method or accessor).
#[derive(Debug, Clone)]
pub struct ClassVTableSlot {
    pub slot_index: u32,
    pub member: String,
    pub accessor: Option<PropertyAccessorKind>,
    pub symbol: String,
}
