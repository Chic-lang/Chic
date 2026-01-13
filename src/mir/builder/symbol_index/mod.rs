use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

use crate::frontend::ast::{Module, PropertyAccessorKind};
use crate::frontend::metadata::{TypeDescriptor, collect_reflection_tables};
use crate::mir::data::{ConstValue, FnTy};
use crate::type_metadata::TypeVariance;

mod storage;
mod updates;

pub use storage::{
    ConstSymbol, ConstructorDeclSymbol, FieldMetadata, FieldSymbol, FunctionDeclSymbol,
    FunctionParamSymbol, FunctionSymbol, PropertyAccessorMetadata, PropertyMetadata,
    PropertySymbol,
};
use storage::{PropertyAccessorLookup, SymbolStorage, TypeGenericParamEntry};
pub(crate) use updates::{candidate_function_names, canonical_method_owner};

#[derive(Clone, Default)]
pub struct SymbolIndex {
    storage: SymbolStorage,
    reflection_types: HashMap<String, TypeDescriptor>,
    overload_ordinals: HashMap<String, usize>,
}

impl Deref for SymbolIndex {
    type Target = SymbolStorage;

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl DerefMut for SymbolIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage
    }
}

impl SymbolIndex {
    fn allocate_internal_name(&mut self, qualified: &str) -> String {
        let counter = self
            .overload_ordinals
            .entry(qualified.to_string())
            .or_insert(0);
        let ordinal = *counter;
        *counter += 1;
        if ordinal == 0 {
            qualified.to_string()
        } else {
            format!("{qualified}#{ordinal}")
        }
    }

    #[must_use]
    pub fn build(module: &Module) -> Self {
        let mut index = Self::default();
        let namespace = module.namespace.as_deref();
        index.collect_items(module.items.iter(), namespace);
        let tables = collect_reflection_tables(module);
        index.reflection_types = tables
            .types
            .into_iter()
            .map(|descriptor| (descriptor.name.clone(), descriptor))
            .collect();
        index
    }

    pub(crate) fn reflection_descriptor(&self, name: &str) -> Option<&TypeDescriptor> {
        self.reflection_types.get(name)
    }

    pub(crate) fn types(&self) -> impl Iterator<Item = &String> {
        self.storage.types.iter()
    }

    #[must_use]
    pub fn delegate_signature(&self, name: &str) -> Option<&FnTy> {
        if let Some(sig) = self.storage.delegate_signatures.get(name) {
            return Some(sig);
        }
        self.storage
            .delegate_signatures
            .iter()
            .find_map(|(qualified, sig)| {
                qualified
                    .rsplit("::")
                    .next()
                    .is_some_and(|short| short == name)
                    .then_some(sig)
            })
    }

    pub(crate) fn function_decl_groups(&self) -> impl Iterator<Item = &Vec<FunctionDeclSymbol>> {
        self.function_decls.values()
    }

    pub(crate) fn constructor_decl_groups(
        &self,
    ) -> impl Iterator<Item = &Vec<ConstructorDeclSymbol>> {
        self.constructor_decls.values()
    }

    pub(crate) fn is_readonly_struct(&self, name: &str) -> bool {
        if self.readonly_structs.contains(name) {
            return true;
        }
        self.reflection_descriptor(name)
            .map(|descriptor| descriptor.readonly)
            .unwrap_or(false)
    }
    pub fn register_method(&mut self, type_name: &str, method: &str) {
        let owner_key = canonical_method_owner(type_name);
        let methods = self.type_methods.entry(owner_key.clone()).or_default();
        *methods.entry(method.to_string()).or_insert(0) += 1;
        if let Some(placeholders) = self.extension_placeholders.get_mut(&owner_key) {
            placeholders.remove(method);
            if placeholders.is_empty() {
                self.extension_placeholders.remove(&owner_key);
            }
        }
    }

    #[must_use]
    pub fn has_field(&self, type_name: &str, field: &str) -> bool {
        self.field_symbol(type_name, field).is_some()
    }

    #[must_use]
    pub(super) fn field_symbol(&self, type_name: &str, field: &str) -> Option<&FieldSymbol> {
        if let Some(symbol) = self
            .type_fields
            .get(type_name)
            .and_then(|fields| fields.get(field))
        {
            return Some(symbol);
        }
        for (candidate, fields) in &self.type_fields {
            if candidate.ends_with(type_name) {
                if let Some(symbol) = fields.get(field) {
                    return Some(symbol);
                }
            }
        }
        None
    }

    #[must_use]
    pub fn field_metadata(&self, type_name: &str, field: &str) -> Option<FieldMetadata> {
        let symbol = self.field_symbol(type_name, field)?;
        Some(FieldMetadata {
            ty: symbol.ty.clone(),
            visibility: symbol.visibility,
            is_static: symbol.is_static,
            is_readonly: symbol.is_readonly,
            is_required: symbol.is_required,
            span: symbol.span,
            namespace: symbol.namespace.clone(),
        })
    }

    #[must_use]
    pub fn has_enum_variant(&self, type_name: &str, variant: &str) -> bool {
        self.enum_variants
            .get(type_name)
            .is_some_and(|variants| variants.contains(variant))
    }

    #[must_use]
    pub fn method_count(&self, type_name: &str, method: &str) -> Option<usize> {
        let owner_key = canonical_method_owner(type_name);
        if let Some(methods) = self.type_methods.get(&owner_key)
            && let Some(count) = methods.get(method)
            && *count > 0
        {
            return Some(*count);
        }

        self.extension_placeholders
            .get(&owner_key)
            .and_then(|set| set.contains(method).then_some(1))
    }

    #[must_use]
    pub fn function_count(&self, qualified: &str) -> Option<usize> {
        self.functions
            .get(qualified)
            .map(|overloads| overloads.len())
    }

    #[must_use]
    pub fn function_signature(&self, qualified: &str) -> Option<&FnTy> {
        let overloads = self.functions.get(qualified)?;
        if overloads.len() == 1 {
            Some(&overloads[0].signature)
        } else {
            None
        }
    }

    #[must_use]
    pub(crate) fn function_overloads(&self, qualified: &str) -> Option<&[FunctionSymbol]> {
        self.functions.get(qualified).map(Vec::as_slice)
    }

    #[must_use]
    pub(crate) fn static_method_overloads<'a>(
        &'a self,
        owner: &str,
        method: &str,
    ) -> Vec<&'a FunctionSymbol> {
        let qualified = format!("{owner}::{method}");
        self.functions
            .get(&qualified)
            .map(|symbols| {
                symbols
                    .iter()
                    .filter(|symbol| symbol.is_static)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn resolve_function_by_suffixes(&self, suffix: &str) -> Vec<String> {
        let mut matches = Vec::new();
        for name in self.functions.keys() {
            if name
                .rsplit("::")
                .next()
                .is_some_and(|candidate| candidate == suffix)
            {
                matches.push(name.clone());
            }
        }
        matches.sort();
        matches.dedup();
        matches
    }

    #[must_use]
    pub fn contains_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }

    #[must_use]
    pub(crate) fn constructor_overloads(&self, owner: &str) -> Vec<&FunctionSymbol> {
        let prefix = format!("{owner}::init");
        let mut matches = Vec::new();
        for (qualified, symbols) in &self.functions {
            if qualified.starts_with(&prefix) {
                matches.extend(symbols.iter());
            }
        }
        matches
    }

    #[must_use]
    pub(crate) fn resolve_function<'a>(
        &'a self,
        namespace: Option<&str>,
        name: &str,
    ) -> Vec<&'a FunctionSymbol> {
        let segments: Vec<&str> = name
            .split("::")
            .filter(|segment| !segment.is_empty())
            .collect();
        if segments.is_empty() {
            return Vec::new();
        }

        let mut seen = HashSet::new();
        let mut matches = Vec::new();
        for candidate in candidate_function_names(namespace, &segments) {
            if let Some(overloads) = self.functions.get(&candidate) {
                for symbol in overloads {
                    let key = (symbol.qualified.clone(), symbol.signature.canonical_name());
                    if seen.insert(key) {
                        matches.push(symbol);
                    }
                }
            }
        }
        if matches.is_empty() && segments.len() == 1 {
            for unique in self.resolve_function_by_suffixes(segments[0]) {
                if let Some(overloads) = self.functions.get(&unique) {
                    for symbol in overloads {
                        let key = (symbol.qualified.clone(), symbol.signature.canonical_name());
                        if seen.insert(key) {
                            matches.push(symbol);
                        }
                    }
                }
            }
        }
        matches
    }

    #[must_use]
    pub(super) fn property(&self, type_name: &str, name: &str) -> Option<&PropertySymbol> {
        if let Some(symbol) = self
            .type_properties
            .get(type_name)
            .and_then(|props| props.get(name))
        {
            return Some(symbol);
        }
        for (candidate, props) in &self.type_properties {
            if candidate.ends_with(type_name) {
                if let Some(symbol) = props.get(name) {
                    return Some(symbol);
                }
            }
        }
        None
    }

    #[must_use]
    pub(super) fn type_generics(&self, name: &str) -> Option<&[TypeGenericParamEntry]> {
        self.type_generics.get(name).map(|list| list.as_slice())
    }

    #[must_use]
    pub(super) fn resolve_type_generics_owner(&self, name: &str) -> Option<String> {
        if self.type_generics.contains_key(name) {
            return Some(name.to_string());
        }
        self.type_generics
            .keys()
            .find(|candidate| candidate == &name || candidate.ends_with(&format!("::{name}")))
            .cloned()
    }

    pub(super) fn drain_type_variance(&mut self) -> HashMap<String, Vec<TypeVariance>> {
        let map = std::mem::take(&mut self.type_generics);
        map.into_iter()
            .map(|(name, params)| {
                let variances = params.into_iter().map(|entry| entry.variance).collect();
                (name, variances)
            })
            .collect()
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn const_symbol(&self, qualified: &str) -> Option<&ConstSymbol> {
        self.constants.get(qualified)
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn function_decls(&self, qualified: &str) -> Option<&[FunctionDeclSymbol]> {
        self.function_decls
            .get(qualified)
            .map(|entries| entries.as_slice())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn function_decl_by_internal(&self, internal: &str) -> Option<&FunctionDeclSymbol> {
        for decls in self.function_decls.values() {
            if let Some(entry) = decls.iter().find(|entry| entry.internal_name == internal) {
                return Some(entry);
            }
        }
        None
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn constructor_decls(&self, owner: &str) -> Option<&[ConstructorDeclSymbol]> {
        self.constructor_decls
            .get(owner)
            .map(|entries| entries.as_slice())
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn constructor_decl_by_internal(
        &self,
        internal: &str,
    ) -> Option<&ConstructorDeclSymbol> {
        for decls in self.constructor_decls.values() {
            if let Some(entry) = decls.iter().find(|entry| entry.internal_name == internal) {
                return Some(entry);
            }
        }
        None
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn type_const(&self, owner: &str, name: &str) -> Option<&ConstSymbol> {
        self.type_constants
            .get(owner)
            .and_then(|consts| consts.get(name))
    }

    #[allow(dead_code)]
    #[must_use]
    pub(super) fn namespace_const(
        &self,
        namespace: Option<&str>,
        name: &str,
    ) -> Option<&ConstSymbol> {
        let mut search = Vec::new();
        if let Some(ns) = namespace {
            let mut current = ns;
            loop {
                search.push(current.to_string());
                if let Some(idx) = current.rfind("::") {
                    current = &current[..idx];
                } else {
                    break;
                }
            }
        }
        search.push(String::new());
        for candidate in search {
            if let Some(map) = self.namespace_constants.get(&candidate) {
                if let Some(symbol) = map.get(name) {
                    return Some(symbol);
                }
            }
        }
        None
    }

    pub fn update_const_value(&mut self, qualified: &str, value: ConstValue) {
        let (owner, namespace, name) = match self.constants.get_mut(qualified) {
            Some(symbol) => {
                symbol.value = Some(value.clone());
                (
                    symbol.owner.clone(),
                    symbol.namespace.clone(),
                    symbol.name.clone(),
                )
            }
            None => return,
        };

        if let Some(owner_name) = owner {
            if let Some(map) = self.type_constants.get_mut(&owner_name) {
                if let Some(entry) = map.get_mut(&name) {
                    entry.value = Some(value.clone());
                }
            }
        } else {
            let ns_key = namespace.as_deref().unwrap_or("").to_string();
            if let Some(map) = self.namespace_constants.get_mut(&ns_key) {
                if let Some(entry) = map.get_mut(&name) {
                    entry.value = Some(value.clone());
                }
            }
        }
    }

    #[must_use]
    pub(super) fn property_accessor(&self, function: &str) -> Option<&PropertyAccessorLookup> {
        self.property_accessors.get(function)
    }

    pub(super) fn property_symbols(&self, ty: &str) -> Option<&HashMap<String, PropertySymbol>> {
        self.type_properties.get(ty)
    }

    #[must_use]
    pub fn type_names(&self) -> &HashSet<String> {
        &self.types
    }

    #[must_use]
    pub fn property_metadata(&self, type_name: &str, property: &str) -> Option<PropertyMetadata> {
        let props = self.type_properties.get(type_name)?;
        let symbol = props.get(property)?;
        let has_setter = symbol.accessors.contains_key(&PropertyAccessorKind::Set);
        let has_init = symbol.accessors.contains_key(&PropertyAccessorKind::Init);
        Some(PropertyMetadata {
            visibility: symbol.visibility,
            is_static: symbol.is_static,
            has_setter,
            has_init,
            span: symbol.span,
            namespace: symbol.namespace.clone(),
            is_required: symbol.is_required,
        })
    }

    #[must_use]
    pub fn required_property_names(&self, type_name: &str) -> Vec<String> {
        self.type_properties
            .get(type_name)
            .map(|props| {
                props
                    .iter()
                    .filter_map(|(name, symbol)| symbol.is_required.then(|| name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn required_field_names(&self, type_name: &str) -> Vec<String> {
        self.type_fields
            .get(type_name)
            .map(|fields| {
                fields
                    .iter()
                    .filter_map(|(name, symbol)| symbol.is_required.then(|| name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn constant_names(&self) -> impl Iterator<Item = &String> {
        self.constants.keys()
    }

    #[must_use]
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();

            if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else if name_parts.is_empty() {
                prefix_parts.join("::")
            } else {
                prefix_parts.extend(name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests;
