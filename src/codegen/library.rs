//! Impact-native reusable library archives (`.clrlib`).
//!
//! The `.clrlib` format mirrors Rust's `.rlib` in spirit: it packages the raw
//! object code alongside metadata describing the compiled module, exported
//! surface area, and dependency set. Front-ends can link these archives without
//! invoking a system linker by extracting the required object members directly.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::chic_kind::ChicKind;
use crate::error::Error;
use crate::frontend::ast::{
    ClassDecl, EnumDecl, ExtensionDecl, InterfaceDecl, Item, Module, NamespaceDecl,
    PropertyAccessorKind, StructDecl, UnionDecl, UsingDirective, UsingKind, Visibility,
};
use crate::frontend::type_utils::type_expr_surface;
use crate::mir::ClassVTable;

const CLRLIB_MAGIC: &[u8; 8] = b"CLRLIB\0\0";
const CLRLIB_VERSION: u32 = 2;

/// Metadata stored in the manifest section of a `.clrlib` archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibManifest {
    pub version: u32,
    pub target_triple: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub exports: Vec<ClrlibExport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<ClrlibAlias>,
    pub dependencies: Vec<ClrlibDependency>,
    pub files: Vec<ClrlibFileEntry>,
    pub class_vtables: Vec<ClrlibClassVTable>,
}

/// Description of a symbol made available to consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibExport {
    pub symbol: String,
    pub category: ExportCategory,
}

/// Categorical marker attached to exported entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExportCategory {
    Function,
    Struct,
    Enum,
    Class,
    Interface,
    Union,
    Extension,
}

/// Description of one import required by the archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibDependency {
    pub reference: String,
    pub kind: DependencyKind,
}

/// Public type alias exposed by the module metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibAlias {
    pub name: String,
    pub target: String,
}

/// Kinds of dependency references tracked in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    UsingNamespace,
    UsingAlias,
    UsingStatic,
    CImport,
}

/// One file bundled into the archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibFileEntry {
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub role: FileRole,
}

/// Logical role for a bundled file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FileRole {
    Object,
    Metadata,
    LlvmIr,
    Other,
}

/// Summary of each class vtable compiled into the archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibClassVTable {
    pub type_name: String,
    pub symbol: String,
    pub version: u64,
    pub slots: Vec<ClrlibClassVTableSlot>,
}

/// Description of a single class vtable slot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClrlibClassVTableSlot {
    pub index: u32,
    pub member: String,
    pub accessor: Option<String>,
    pub symbol: String,
}

/// Bundle a set of object files and metadata into a reusable `.clrlib`.
pub fn write_clrlib_archive(
    module: &Module,
    class_vtables: &[ClassVTable],
    target_triple: &str,
    kind: ChicKind,
    output: &Path,
    object_files: &[(&str, &Path)],
    metadata_files: &[(&str, &Path)],
    extra_files: &[(&str, FileRole, &Path)],
) -> Result<PathBuf, Error> {
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let mut files = Vec::new();
    let mut payloads = Vec::new();

    for (name, path) in object_files {
        let payload = fs::read(path)?;
        let entry = manifest_entry(name, FileRole::Object, &payload);
        files.push(entry);
        payloads.push(((*name).to_string(), payload));
    }

    for (name, path) in metadata_files {
        let payload = fs::read(path)?;
        let entry = manifest_entry(name, FileRole::Metadata, &payload);
        files.push(entry);
        payloads.push(((*name).to_string(), payload));
    }

    for (name, role, path) in extra_files {
        let payload = fs::read(path)?;
        let entry = manifest_entry(name, role.clone(), &payload);
        files.push(entry);
        payloads.push(((*name).to_string(), payload));
    }

    let manifest = ClrlibManifest {
        version: CLRLIB_VERSION,
        target_triple: target_triple.to_string(),
        kind: kind.as_str().to_string(),
        namespace: module.namespace.clone(),
        exports: collect_exports(module),
        aliases: collect_aliases(module),
        dependencies: collect_dependencies(module),
        files,
        class_vtables: summarize_class_vtables(class_vtables),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|err| Error::Codegen(format!("failed to encode clrlib manifest: {err}")))?;

    write_package(output, &manifest_bytes, &payloads)?;
    Ok(output.to_path_buf())
}

fn manifest_entry(name: &str, role: FileRole, payload: &[u8]) -> ClrlibFileEntry {
    let mut hasher = Hasher::new();
    hasher.update(payload);
    let hash = hasher.finalize().to_hex().to_string();
    ClrlibFileEntry {
        name: name.to_string(),
        size: payload.len() as u64,
        hash,
        role,
    }
}

fn write_package(
    path: &Path,
    manifest: &[u8],
    payloads: &[(String, Vec<u8>)],
) -> Result<(), Error> {
    let mut file = fs::File::create(path)?;
    file.write_all(CLRLIB_MAGIC)?;
    write_u32(&mut file, CLRLIB_VERSION)?;
    write_u32(&mut file, manifest.len() as u32)?;
    file.write_all(manifest)?;

    for (name, payload) in payloads {
        write_u32(&mut file, name.len() as u32)?;
        write_u64(&mut file, payload.len() as u64)?;
        file.write_all(name.as_bytes())?;
        file.write_all(payload)?;
    }

    file.flush()?;
    Ok(())
}

fn summarize_class_vtables(class_vtables: &[ClassVTable]) -> Vec<ClrlibClassVTable> {
    let mut tables = Vec::with_capacity(class_vtables.len());
    for table in class_vtables {
        let slots = table
            .slots
            .iter()
            .map(|slot| ClrlibClassVTableSlot {
                index: slot.slot_index,
                member: slot.member.clone(),
                accessor: slot
                    .accessor
                    .map(accessor_label)
                    .map(|label| label.to_string()),
                symbol: slot.symbol.clone(),
            })
            .collect();
        tables.push(ClrlibClassVTable {
            type_name: table.type_name.clone(),
            symbol: table.symbol.clone(),
            version: table.version,
            slots,
        });
    }
    tables
}

fn accessor_label(kind: PropertyAccessorKind) -> &'static str {
    match kind {
        PropertyAccessorKind::Get => "get",
        PropertyAccessorKind::Set => "set",
        PropertyAccessorKind::Init => "init",
    }
}

fn write_u32(writer: &mut fs::File, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_u64(writer: &mut fs::File, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn collect_exports(module: &Module) -> Vec<ClrlibExport> {
    let mut exports = Vec::new();
    let mut scope = Vec::new();
    if let Some(ns) = &module.namespace {
        scope.extend(ns.split('.').map(str::to_string));
    }
    collect_exports_from_items(&module.items, &mut scope, &mut exports);
    exports
}

fn collect_exports_from_items(
    items: &[Item],
    scope: &mut Vec<String>,
    exports: &mut Vec<ClrlibExport>,
) {
    for item in items {
        match item {
            Item::Function(func) => {
                if func.visibility == Visibility::Public {
                    let symbol = qualify(scope, &func.name);
                    exports.push(ClrlibExport {
                        symbol,
                        category: ExportCategory::Function,
                    });
                }
            }
            Item::Struct(def) => push_type_export(def, ExportCategory::Struct, scope, exports),
            Item::Union(def) => push_union_export(def, scope, exports),
            Item::Enum(def) => push_enum_export(def, scope, exports),
            Item::Delegate(_) => {}
            Item::Const(_) => {}
            Item::Class(def) => push_class_export(def, scope, exports),
            Item::Interface(def) => push_interface_export(def, scope, exports),
            Item::Extension(def) => push_extension_export(def, scope, exports),
            Item::Trait(_) | Item::Impl(_) => {}
            Item::Namespace(ns) => {
                push_namespace(ns, scope, exports);
            }
            Item::Static(_) | Item::Import(_) | Item::TestCase(_) | Item::TypeAlias(_) => {}
        }
    }
}

fn push_namespace(ns: &NamespaceDecl, scope: &mut Vec<String>, exports: &mut Vec<ClrlibExport>) {
    let pushed = extend_scope_with_namespace(scope, &ns.name);
    collect_exports_from_items(&ns.items, scope, exports);
    scope.truncate(scope.len().saturating_sub(pushed));
}

fn extend_scope_with_namespace(scope: &mut Vec<String>, namespace: &str) -> usize {
    let parts: Vec<String> = namespace
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();
    let mut prefix = 0usize;
    while prefix < parts.len() && prefix < scope.len() && scope[prefix] == parts[prefix] {
        prefix += 1;
    }
    for part in parts.iter().skip(prefix) {
        scope.push(part.clone());
    }
    parts.len().saturating_sub(prefix)
}

fn push_type_export(
    def: &StructDecl,
    category: ExportCategory,
    scope: &mut Vec<String>,
    exports: &mut Vec<ClrlibExport>,
) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &def.name);
    exports.push(ClrlibExport { symbol, category });
}

fn push_union_export(def: &UnionDecl, scope: &mut Vec<String>, exports: &mut Vec<ClrlibExport>) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &def.name);
    exports.push(ClrlibExport {
        symbol,
        category: ExportCategory::Union,
    });
}

fn push_enum_export(def: &EnumDecl, scope: &mut Vec<String>, exports: &mut Vec<ClrlibExport>) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &def.name);
    exports.push(ClrlibExport {
        symbol,
        category: ExportCategory::Enum,
    });
}

fn push_class_export(def: &ClassDecl, scope: &mut Vec<String>, exports: &mut Vec<ClrlibExport>) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &def.name);
    exports.push(ClrlibExport {
        symbol,
        category: ExportCategory::Class,
    });
}

fn push_interface_export(
    def: &InterfaceDecl,
    scope: &mut Vec<String>,
    exports: &mut Vec<ClrlibExport>,
) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &def.name);
    exports.push(ClrlibExport {
        symbol,
        category: ExportCategory::Interface,
    });
}

fn push_extension_export(
    def: &ExtensionDecl,
    scope: &mut Vec<String>,
    exports: &mut Vec<ClrlibExport>,
) {
    if def.visibility != Visibility::Public {
        return;
    }
    let symbol = qualify(scope, &format!("Extension<{}>", def.target.name));
    exports.push(ClrlibExport {
        symbol,
        category: ExportCategory::Extension,
    });
}

fn collect_dependencies(module: &Module) -> Vec<ClrlibDependency> {
    let mut deps = Vec::new();
    collect_dependencies_from_items(&module.items, &mut deps);
    deps
}

fn collect_dependencies_from_items(items: &[Item], deps: &mut Vec<ClrlibDependency>) {
    for item in items {
        match item {
            Item::Import(using) => deps.push(dependency_from_using(using)),
            Item::Namespace(ns) => collect_dependencies_from_items(&ns.items, deps),
            _ => {}
        }
    }
}

fn dependency_from_using(using: &UsingDirective) -> ClrlibDependency {
    match &using.kind {
        UsingKind::Namespace { path } => ClrlibDependency {
            reference: path.clone(),
            kind: DependencyKind::UsingNamespace,
        },
        UsingKind::Alias { alias, target } => ClrlibDependency {
            reference: format!("{alias}={target}"),
            kind: DependencyKind::UsingAlias,
        },
        UsingKind::Static { target } => ClrlibDependency {
            reference: target.clone(),
            kind: DependencyKind::UsingStatic,
        },
        UsingKind::CImport { header } => ClrlibDependency {
            reference: header.clone(),
            kind: DependencyKind::CImport,
        },
    }
}

fn qualify(scope: &[String], name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else if name.is_empty() {
        scope.join("::")
    } else {
        format!("{}::{name}", scope.join("::"))
    }
}

fn collect_aliases(module: &Module) -> Vec<ClrlibAlias> {
    let mut aliases = Vec::new();
    let mut scope = Vec::new();
    if let Some(ns) = &module.namespace {
        scope.extend(
            ns.split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string),
        );
    }
    collect_aliases_from_items(&module.items, &mut scope, &mut aliases);
    aliases
}

fn collect_aliases_from_items(
    items: &[Item],
    scope: &mut Vec<String>,
    aliases: &mut Vec<ClrlibAlias>,
) {
    for item in items {
        match item {
            Item::Namespace(ns) => {
                let pushed = extend_scope_with_namespace(scope, &ns.name);
                collect_aliases_from_items(&ns.items, scope, aliases);
                scope.truncate(scope.len().saturating_sub(pushed));
            }
            Item::Struct(def) => {
                let pushed = extend_scope_with_namespace(scope, &def.name);
                collect_aliases_from_items(&def.nested_types, scope, aliases);
                scope.truncate(scope.len().saturating_sub(pushed));
            }
            Item::Class(def) => {
                let pushed = extend_scope_with_namespace(scope, &def.name);
                collect_aliases_from_items(&def.nested_types, scope, aliases);
                scope.truncate(scope.len().saturating_sub(pushed));
            }
            Item::TypeAlias(alias) if alias.visibility == Visibility::Public => {
                let name = qualify(scope, &alias.name);
                let mut rendered = name.clone();
                if let Some(generics) = &alias.generics {
                    if !generics.params.is_empty() {
                        let params = generics
                            .params
                            .iter()
                            .map(|param| param.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");
                        rendered = format!("{rendered}<{params}>");
                    }
                }
                let target = type_expr_surface(&alias.target);
                aliases.push(ClrlibAlias {
                    name: rendered,
                    target,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{
        DocComment, FunctionDecl, MemberDispatch, Parameter, Signature, TypeAliasDecl, TypeExpr,
    };
    use crate::mir::ClassVTableSlot;
    use tempfile::tempdir;

    fn sample_module() -> Module {
        let mut module = Module::new(Some("Interop".to_string()));
        module.push_item(Item::TypeAlias(TypeAliasDecl {
            visibility: Visibility::Public,
            name: "Sample".to_string(),
            target: TypeExpr::simple("ushort"),
            generics: None,
            attributes: Vec::new(),
            doc: None,
            span: None,
        }));
        module.push_item(Item::Function(FunctionDecl {
            visibility: Visibility::Public,
            name: "Add".to_string(),
            name_span: None,
            signature: Signature {
                parameters: vec![
                    Parameter {
                        binding: crate::frontend::ast::BindingModifier::Value,
                        binding_nullable: false,
                        name: "left".to_string(),
                        name_span: None,
                        ty: TypeExpr::simple("int"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: false,
                    },
                    Parameter {
                        binding: crate::frontend::ast::BindingModifier::Value,
                        binding_nullable: false,
                        name: "right".to_string(),
                        name_span: None,
                        ty: TypeExpr::simple("int"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: false,
                    },
                ],
                return_type: TypeExpr::simple("int"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: None,
            is_async: false,
            is_constexpr: false,
            doc: Some(DocComment::new(vec!["Adds two integers.".into()])),
            modifiers: Vec::new(),
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        }));
        module.push_item(Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: "Point".to_string(),
            fields: vec![],
            properties: Vec::new(),
            constructors: Vec::new(),
            consts: Vec::new(),
            methods: Vec::new(),
            nested_types: Vec::new(),
            bases: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            doc: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            mmio: None,
            generics: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }));
        module.push_item(Item::Import(UsingDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: UsingKind::Namespace {
                path: "Std.Math".to_string(),
            },
        }));
        module
    }

    #[test]
    fn collects_exports_for_public_items() {
        let module = sample_module();
        let exports = collect_exports(&module);
        assert_eq!(exports.len(), 2);
        assert!(exports.iter().any(|export| export.symbol == "Interop::Add"
            && matches!(export.category, ExportCategory::Function)));
        assert!(
            exports
                .iter()
                .any(|export| export.symbol == "Interop::Point"
                    && matches!(export.category, ExportCategory::Struct))
        );
    }

    #[test]
    fn collects_aliases_for_public_items() {
        let module = sample_module();
        let aliases = collect_aliases(&module);
        assert!(
            aliases
                .iter()
                .any(|alias| alias.name == "Interop::Sample" && alias.target == "ushort"),
            "missing alias in manifest aliases: {aliases:?}"
        );
    }

    #[test]
    fn collects_dependencies_from_using_directives() {
        let module = sample_module();
        let deps = collect_dependencies(&module);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].reference, "Std.Math");
        assert!(matches!(deps[0].kind, DependencyKind::UsingNamespace));
    }

    #[test]
    fn writes_clrlib_archive_with_manifest_and_payloads() -> Result<(), Error> {
        let module = sample_module();
        let dir = tempdir().unwrap();
        let object_path = dir.path().join("module.o");
        let object_bytes = b"object-bytes";
        fs::write(&object_path, object_bytes)?;
        let metadata_path = dir.path().join("module.meta.o");
        fs::write(&metadata_path, b"metadata-bytes")?;
        let ir_path = dir.path().join("module.ll");
        fs::write(&ir_path, "define void @f() {}")?;

        let archive_path = dir.path().join("module.clrlib");
        write_clrlib_archive(
            &module,
            &[],
            "x86_64-unknown-linux-gnu",
            ChicKind::StaticLibrary,
            &archive_path,
            &[("objects/module.o", &object_path)],
            &[("objects/module.meta.o", &metadata_path)],
            &[("ir/module.ll", FileRole::LlvmIr, &ir_path)],
        )?;

        let bytes = fs::read(&archive_path)?;
        assert_eq!(&bytes[..CLRLIB_MAGIC.len()], CLRLIB_MAGIC);
        let manifest_len = u32::from_le_bytes(
            bytes[CLRLIB_MAGIC.len() + 4..CLRLIB_MAGIC.len() + 8]
                .try_into()
                .unwrap(),
        );
        let manifest_offset = CLRLIB_MAGIC.len() + 8;
        let manifest_bytes = &bytes[manifest_offset..manifest_offset + manifest_len as usize];
        let manifest: ClrlibManifest = serde_json::from_slice(manifest_bytes).unwrap();
        assert_eq!(manifest.files.len(), 3);
        assert!(manifest.files.iter().any(|entry| {
            entry.name == "objects/module.o"
                && entry.size == object_bytes.len() as u64
                && matches!(entry.role, FileRole::Object)
        }));
        assert!(
            manifest
                .files
                .iter()
                .any(|entry| matches!(entry.role, FileRole::LlvmIr))
        );
        Ok(())
    }

    #[test]
    fn clrlib_manifest_tracks_reflection_metadata() -> Result<(), Error> {
        let module = sample_module();
        let dir = tempdir().unwrap();
        let object_path = dir.path().join("module.o");
        fs::write(&object_path, b"object-bytes")?;
        let metadata_path = dir.path().join("module.meta.o");
        fs::write(&metadata_path, b"metadata-bytes")?;
        let reflection_path = dir.path().join("module.reflect.json");
        fs::write(&reflection_path, br#"{"types":[]}"#)?;

        let archive_path = dir.path().join("module.clrlib");
        write_clrlib_archive(
            &module,
            &[],
            "x86_64-unknown-linux-gnu",
            ChicKind::StaticLibrary,
            &archive_path,
            &[("objects/module.o", &object_path)],
            &[
                ("objects/module.meta.o", &metadata_path),
                ("metadata/module.reflect.json", &reflection_path),
            ],
            &[],
        )?;

        let bytes = fs::read(&archive_path)?;
        let manifest_len = u32::from_le_bytes(
            bytes[CLRLIB_MAGIC.len() + 4..CLRLIB_MAGIC.len() + 8]
                .try_into()
                .unwrap(),
        );
        let manifest_offset = CLRLIB_MAGIC.len() + 8;
        let manifest_bytes = &bytes[manifest_offset..manifest_offset + manifest_len as usize];
        let manifest: ClrlibManifest = serde_json::from_slice(manifest_bytes).unwrap();
        assert!(
            manifest
                .files
                .iter()
                .any(|entry| entry.name == "metadata/module.reflect.json"),
            "reflection metadata entry missing from clrlib manifest"
        );
        assert!(manifest.class_vtables.is_empty());
        Ok(())
    }

    #[test]
    fn manifest_embeds_class_vtable_metadata() -> Result<(), Error> {
        let module = sample_module();
        let dir = tempdir().unwrap();
        let object_path = dir.path().join("module.o");
        fs::write(&object_path, b"object")?;
        let metadata_path = dir.path().join("module.meta.o");
        fs::write(&metadata_path, b"metadata")?;

        let vtables = vec![ClassVTable {
            type_name: "Interop::Widget".into(),
            symbol: "__class_vtable_Interop__Widget".into(),
            version: 0xA5A5_F0F0_DADA_C0DE,
            slots: vec![
                ClassVTableSlot {
                    slot_index: 0,
                    member: "get_Value".into(),
                    accessor: Some(PropertyAccessorKind::Get),
                    symbol: "Interop::Widget::get_Value#1".into(),
                },
                ClassVTableSlot {
                    slot_index: 1,
                    member: "set_Value".into(),
                    accessor: Some(PropertyAccessorKind::Set),
                    symbol: "Interop::Widget::set_Value#1".into(),
                },
            ],
        }];

        let archive_path = dir.path().join("module.clrlib");
        write_clrlib_archive(
            &module,
            &vtables,
            "x86_64-unknown-linux-gnu",
            ChicKind::StaticLibrary,
            &archive_path,
            &[("objects/module.o", &object_path)],
            &[("objects/module.meta.o", &metadata_path)],
            &[],
        )?;

        let bytes = fs::read(&archive_path)?;
        let manifest_len = u32::from_le_bytes(
            bytes[CLRLIB_MAGIC.len() + 4..CLRLIB_MAGIC.len() + 8]
                .try_into()
                .unwrap(),
        );
        let manifest_offset = CLRLIB_MAGIC.len() + 8;
        let manifest_bytes = &bytes[manifest_offset..manifest_offset + manifest_len as usize];
        let manifest: ClrlibManifest = serde_json::from_slice(manifest_bytes).unwrap();
        assert_eq!(manifest.class_vtables.len(), 1);
        let table = &manifest.class_vtables[0];
        assert_eq!(table.type_name, "Interop::Widget");
        assert_eq!(table.symbol, "__class_vtable_Interop__Widget");
        assert_eq!(table.version, 0xA5A5_F0F0_DADA_C0DE);
        assert_eq!(table.slots.len(), 2);
        assert_eq!(table.slots[0].member, "get_Value");
        assert_eq!(table.slots[0].accessor.as_deref(), Some("get"));
        assert_eq!(table.slots[1].accessor.as_deref(), Some("set"));
        Ok(())
    }
}
