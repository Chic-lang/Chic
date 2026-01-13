//! Module attribute and documentation helpers for metadata emission.

use std::fmt::Write;

use crate::frontend::ast::{
    ClassDecl, ClassMember, ConstructorDecl, ConstructorKind, DocComment, EnumDecl, ExtensionDecl,
    ExtensionMember, FieldDecl, FunctionDecl, InlineAttr, InterfaceDecl, InterfaceMember, Item,
    Module, PropertyAccessorKind, PropertyDecl, StructDecl, TestCaseDecl, UnionDecl, UnionField,
    UnionMember, UnionViewDecl, UsingDirective, UsingKind,
};
use crate::mir::MirModule;

use super::schema::MetadataWriter;
use super::types::{TypeMetadataCache, TypeMetadataEntry, TypeMetadataFingerprint};

/// Append module-level attributes (e.g., `no_std`, `global_allocator`) into the payload.
pub(crate) fn append_module_attributes(payload: &mut MetadataWriter, mir: &MirModule) {
    if mir.attributes.is_no_std() {
        payload.push_str("profile=no_std\n");
    }
    if mir.attributes.is_no_main() {
        payload.push_str("no_main=1\n");
    }
    if let Some(global_allocator) = &mir.attributes.global_allocator {
        let _ = writeln!(payload, "global_allocator={}", global_allocator.type_name);
        if let Some(target) = &global_allocator.target {
            let _ = writeln!(payload, "global_allocator_target={target}");
        }
    }
}

/// Append documentation derived from AST items into the payload.
pub(crate) fn append_doc_metadata(
    payload: &mut MetadataWriter,
    module: &Module,
    type_cache: &mut TypeMetadataCache,
) {
    let mut scope = Vec::new();
    if let Some(ns) = &module.namespace {
        scope.extend(ns.split('.').map(str::to_string));
    }
    append_docs_from_items(payload, &module.items, &mut scope, type_cache);
}

pub(crate) fn append_inline_metadata(payload: &mut MetadataWriter, module: &Module) {
    let mut scope = Vec::new();
    if let Some(ns) = &module.namespace {
        scope.extend(ns.split('.').map(str::to_string));
    }
    append_inline_from_items(payload, &module.items, &mut scope);
}

fn append_docs_from_items(
    payload: &mut MetadataWriter,
    items: &[Item],
    scope: &mut Vec<String>,
    type_cache: &mut TypeMetadataCache,
) {
    for item in items {
        match item {
            Item::Function(func) => append_function_doc(payload, scope, func),
            Item::Struct(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
                append_struct_docs(payload, scope, def);
            }
            Item::Union(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
                append_union_docs(payload, scope, def);
            }
            Item::Enum(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
                append_enum_docs(payload, scope, def);
            }
            Item::Delegate(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
            }
            Item::Class(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
                append_class_docs(payload, scope, def);
            }
            Item::Interface(def) => {
                let name = qualified_name(scope, &def.name);
                register_type(type_cache, name);
                append_interface_docs(payload, scope, def);
            }
            Item::Extension(def) => {
                let label = format!("extension {}", def.target.name);
                let name = qualified_name(scope, &label);
                register_type(type_cache, name);
                append_extension_docs(payload, scope, def);
            }
            Item::TestCase(test) => append_testcase_doc(payload, scope, test),
            Item::Const(_) | Item::Static(_) | Item::TypeAlias(_) => {}
            Item::Trait(_) | Item::Impl(_) => {}
            Item::Namespace(ns) => {
                let name = qualified_name(scope, &ns.name);
                if let Some(doc) = &ns.doc {
                    append_doc_entry(payload, &name, doc);
                }
                let parts: Vec<String> = ns.name.split('.').map(str::to_string).collect();
                let mut prefix = 0;
                while prefix < parts.len() && prefix < scope.len() && scope[prefix] == parts[prefix]
                {
                    prefix += 1;
                }
                for part in parts.iter().skip(prefix) {
                    scope.push(part.clone());
                }
                append_docs_from_items(payload, &ns.items, scope, type_cache);
                for _ in parts.iter().skip(prefix) {
                    scope.pop();
                }
            }
            Item::Import(using) => append_using_doc(payload, scope, using),
        }
    }
}

fn append_inline_from_items(payload: &mut MetadataWriter, items: &[Item], scope: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Struct(def) => {
                if matches!(def.inline_attr, Some(InlineAttr::Cross)) {
                    let name = qualified_name(scope, &def.name);
                    let _ = writeln!(payload, "inline:{name}=cross");
                }
                if !def.nested_types.is_empty() {
                    scope.push(def.name.clone());
                    append_inline_from_items(payload, &def.nested_types, scope);
                    scope.pop();
                }
            }
            Item::Namespace(ns) => {
                let parts: Vec<String> = ns.name.split('.').map(str::to_string).collect();
                let mut prefix = 0;
                while prefix < parts.len() && prefix < scope.len() && scope[prefix] == parts[prefix]
                {
                    prefix += 1;
                }
                for part in parts.iter().skip(prefix) {
                    scope.push(part.clone());
                }
                append_inline_from_items(payload, &ns.items, scope);
                for _ in parts.iter().skip(prefix) {
                    scope.pop();
                }
            }
            _ => {}
        }
    }
}

fn qualified_name(scope: &[String], name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else if name.is_empty() {
        scope.join(".")
    } else {
        format!("{}.{name}", scope.join("."))
    }
}

fn append_doc_entry(payload: &mut MetadataWriter, path: &str, doc: &DocComment) {
    if doc.is_empty() {
        return;
    }
    payload.push_str("doc:");
    payload.push_str(path);
    payload.push('=');
    let text = doc.as_text().replace('\n', "\\n");
    payload.push_str(&text);
    payload.push('\n');
}

fn append_function_doc(payload: &mut MetadataWriter, scope: &[String], func: &FunctionDecl) {
    if let Some(doc) = &func.doc {
        let name = qualified_name(scope, &func.name);
        append_doc_entry(payload, &name, doc);
    }
}

fn append_constructor_doc(payload: &mut MetadataWriter, scope: &[String], ctor: &ConstructorDecl) {
    if let Some(doc) = &ctor.doc {
        let base = match ctor.kind {
            ConstructorKind::Convenience => "convenience init",
            ConstructorKind::Designated => "init",
        };
        let name = qualified_name(scope, base);
        append_doc_entry(payload, &name, doc);
    }
}

fn append_field_doc(payload: &mut MetadataWriter, scope: &[String], field: &FieldDecl) {
    if let Some(doc) = &field.doc {
        let display_name = field.display_name.as_deref().unwrap_or(&field.name);
        let name = qualified_name(scope, display_name);
        append_doc_entry(payload, &name, doc);
    }
}

fn append_property_doc(payload: &mut MetadataWriter, scope: &[String], property: &PropertyDecl) {
    if let Some(doc) = &property.doc {
        let name = qualified_name(scope, &property.name);
        append_doc_entry(payload, &name, doc);
    }

    for accessor in &property.accessors {
        if let Some(doc) = &accessor.doc {
            let label = format!("{}::{}", property.name, accessor_kind_label(accessor.kind));
            let name = qualified_name(scope, &label);
            append_doc_entry(payload, &name, doc);
        }
    }
}

fn accessor_kind_label(kind: PropertyAccessorKind) -> &'static str {
    match kind {
        PropertyAccessorKind::Get => "get",
        PropertyAccessorKind::Set => "set",
        PropertyAccessorKind::Init => "init",
    }
}

fn append_struct_docs(payload: &mut MetadataWriter, scope: &mut Vec<String>, def: &StructDecl) {
    let name = qualified_name(scope, &def.name);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(def.name.clone());
    for field in &def.fields {
        append_field_doc(payload, scope, field);
    }
    for property in &def.properties {
        append_property_doc(payload, scope, property);
    }
    scope.pop();
}

fn append_union_field_doc(payload: &mut MetadataWriter, scope: &[String], field: &UnionField) {
    if let Some(doc) = &field.doc {
        let name = qualified_name(scope, &field.name);
        append_doc_entry(payload, &name, doc);
    }
}

fn append_union_view_docs(
    payload: &mut MetadataWriter,
    scope: &mut Vec<String>,
    view: &UnionViewDecl,
) {
    let name = qualified_name(scope, &view.name);
    if let Some(doc) = &view.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(view.name.clone());
    for field in &view.fields {
        append_field_doc(payload, scope, field);
    }
    scope.pop();
}

fn append_union_docs(payload: &mut MetadataWriter, scope: &mut Vec<String>, def: &UnionDecl) {
    let name = qualified_name(scope, &def.name);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(def.name.clone());
    for member in &def.members {
        match member {
            UnionMember::Field(field) => append_union_field_doc(payload, scope, field),
            UnionMember::View(view) => append_union_view_docs(payload, scope, view),
        }
    }
    scope.pop();
}

fn append_enum_docs(payload: &mut MetadataWriter, scope: &mut Vec<String>, def: &EnumDecl) {
    let name = qualified_name(scope, &def.name);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(def.name.clone());
    for variant in &def.variants {
        let variant_name = qualified_name(scope, &variant.name);
        if let Some(doc) = &variant.doc {
            append_doc_entry(payload, &variant_name, doc);
        }
        scope.push(variant.name.clone());
        for field in &variant.fields {
            append_field_doc(payload, scope, field);
        }
        scope.pop();
    }
    scope.pop();
}

fn append_class_docs(payload: &mut MetadataWriter, scope: &mut Vec<String>, def: &ClassDecl) {
    let name = qualified_name(scope, &def.name);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(def.name.clone());
    for member in &def.members {
        match member {
            ClassMember::Field(field) => append_field_doc(payload, scope, field),
            ClassMember::Method(func) => append_function_doc(payload, scope, func),
            ClassMember::Property(property) => append_property_doc(payload, scope, property),
            ClassMember::Constructor(ctor) => append_constructor_doc(payload, scope, ctor),
            ClassMember::Const(_) => {}
        }
    }
    scope.pop();
}

fn append_interface_docs(
    payload: &mut MetadataWriter,
    scope: &mut Vec<String>,
    def: &InterfaceDecl,
) {
    let name = qualified_name(scope, &def.name);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(def.name.clone());
    for member in &def.members {
        match member {
            InterfaceMember::Method(func) => append_function_doc(payload, scope, func),
            InterfaceMember::Property(property) => append_property_doc(payload, scope, property),
            InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => {}
        }
    }
    scope.pop();
}

fn append_extension_docs(
    payload: &mut MetadataWriter,
    scope: &mut Vec<String>,
    def: &ExtensionDecl,
) {
    let target_name = &def.target.name;
    let label = format!("extension {target_name}");
    let name = qualified_name(scope, &label);
    if let Some(doc) = &def.doc {
        append_doc_entry(payload, &name, doc);
    }
    scope.push(label);
    for member in &def.members {
        match member {
            ExtensionMember::Method(method) => {
                append_function_doc(payload, scope, &method.function)
            }
        }
    }
    scope.pop();
}

fn append_testcase_doc(payload: &mut MetadataWriter, scope: &[String], test: &TestCaseDecl) {
    if let Some(doc) = &test.doc {
        let name = qualified_name(scope, &test.name);
        append_doc_entry(payload, &name, doc);
    }
}

fn append_using_doc(payload: &mut MetadataWriter, scope: &[String], using: &UsingDirective) {
    if let Some(doc) = &using.doc {
        let prefix = if using.is_global { "global " } else { "" };
        let descriptor = match &using.kind {
            UsingKind::Namespace { path } => format!("{prefix}import {path}"),
            UsingKind::Alias { alias, target } => format!("{prefix}import {alias} = {target}"),
            UsingKind::Static { target } => format!("{prefix}import static {target}"),
            UsingKind::CImport { header } => format!("@cimport \"{header}\""),
        };
        let name = qualified_name(scope, &descriptor);
        append_doc_entry(payload, &name, doc);
    }
}

fn register_type(cache: &mut TypeMetadataCache, qualified_name: String) {
    let fingerprint = TypeMetadataFingerprint::new(qualified_name);
    let _ = cache.ensure_with(fingerprint, |fp| {
        TypeMetadataEntry::new(fp.as_str().to_string())
    });
}
