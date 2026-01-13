//! Formatting helpers for Chic textual code generation.

use std::fmt::{self, Write};

use crate::codegen::text::format::pretty::{
    format_const_declarators, format_parameter, format_union_modifiers, format_visibility,
};

use crate::frontend::ast::{
    ClassDecl, ClassMember, ConstDeclaration, ConstItemDecl, ConstMemberDecl, ConstructorDecl,
    ConstructorInitTarget, ConstructorKind, DocComment, EnumDecl, ExtensionDecl, ExtensionMember,
    FieldDecl, FunctionDecl, InterfaceDecl, InterfaceMember, Item, Module, NamespaceDecl,
    PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind, PropertyDecl, Statement,
    StructDecl, TestCaseDecl, TypeAliasDecl, UnionDecl, UnionField, UnionMember, UnionViewDecl,
    UsingDirective, UsingKind, Visibility,
};

/// Write module-level constructs into the provided output buffer.
pub(crate) fn write_module<W: Write>(out: &mut W, module: &Module) -> fmt::Result {
    if let Some(ns) = &module.namespace {
        writeln!(out, "package-namespace {ns}")?;
    }

    write_items(out, &module.items, 0)?;
    Ok(())
}

fn write_items<W: Write>(out: &mut W, items: &[Item], indent: usize) -> fmt::Result {
    for item in items {
        match item {
            Item::Import(import) => {
                write_doc(out, import.doc.as_ref(), indent)?;
                write_import(out, import, indent)?;
            }
            Item::Const(const_item) => write_const_item(out, const_item, indent)?,
            Item::Namespace(ns) => write_namespace(out, ns, indent)?,
            Item::Function(func) => write_function(out, func, indent)?,
            Item::Struct(def) => write_struct(out, def, indent)?,
            Item::Union(def) => write_union(out, def, indent)?,
            Item::Enum(def) => write_enum(out, def, indent)?,
            Item::Class(def) => write_class(out, def, indent)?,
            Item::Interface(def) => write_interface(out, def, indent)?,
            Item::Delegate(def) => {
                write_doc(out, def.doc.as_ref(), indent)?;
                let params = def
                    .signature
                    .parameters
                    .iter()
                    .map(format_parameter)
                    .collect::<Vec<_>>()
                    .join(", ");
                let generics = def
                    .generics
                    .as_ref()
                    .map(|def| {
                        let names = def
                            .params
                            .iter()
                            .map(|param| param.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("<{names}>")
                    })
                    .unwrap_or_default();
                write_indent(out, indent)?;
                writeln!(
                    out,
                    "{}delegate {}{}({}) -> {};",
                    format_visibility(def.visibility),
                    def.name,
                    generics,
                    params,
                    def.signature.return_type.name
                )?;
            }
            Item::TypeAlias(alias) => write_type_alias(out, alias, indent)?,
            Item::Extension(def) => write_extension(out, def, indent)?,
            Item::Static(_) => {}
            Item::Trait(_) | Item::Impl(_) => {}
            Item::TestCase(test) => write_testcase(out, test, indent)?,
        }
    }
    Ok(())
}

fn write_namespace<W: Write>(out: &mut W, ns: &NamespaceDecl, indent: usize) -> fmt::Result {
    write_doc(out, ns.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    writeln!(out, "namespace {} {{", ns.name)?;
    write_items(out, &ns.items, indent + 1)?;
    write_indent(out, indent)?;
    writeln!(out, "}} // namespace {}", ns.name)?;
    Ok(())
}

fn write_type_alias<W: Write>(out: &mut W, alias: &TypeAliasDecl, indent: usize) -> fmt::Result {
    write_doc(out, alias.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let visibility = format_visibility(alias.visibility);
    let generics = alias
        .generics
        .as_ref()
        .map(|params| {
            let names = params
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            if names.is_empty() {
                String::new()
            } else {
                format!("<{names}>")
            }
        })
        .unwrap_or_default();
    writeln!(
        out,
        "{visibility}typealias {}{} = {};",
        alias.name, generics, alias.target.name
    )
}

fn write_function<W: Write>(out: &mut W, func: &FunctionDecl, indent: usize) -> fmt::Result {
    write_doc(out, func.doc.as_ref(), indent)?;
    let params = func
        .signature
        .parameters
        .iter()
        .map(format_parameter)
        .collect::<Vec<_>>()
        .join(", ");
    let visibility = format_visibility(func.visibility);
    let async_kw = if func.is_async { "async " } else { "" };
    write_indent(out, indent)?;
    if let Some(body) = &func.body {
        writeln!(
            out,
            "{}{}fn {}({}) -> {} {{",
            visibility, async_kw, func.name, params, func.signature.return_type.name
        )?;
        for statement in &body.statements {
            write_statement(out, statement, indent + 1)?;
        }
        write_indent(out, indent)?;
        writeln!(out, "}}")?;
    } else {
        writeln!(
            out,
            "{}{}fn {}({}) -> {};",
            visibility, async_kw, func.name, params, func.signature.return_type.name
        )?;
    }
    Ok(())
}

fn write_constructor<W: Write>(out: &mut W, ctor: &ConstructorDecl, indent: usize) -> fmt::Result {
    write_doc(out, ctor.doc.as_ref(), indent)?;
    let params = ctor
        .parameters
        .iter()
        .map(format_parameter)
        .collect::<Vec<_>>()
        .join(", ");
    let visibility = format_visibility(ctor.visibility);
    let prefix = match ctor.kind {
        ConstructorKind::Convenience => format!("{visibility}convenience init({params})"),
        ConstructorKind::Designated => format!("{visibility}init({params})"),
    };
    let initializer = ctor
        .initializer
        .as_ref()
        .map(|init| {
            let target = match init.target {
                ConstructorInitTarget::SelfType => "self",
                ConstructorInitTarget::Super => "super",
            };
            let args = init
                .arguments
                .iter()
                .map(|expr| expr.text.trim())
                .collect::<Vec<_>>()
                .join(", ");
            if args.is_empty() {
                format!(" : {target}()")
            } else {
                format!(" : {target}({args})")
            }
        })
        .unwrap_or_default();

    write_indent(out, indent)?;
    if let Some(body) = &ctor.body {
        writeln!(out, "{}{} {{", prefix, initializer)?;
        for statement in &body.statements {
            write_statement(out, statement, indent + 1)?;
        }
        write_indent(out, indent)?;
        writeln!(out, "}}")?;
    } else {
        writeln!(out, "{}{};", prefix, initializer)?;
    }
    Ok(())
}

fn write_struct<W: Write>(out: &mut W, def: &StructDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let readonly = if def.is_readonly { "readonly " } else { "" };
    writeln!(
        out,
        "{}{}struct {} {{",
        format_visibility(def.visibility),
        readonly,
        def.name
    )?;
    for const_member in &def.consts {
        write_const_member(out, const_member, indent + 1)?;
    }
    for field in &def.fields {
        write_field(out, field, indent + 1)?;
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_union<W: Write>(out: &mut W, def: &UnionDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    writeln!(
        out,
        "{}union {} {{",
        format_visibility(def.visibility),
        def.name
    )?;
    for member in &def.members {
        match member {
            UnionMember::Field(field) => write_union_field(out, field, indent + 1)?,
            UnionMember::View(view) => write_union_view(out, view, indent + 1)?,
        }
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_union_field<W: Write>(out: &mut W, field: &UnionField, indent: usize) -> fmt::Result {
    write_doc(out, field.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let modifiers = format_union_modifiers(field.is_readonly);
    writeln!(
        out,
        "{}{}{} {};",
        format_visibility(field.visibility),
        modifiers,
        field.ty.name,
        field.name
    )?;
    Ok(())
}

fn write_union_view<W: Write>(out: &mut W, view: &UnionViewDecl, indent: usize) -> fmt::Result {
    write_doc(out, view.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let modifiers = format_union_modifiers(view.is_readonly);
    writeln!(
        out,
        "{}{}struct {} {{",
        format_visibility(view.visibility),
        modifiers,
        view.name
    )?;
    for field in &view.fields {
        write_field(out, field, indent + 1)?;
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_enum<W: Write>(out: &mut W, def: &EnumDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    writeln!(
        out,
        "{}enum {} {{",
        format_visibility(def.visibility),
        def.name
    )?;
    for variant in &def.variants {
        write_doc(out, variant.doc.as_ref(), indent + 1)?;
        write_indent(out, indent + 1)?;
        if variant.fields.is_empty() {
            writeln!(out, "{},", variant.name)?;
        } else {
            writeln!(out, "{} {{", variant.name)?;
            for field in &variant.fields {
                write_field(out, field, indent + 2)?;
            }
            write_indent(out, indent + 1)?;
            writeln!(out, "}},")?;
        }
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_class<W: Write>(out: &mut W, def: &ClassDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    if def.bases.is_empty() {
        writeln!(
            out,
            "{}class {} {{",
            format_visibility(def.visibility),
            def.name
        )?;
    } else {
        let bases = def
            .bases
            .iter()
            .map(|ty| ty.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "{}class {} : {} {{",
            format_visibility(def.visibility),
            def.name,
            bases
        )?;
    }
    for member in &def.members {
        match member {
            ClassMember::Field(field) => write_field(out, field, indent + 1)?,
            ClassMember::Method(func) => write_function(out, func, indent + 1)?,
            ClassMember::Property(property) => write_property(out, property, indent + 1)?,
            ClassMember::Constructor(ctor) => write_constructor(out, ctor, indent + 1)?,
            ClassMember::Const(const_member) => write_const_member(out, const_member, indent + 1)?,
        }
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_interface<W: Write>(out: &mut W, def: &InterfaceDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    if def.bases.is_empty() {
        writeln!(
            out,
            "{}interface {} {{",
            format_visibility(def.visibility),
            def.name
        )?;
    } else {
        let bases = def
            .bases
            .iter()
            .map(|ty| ty.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "{}interface {} : {} {{",
            format_visibility(def.visibility),
            def.name,
            bases
        )?;
    }
    for member in &def.members {
        match member {
            InterfaceMember::Method(func) => write_function(out, func, indent + 1)?,
            InterfaceMember::Property(property) => write_property(out, property, indent + 1)?,
            InterfaceMember::Const(const_member) => {
                write_const_member(out, const_member, indent + 1)?;
            }
            InterfaceMember::AssociatedType(_) => {}
        }
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_extension<W: Write>(out: &mut W, def: &ExtensionDecl, indent: usize) -> fmt::Result {
    write_doc(out, def.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    writeln!(
        out,
        "{}extension {} {{",
        format_visibility(def.visibility),
        def.target.name
    )?;
    for member in &def.members {
        match member {
            ExtensionMember::Method(method) => write_function(out, &method.function, indent + 1)?,
        }
    }
    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_field<W: Write>(out: &mut W, field: &FieldDecl, indent: usize) -> fmt::Result {
    write_doc(out, field.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let required = if field.is_required { "required " } else { "" };
    let name = field.display_name.as_deref().unwrap_or(&field.name);
    writeln!(
        out,
        "{}{}{}: {};",
        format_visibility(field.visibility),
        required,
        name,
        field.ty.name
    )?;
    Ok(())
}

fn write_property<W: Write>(out: &mut W, property: &PropertyDecl, indent: usize) -> fmt::Result {
    write_doc(out, property.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let required = if property.is_required {
        "required "
    } else {
        ""
    };
    writeln!(
        out,
        "{}{}{}: {} {{",
        format_visibility(property.visibility),
        required,
        property.name,
        property.ty.name
    )?;

    for accessor in &property.accessors {
        write_property_accessor(out, accessor, indent + 1)?;
    }

    write_indent(out, indent)?;
    writeln!(out, "}}")?;
    Ok(())
}

fn write_property_accessor<W: Write>(
    out: &mut W,
    accessor: &PropertyAccessor,
    indent: usize,
) -> fmt::Result {
    write_doc(out, accessor.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let vis = accessor.visibility.map(format_visibility).unwrap_or("");
    let kind = match accessor.kind {
        PropertyAccessorKind::Get => "get",
        PropertyAccessorKind::Set => "set",
        PropertyAccessorKind::Init => "init",
    };

    match &accessor.body {
        PropertyAccessorBody::Auto => {
            writeln!(out, "{}{};", vis, kind)?;
        }
        PropertyAccessorBody::Expression(expr) => {
            writeln!(out, "{}{} => {};", vis, kind, expr.text.trim())?;
        }
        PropertyAccessorBody::Block(_) => {
            writeln!(out, "{}{} {{ /* ... */ }}", vis, kind)?;
        }
    }
    Ok(())
}

fn write_testcase<W: Write>(out: &mut W, test: &TestCaseDecl, indent: usize) -> fmt::Result {
    write_doc(out, test.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let async_kw = if test.is_async { "async " } else { "" };
    writeln!(out, "{}testcase {} {{", async_kw, test.name)?;
    for statement in &test.body.statements {
        write_statement(out, statement, indent + 1)?;
    }
    write_indent(out, indent)?;
    writeln!(out, "}} // testcase")?;
    Ok(())
}

fn write_statement<W: Write>(out: &mut W, statement: &Statement, indent: usize) -> fmt::Result {
    write_indent(out, indent)?;
    let kind = &statement.kind;
    writeln!(out, "stmt {kind:?}")?;
    Ok(())
}

fn write_const_item<W: Write>(out: &mut W, item: &ConstItemDecl, indent: usize) -> fmt::Result {
    write_const_common(out, Some(item.visibility), &[], &item.declaration, indent)?;
    Ok(())
}

fn write_const_member<W: Write>(
    out: &mut W,
    member: &ConstMemberDecl,
    indent: usize,
) -> fmt::Result {
    write_const_common(
        out,
        Some(member.visibility),
        member.modifiers.as_slice(),
        &member.declaration,
        indent,
    )?;
    Ok(())
}

fn write_const_common<W: Write>(
    out: &mut W,
    visibility: Option<Visibility>,
    modifiers: &[String],
    declaration: &ConstDeclaration,
    indent: usize,
) -> fmt::Result {
    write_doc(out, declaration.doc.as_ref(), indent)?;
    write_indent(out, indent)?;
    let visibility_text = visibility.map(format_visibility).unwrap_or("");
    let modifiers_text = if modifiers.is_empty() {
        String::new()
    } else {
        format!("{} ", modifiers.join(" "))
    };
    let declarators = format_const_declarators(declaration);
    writeln!(
        out,
        "{}{}const {} {};",
        visibility_text, modifiers_text, declaration.ty.name, declarators
    )?;
    Ok(())
}

fn write_doc<W: Write>(out: &mut W, doc: Option<&DocComment>, indent: usize) -> fmt::Result {
    if let Some(doc) = doc {
        for line in &doc.lines {
            write_indent(out, indent)?;
            if line.is_empty() {
                writeln!(out, "///")?;
            } else {
                writeln!(out, "/// {line}")?;
            }
        }
    }
    Ok(())
}

fn write_import<W: Write>(out: &mut W, using: &UsingDirective, indent: usize) -> fmt::Result {
    write_indent(out, indent)?;
    let prefix = if using.is_global { "global " } else { "" };
    match &using.kind {
        UsingKind::Namespace { path } => {
            writeln!(out, "{prefix}import {path};")?;
        }
        UsingKind::Alias { alias, target } => {
            writeln!(out, "{prefix}import {alias} = {target};")?;
        }
        UsingKind::Static { target } => {
            writeln!(out, "{prefix}import static {target};")?;
        }
        UsingKind::CImport { header } => {
            writeln!(out, "@cimport \"{header}\";")?;
        }
    }
    Ok(())
}

fn write_indent<W: Write>(out: &mut W, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        out.write_str("    ")?;
    }
    Ok(())
}
