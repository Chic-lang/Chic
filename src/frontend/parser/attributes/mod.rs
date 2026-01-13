use super::{Parser, Statement, StatementKind};
use crate::frontend::ast::{Attribute, MmioAccess, MmioEndianness, MmioFieldAttr, MmioStructAttr};
use crate::frontend::diagnostics::Span;
use std::convert::TryFrom;

mod apply;
mod flags;
mod grammar;
mod mmio;
mod utils;

pub(crate) use flags::{
    AttributeFlags, FunctionAttributeSet, ParsedExternSpec, StaticAttributeSet,
};

#[derive(Clone, Default)]
pub(crate) struct CollectedAttributes {
    pub(crate) builtin: AttributeFlags,
    pub(crate) list: Vec<Attribute>,
}

impl CollectedAttributes {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.builtin.is_empty() && self.list.is_empty()
    }

    pub(crate) fn push(&mut self, attr: Attribute) {
        self.list.push(attr);
    }

    pub(crate) fn take_list(&mut self) -> Vec<Attribute> {
        std::mem::take(&mut self.list)
    }

    pub(crate) fn take_c_imports(&mut self) -> Vec<(String, Option<Span>)> {
        self.builtin.take_c_imports()
    }

    pub(crate) fn take_friend_namespaces(&mut self) -> Vec<(String, Option<Span>)> {
        let friends = self.builtin.take_friend_namespaces();
        self.list
            .retain(|attr| !attr.name.eq_ignore_ascii_case("friend"));
        friends
    }

    pub(crate) fn take_package_imports(&mut self) -> Vec<(String, Option<Span>)> {
        let imports = self.builtin.take_package_imports();
        self.list
            .retain(|attr| !attr.name.eq_ignore_ascii_case("package"));
        imports
    }

    pub(in crate::frontend::parser) fn take_function_attributes(&mut self) -> FunctionAttributeSet {
        self.builtin.take_function_attributes()
    }

    pub(in crate::frontend::parser) fn take_static_attributes(&mut self) -> StaticAttributeSet {
        self.builtin.take_static_attributes()
    }

    pub(crate) fn into_parts(self) -> (AttributeFlags, Vec<Attribute>) {
        (self.builtin, self.list)
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParsedAttributeArgument {
    pub(super) name: String,
    pub(super) value: ParsedAttributeValue,
    pub(super) span: Span,
}

#[derive(Debug, Clone)]
pub(super) enum ParsedAttributeValue {
    Int(u64),
    Str(String),
    Bool(bool),
}
