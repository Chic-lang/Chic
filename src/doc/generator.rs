use std::fs;
use std::path::PathBuf;

use crate::doc::extensions::{DocExtensions, resolve_extensions};
use crate::doc::markdown::symbol_kind_label;
use crate::doc::model::{DocDiagnostic, SymbolDocs, SymbolKind};
use crate::doc::template::DocTemplate;
use crate::doc::xml::parse_xml_doc;
use crate::error::{Error, Result};
use crate::frontend::metadata::reflection::{
    MemberDescriptor, MemberKind, ReflectionTables, TypeDescriptor, TypeKind,
};

#[derive(Debug, Clone, Copy)]
pub enum DocOutputLayout {
    SingleFile,
    PerType,
}

#[derive(Debug, Clone)]
pub struct DocGenerationOptions {
    pub output_root: PathBuf,
    pub layout: DocOutputLayout,
    pub template: DocTemplate,
    pub front_matter_template: Option<String>,
    pub banner: Option<String>,
    pub heading_level: usize,
    pub tag_handlers: Vec<String>,
    pub link_resolver: Option<String>,
}

impl Default for DocGenerationOptions {
    fn default() -> Self {
        Self {
            output_root: PathBuf::from("docs/api"),
            layout: DocOutputLayout::PerType,
            template: DocTemplate::none(),
            front_matter_template: None,
            banner: None,
            heading_level: 1,
            tag_handlers: Vec::new(),
            link_resolver: None,
        }
    }
}

#[derive(Debug)]
pub struct GeneratedDocFile {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Default)]
pub struct DocGenerationResult {
    pub files: Vec<GeneratedDocFile>,
    pub diagnostics: Vec<DocDiagnostic>,
}

pub fn generate_markdown(
    tables: &ReflectionTables,
    options: &DocGenerationOptions,
) -> Result<DocGenerationResult> {
    let exts = resolve_extensions(&options.tag_handlers, options.link_resolver.as_deref());
    let mut diagnostics = Vec::new();
    let mut symbols = Vec::new();
    for ty in &tables.types {
        let mut symbol = symbol_from_type(ty, &exts, &mut diagnostics)?;
        symbol.full_name = ty.full_name.clone();
        symbols.push(symbol);
    }

    fs::create_dir_all(&options.output_root).map_err(Error::Io)?;
    let mut result = DocGenerationResult::default();
    result.diagnostics.extend(diagnostics);

    match options.layout {
        DocOutputLayout::SingleFile => {
            let file_path = options.output_root.join("API.md");
            let mut body = String::new();
            for (idx, symbol) in symbols.iter().enumerate() {
                if idx > 0 {
                    body.push('\n');
                }
                let content = render_document(symbol, options, &exts);
                body.push_str(&content);
                if !content.ends_with('\n') {
                    body.push('\n');
                }
            }
            fs::write(&file_path, &body).map_err(Error::Io)?;
            result.files.push(GeneratedDocFile {
                path: file_path,
                contents: body,
            });
        }
        DocOutputLayout::PerType => {
            for symbol in symbols {
                let relative = path_from_symbol(&symbol);
                let file_path = options.output_root.join(relative);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).map_err(Error::Io)?;
                }
                let content = render_document(&symbol, options, &exts);
                fs::write(&file_path, &content).map_err(Error::Io)?;
                result.files.push(GeneratedDocFile {
                    path: file_path,
                    contents: content,
                });
            }
        }
    }

    Ok(result)
}

fn path_from_symbol(symbol: &SymbolDocs) -> PathBuf {
    let mut parts: Vec<String> = symbol.full_name.split("::").map(str::to_string).collect();
    if parts.len() > 1 {
        let file = parts.pop().unwrap_or_else(|| symbol.name.clone());
        let mut path = PathBuf::new();
        for part in parts {
            path.push(part);
        }
        path.push(format!("{file}.md"));
        path
    } else {
        PathBuf::from(format!("{}.md", symbol.name))
    }
}

fn symbol_from_type(
    ty: &TypeDescriptor,
    exts: &DocExtensions,
    diagnostics: &mut Vec<DocDiagnostic>,
) -> Result<SymbolDocs> {
    let doc = parse_xml_doc("", &ty.name, exts);
    diagnostics.extend(doc.diagnostics.clone());
    let mut members = Vec::new();
    for member in &ty.members {
        members.push(symbol_from_member(member, &ty.name, exts, diagnostics)?);
    }
    Ok(SymbolDocs {
        name: display_name(&ty.name),
        full_name: ty.name.clone(),
        kind: symbol_kind_from_type(&ty.kind),
        signature: Some(type_signature(ty)),
        doc,
        parameters: Vec::new(),
        return_type: None,
        members,
    })
}

fn symbol_from_member(
    member: &MemberDescriptor,
    parent: &str,
    exts: &DocExtensions,
    diagnostics: &mut Vec<DocDiagnostic>,
) -> Result<SymbolDocs> {
    let full = format!("{parent}.{}", member.name);
    let doc = parse_xml_doc("", &full, exts);
    diagnostics.extend(doc.diagnostics.clone());
    let mut children = Vec::new();
    for child in &member.children {
        children.push(symbol_from_member(child, &full, exts, diagnostics)?);
    }
    let (parameters, return_type, signature) = member_details(member);
    Ok(SymbolDocs {
        name: member.name.clone(),
        full_name: full,
        kind: symbol_kind_from_member(&member.kind),
        signature,
        doc,
        parameters,
        return_type,
        members: children,
    })
}

fn display_name(name: &str) -> String {
    name.split("::").last().unwrap_or(name).to_string()
}

fn type_signature(ty: &TypeDescriptor) -> String {
    let mut sig = display_name(&ty.name);
    let generics: Vec<String> = ty
        .generic_arguments
        .iter()
        .map(|arg| arg.name.clone())
        .collect();
    if !generics.is_empty() {
        sig.push('<');
        sig.push_str(&generics.join(", "));
        sig.push('>');
    }
    sig
}

fn member_details(
    member: &MemberDescriptor,
) -> (
    Vec<crate::frontend::metadata::reflection::ParameterDescriptor>,
    Option<String>,
    Option<String>,
) {
    match member.kind {
        MemberKind::Method | MemberKind::ExtensionMethod | MemberKind::TraitMethod => {
            if let Some(method) = &member.method {
                let signature = format_signature(
                    &member.name,
                    &method.parameters,
                    Some(&method.return_type.name),
                    member.kind == MemberKind::Constructor,
                );
                (
                    method.parameters.clone(),
                    Some(method.return_type.name.clone()),
                    Some(signature),
                )
            } else {
                (Vec::new(), None, None)
            }
        }
        MemberKind::Constructor => {
            if let Some(ctor) = &member.constructor {
                let signature = format_signature(&member.name, &ctor.parameters, None, true);
                (ctor.parameters.clone(), None, Some(signature))
            } else {
                (Vec::new(), None, None)
            }
        }
        MemberKind::Property => {
            if let Some(prop) = &member.property {
                let signature = format!("{}: {}", member.name, prop.property_type.name.clone());
                (
                    prop.parameters.clone(),
                    Some(prop.property_type.name.clone()),
                    Some(signature),
                )
            } else {
                (Vec::new(), None, None)
            }
        }
        MemberKind::Field | MemberKind::UnionField | MemberKind::UnionView | MemberKind::Const => {
            if let Some(field) = &member.field {
                let sig = format!("{}: {}", member.name, field.field_type.name);
                (Vec::new(), Some(field.field_type.name.clone()), Some(sig))
            } else {
                (Vec::new(), None, Some(member.name.clone()))
            }
        }
        MemberKind::EnumVariant => (Vec::new(), None, Some(member.name.clone())),
        MemberKind::AssociatedType => (Vec::new(), None, Some(member.name.clone())),
    }
}

fn format_signature(
    name: &str,
    parameters: &[crate::frontend::metadata::reflection::ParameterDescriptor],
    return_type: Option<&str>,
    is_constructor: bool,
) -> String {
    let mut sig = String::new();
    sig.push_str(name);
    sig.push('(');
    let mut first = true;
    for param in parameters {
        if !first {
            sig.push_str(", ");
        }
        sig.push_str(&param.name);
        sig.push_str(": ");
        sig.push_str(&param.parameter_type.name);
        first = false;
    }
    sig.push(')');
    if let Some(ret) = return_type {
        if !ret.is_empty() && !is_constructor {
            sig.push_str(" -> ");
            sig.push_str(ret);
        }
    }
    sig
}

fn symbol_kind_from_type(kind: &TypeKind) -> SymbolKind {
    match kind {
        TypeKind::Struct => SymbolKind::Struct,
        TypeKind::Record => SymbolKind::Record,
        TypeKind::Class => SymbolKind::Class,
        TypeKind::Enum => SymbolKind::Enum,
        TypeKind::Interface => SymbolKind::Interface,
        TypeKind::Union => SymbolKind::Union,
        TypeKind::Extension => SymbolKind::Extension,
        TypeKind::Trait => SymbolKind::Trait,
        TypeKind::Delegate => SymbolKind::Delegate,
        TypeKind::Impl => SymbolKind::Impl,
        TypeKind::Function => SymbolKind::Function,
        TypeKind::Const => SymbolKind::Const,
        TypeKind::Static => SymbolKind::Static,
    }
}

fn symbol_kind_from_member(kind: &MemberKind) -> SymbolKind {
    match kind {
        MemberKind::Field | MemberKind::UnionField | MemberKind::UnionView => SymbolKind::Field,
        MemberKind::Property => SymbolKind::Property,
        MemberKind::Method | MemberKind::ExtensionMethod => SymbolKind::Method,
        MemberKind::Constructor => SymbolKind::Constructor,
        MemberKind::Const | MemberKind::EnumVariant => SymbolKind::Const,
        MemberKind::AssociatedType => SymbolKind::TraitMethod,
        MemberKind::TraitMethod => SymbolKind::TraitMethod,
    }
}

fn render_document(
    symbol: &SymbolDocs,
    options: &DocGenerationOptions,
    exts: &DocExtensions,
) -> String {
    let mut out = String::new();
    if let Some(front) = options
        .front_matter_template
        .as_ref()
        .map(|tpl| apply_front_matter(tpl, symbol))
    {
        if !front.is_empty() {
            out.push_str(&front);
            if !front.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }
    if let Some(banner) = &options.banner {
        out.push_str(banner);
        if !banner.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&options.template.render(symbol, exts, options.heading_level));
    out
}

fn apply_front_matter(template: &str, symbol: &SymbolDocs) -> String {
    let mut out = template.to_string();
    out = out.replace("{{name}}", &symbol.name);
    out = out.replace("{{full_name}}", &symbol.full_name);
    out = out.replace("{{title}}", &symbol.name);
    out = out.replace("{{kind}}", symbol_kind_label(&symbol.kind));
    out
}
