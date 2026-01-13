use crate::frontend::ast::{ClassDecl, EnumDecl, FunctionDecl, Item, StructDecl};
use crate::frontend::diagnostics::Diagnostic;
use std::collections::HashMap;

use super::handlers;
use super::model::{MacroInvocation, normalise_name};

pub type DeriveHandler = fn(DeriveInput<'_>) -> DeriveOutput;
pub type AttributeHandler = fn(AttributeInput<'_>) -> AttributeOutput;

pub struct MacroRegistry {
    derive_macros: HashMap<String, DeriveHandler>,
    attribute_macros: HashMap<String, AttributeHandler>,
}

impl MacroRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            derive_macros: HashMap::new(),
            attribute_macros: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register_derive("Clone", handlers::derive_clone);
        registry.register_derive("Equatable", handlers::derive_equatable);
        registry.register_derive("Hashable", handlers::derive_hashable);
        registry.register_attribute("memoize", handlers::memoize_attribute);
        for name in [
            "allow",
            "dead_code",
            "unused_param",
            "style",
            "correctness",
            "perf",
            "pedantic",
            "all",
        ] {
            registry.register_attribute(name, handlers::noop_attribute);
        }
        registry
    }

    pub fn register_derive(&mut self, name: impl AsRef<str>, handler: DeriveHandler) {
        self.derive_macros
            .insert(normalise_name(name.as_ref()), handler);
    }

    pub fn register_attribute(&mut self, name: impl AsRef<str>, handler: AttributeHandler) {
        self.attribute_macros
            .insert(normalise_name(name.as_ref()), handler);
    }

    pub fn get_derive(&self, name: &str) -> Option<&DeriveHandler> {
        self.derive_macros.get(&normalise_name(name))
    }

    pub fn get_attribute(&self, name: &str) -> Option<&AttributeHandler> {
        self.attribute_macros.get(&normalise_name(name))
    }
}

pub struct DeriveInput<'i> {
    pub invocation: &'i MacroInvocation,
    pub target: DeriveTarget<'i>,
}

pub enum DeriveTarget<'a> {
    Struct(&'a mut StructDecl),
    Enum(&'a mut EnumDecl),
    Class(&'a mut ClassDecl),
}

pub struct DeriveOutput {
    pub new_items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
}

impl DeriveOutput {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            new_items: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

pub struct AttributeInput<'i> {
    pub invocation: &'i MacroInvocation,
    pub target: AttributeTarget<'i>,
}

pub enum AttributeTarget<'a> {
    Function(&'a mut FunctionDecl),
    Method {
        owner: String,
        function: &'a mut FunctionDecl,
    },
}

pub struct AttributeOutput {
    pub new_items: Vec<Item>,
    pub diagnostics: Vec<Diagnostic>,
}

impl AttributeOutput {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            new_items: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}
