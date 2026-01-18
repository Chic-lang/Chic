use crate::diagnostics::DiagnosticCode;
use crate::frontend::ast::Item;
use crate::frontend::ast::items::{
    ClassDecl, ClassMember, ConstItemDecl, DelegateDecl, EnumDecl, ExtensionDecl, ExtensionMember,
    FunctionDecl, ImplDecl, ImplMember, InterfaceDecl, InterfaceMember, StaticItemDecl, StructDecl,
    TraitDecl, TraitMember, UnionDecl, UnionMember,
};
use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::manifest::{DocEnforcementScope, DocEnforcementSeverity, MissingDocsRule};

use super::FrontendModuleState;

pub(super) fn enforce_missing_docs(
    modules: &[FrontendModuleState],
    rule: &MissingDocsRule,
) -> Vec<Diagnostic> {
    if matches!(rule.severity, DocEnforcementSeverity::Ignore) {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for module in modules {
        if module.is_stdlib {
            continue;
        }
        let mut scope = Vec::new();
        if let Some(namespace) = &module.parse.module.namespace {
            extend_symbol_path(&mut scope, namespace);
        }
        collect_item_docs(
            &module.parse.module.items,
            &mut scope,
            rule.scope,
            rule.severity,
            &mut diagnostics,
        );
    }
    diagnostics
}

fn collect_item_docs(
    items: &[Item],
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            Item::Function(func) => {
                check_function_docs(func, scope, rule_scope, severity, diagnostics)
            }
            Item::Struct(decl) => check_struct_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Union(decl) => check_union_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Enum(decl) => check_enum_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Class(decl) => check_class_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Interface(decl) => {
                check_interface_docs(decl, scope, rule_scope, severity, diagnostics)
            }
            Item::Delegate(decl) => {
                check_delegate_docs(decl, scope, rule_scope, severity, diagnostics)
            }
            Item::Trait(decl) => check_trait_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Impl(decl) => check_impl_docs(decl, scope, rule_scope, severity, diagnostics),
            Item::Extension(decl) => {
                check_extension_docs(decl, scope, rule_scope, severity, diagnostics)
            }
            Item::Namespace(ns) => {
                let added = extend_symbol_path(scope, &ns.name);
                collect_item_docs(&ns.items, scope, rule_scope, severity, diagnostics);
                scope.truncate(scope.len().saturating_sub(added));
            }
            Item::Const(decl) => {
                check_const_item_docs(decl, scope, rule_scope, severity, diagnostics)
            }
            Item::Static(decl) => {
                check_static_item_docs(decl, scope, rule_scope, severity, diagnostics)
            }
            Item::TypeAlias(_) | Item::TestCase(_) | Item::Import(_) => {}
        }
    }
}

fn check_function_docs(
    func: &FunctionDecl,
    scope: &[String],
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(func.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &func.name);
    ensure_doc(&name, func.doc.as_ref(), None, severity, diagnostics);
}

fn check_struct_docs(
    decl: &StructDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(decl.name.clone());
    for field in &decl.fields {
        if visibility_in_scope(field.visibility, rule_scope) {
            let field_name = qualified_name(scope, &field.name);
            ensure_doc(&field_name, field.doc.as_ref(), None, severity, diagnostics);
        }
    }
    for property in &decl.properties {
        if visibility_in_scope(property.visibility, rule_scope) {
            let property_name = qualified_name(scope, &property.name);
            ensure_doc(
                &property_name,
                property.doc.as_ref(),
                property.span,
                severity,
                diagnostics,
            );
        }
    }
    for ctor in &decl.constructors {
        if visibility_in_scope(ctor.visibility, rule_scope) {
            let ctor_name = qualified_name(scope, &decl.name);
            ensure_doc(
                &ctor_name,
                ctor.doc.as_ref(),
                ctor.span,
                severity,
                diagnostics,
            );
        }
    }
    for konst in &decl.consts {
        if visibility_in_scope(konst.visibility, rule_scope) {
            let prefix = qualified_name(scope, "");
            check_const_decl_docs(
                &konst.declaration,
                konst.visibility,
                &prefix,
                rule_scope,
                severity,
                diagnostics,
            );
        }
    }
    for method in &decl.methods {
        if visibility_in_scope(method.visibility, rule_scope) {
            let method_name = qualified_name(scope, &method.name);
            ensure_doc(
                &method_name,
                method.doc.as_ref(),
                None,
                severity,
                diagnostics,
            );
        }
    }
    for nested in &decl.nested_types {
        collect_item_docs(&[nested.clone()], scope, rule_scope, severity, diagnostics);
    }
    scope.pop();
}

fn check_union_docs(
    decl: &UnionDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(decl.name.clone());
    for member in &decl.members {
        match member {
            UnionMember::Field(field) if visibility_in_scope(field.visibility, rule_scope) => {
                let field_name = qualified_name(scope, &field.name);
                ensure_doc(&field_name, field.doc.as_ref(), None, severity, diagnostics);
            }
            UnionMember::View(view) if visibility_in_scope(view.visibility, rule_scope) => {
                let view_name = qualified_name(scope, &view.name);
                ensure_doc(&view_name, view.doc.as_ref(), None, severity, diagnostics);
                for field in &view.fields {
                    if visibility_in_scope(field.visibility, rule_scope) {
                        let field_name = qualified_name(scope, &field.name);
                        ensure_doc(&field_name, field.doc.as_ref(), None, severity, diagnostics);
                    }
                }
            }
            _ => {}
        }
    }
    scope.pop();
}

fn check_enum_docs(
    decl: &EnumDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(decl.name.clone());
    for variant in &decl.variants {
        let variant_name = qualified_name(scope, &variant.name);
        ensure_doc(
            &variant_name,
            variant.doc.as_ref(),
            None,
            severity,
            diagnostics,
        );
        for field in &variant.fields {
            if visibility_in_scope(field.visibility, rule_scope) {
                let field_name = qualified_name(scope, &field.name);
                ensure_doc(&field_name, field.doc.as_ref(), None, severity, diagnostics);
            }
        }
    }
    scope.pop();
}

fn check_class_docs(
    decl: &ClassDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(decl.name.clone());
    for member in &decl.members {
        match member {
            ClassMember::Field(field) if visibility_in_scope(field.visibility, rule_scope) => {
                let field_name = qualified_name(scope, &field.name);
                ensure_doc(&field_name, field.doc.as_ref(), None, severity, diagnostics);
            }
            ClassMember::Method(method) if visibility_in_scope(method.visibility, rule_scope) => {
                let method_name = qualified_name(scope, &method.name);
                ensure_doc(
                    &method_name,
                    method.doc.as_ref(),
                    None,
                    severity,
                    diagnostics,
                );
            }
            ClassMember::Property(property)
                if visibility_in_scope(property.visibility, rule_scope) =>
            {
                let prop_name = qualified_name(scope, &property.name);
                ensure_doc(
                    &prop_name,
                    property.doc.as_ref(),
                    property.span,
                    severity,
                    diagnostics,
                );
            }
            ClassMember::Constructor(ctor) if visibility_in_scope(ctor.visibility, rule_scope) => {
                let ctor_name = qualified_name(scope, &decl.name);
                ensure_doc(
                    &ctor_name,
                    ctor.doc.as_ref(),
                    ctor.span,
                    severity,
                    diagnostics,
                );
            }
            ClassMember::Const(konst) if visibility_in_scope(konst.visibility, rule_scope) => {
                let prefix = qualified_name(scope, "");
                check_const_decl_docs(
                    &konst.declaration,
                    konst.visibility,
                    &prefix,
                    rule_scope,
                    severity,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
    for nested in &decl.nested_types {
        collect_item_docs(&[nested.clone()], scope, rule_scope, severity, diagnostics);
    }
    scope.pop();
}

fn check_interface_docs(
    decl: &InterfaceDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(decl.name.clone());
    for member in &decl.members {
        match member {
            InterfaceMember::Method(method)
                if visibility_in_scope(method.visibility, rule_scope) =>
            {
                let method_name = qualified_name(scope, &method.name);
                ensure_doc(
                    &method_name,
                    method.doc.as_ref(),
                    None,
                    severity,
                    diagnostics,
                );
            }
            InterfaceMember::Property(property)
                if visibility_in_scope(property.visibility, rule_scope) =>
            {
                let prop_name = qualified_name(scope, &property.name);
                ensure_doc(
                    &prop_name,
                    property.doc.as_ref(),
                    property.span,
                    severity,
                    diagnostics,
                );
            }
            InterfaceMember::Const(konst) if visibility_in_scope(konst.visibility, rule_scope) => {
                let prefix = qualified_name(scope, "");
                check_const_decl_docs(
                    &konst.declaration,
                    konst.visibility,
                    &prefix,
                    rule_scope,
                    severity,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
    scope.pop();
}

fn check_delegate_docs(
    decl: &DelegateDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), decl.span, severity, diagnostics);
}

fn check_trait_docs(
    decl: &TraitDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let name = qualified_name(scope, &decl.name);
    ensure_doc(&name, decl.doc.as_ref(), decl.span, severity, diagnostics);
    scope.push(decl.name.clone());
    for member in &decl.members {
        match member {
            TraitMember::Method(method) if visibility_in_scope(method.visibility, rule_scope) => {
                let method_name = qualified_name(scope, &method.name);
                ensure_doc(
                    &method_name,
                    method.doc.as_ref(),
                    None,
                    severity,
                    diagnostics,
                );
            }
            TraitMember::AssociatedType(assoc) => {
                let assoc_name = qualified_name(scope, &assoc.name);
                ensure_doc(
                    &assoc_name,
                    assoc.doc.as_ref(),
                    assoc.span,
                    severity,
                    diagnostics,
                );
            }
            TraitMember::Const(konst) if visibility_in_scope(konst.visibility, rule_scope) => {
                let prefix = qualified_name(scope, "");
                check_const_decl_docs(
                    &konst.declaration,
                    konst.visibility,
                    &prefix,
                    rule_scope,
                    severity,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
    scope.pop();
}

fn check_impl_docs(
    decl: &ImplDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let target_name = decl.target.name.clone();
    let name = qualified_name(scope, &target_name);
    ensure_doc(&name, decl.doc.as_ref(), decl.span, severity, diagnostics);
    scope.push(target_name.clone());
    for member in &decl.members {
        match member {
            ImplMember::Method(method) if visibility_in_scope(method.visibility, rule_scope) => {
                let method_name = qualified_name(scope, &method.name);
                ensure_doc(
                    &method_name,
                    method.doc.as_ref(),
                    None,
                    severity,
                    diagnostics,
                );
            }
            ImplMember::AssociatedType(assoc) => {
                let assoc_name = qualified_name(scope, &assoc.name);
                ensure_doc(
                    &assoc_name,
                    assoc.doc.as_ref(),
                    assoc.span,
                    severity,
                    diagnostics,
                );
            }
            ImplMember::Const(konst) if visibility_in_scope(konst.visibility, rule_scope) => {
                let prefix = qualified_name(scope, "");
                check_const_decl_docs(
                    &konst.declaration,
                    konst.visibility,
                    &prefix,
                    rule_scope,
                    severity,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
    scope.pop();
}

fn check_extension_docs(
    decl: &ExtensionDecl,
    scope: &mut Vec<String>,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let target_name = decl.target.name.clone();
    let name = qualified_name(scope, &target_name);
    ensure_doc(&name, decl.doc.as_ref(), None, severity, diagnostics);
    scope.push(target_name.clone());
    for member in &decl.members {
        match member {
            ExtensionMember::Method(method)
                if visibility_in_scope(method.function.visibility, rule_scope) =>
            {
                let method_name = qualified_name(scope, &method.function.name);
                ensure_doc(
                    &method_name,
                    method.function.doc.as_ref(),
                    None,
                    severity,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
    scope.pop();
}

fn check_const_item_docs(
    decl: &ConstItemDecl,
    scope: &[String],
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let prefix = qualified_name(scope, "");
    check_const_decl_docs(
        &decl.declaration,
        decl.visibility,
        &prefix,
        rule_scope,
        severity,
        diagnostics,
    );
}

fn check_static_item_docs(
    decl: &StaticItemDecl,
    scope: &[String],
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !visibility_in_scope(decl.visibility, rule_scope) {
        return;
    }
    let decl_doc = decl.declaration.doc.as_ref();
    for declarator in &decl.declaration.declarators {
        let name = qualified_name(scope, &declarator.name);
        let span = decl.declaration.span.or(declarator.span);
        ensure_doc(&name, decl_doc, span, severity, diagnostics);
    }
}

fn check_const_decl_docs(
    decl: &crate::frontend::ast::items::ConstDeclaration,
    visibility: crate::frontend::ast::items::Visibility,
    type_prefix: &str,
    rule_scope: DocEnforcementScope,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let doc = decl.doc.as_ref();
    for declarator in &decl.declarators {
        let name = if type_prefix.is_empty() {
            declarator.name.clone()
        } else {
            format!("{}::{}", type_prefix, declarator.name)
        };
        if visibility_in_scope(visibility, rule_scope) {
            ensure_doc(
                &name,
                doc,
                decl.span.or(declarator.span),
                severity,
                diagnostics,
            );
        }
    }
}

fn ensure_doc(
    name: &str,
    doc: Option<&crate::frontend::ast::items::DocComment>,
    span: Option<Span>,
    severity: DocEnforcementSeverity,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let present = doc.map(|doc| !doc.is_empty()).unwrap_or(false);
    if present {
        return;
    }
    let mut diagnostic = match severity {
        DocEnforcementSeverity::Error => {
            Diagnostic::error(format!("'{name}' is missing XML documentation"), span)
        }
        DocEnforcementSeverity::Warning => {
            Diagnostic::warning(format!("'{name}' is missing XML documentation"), span)
        }
        DocEnforcementSeverity::Ignore => return,
    };
    diagnostic.code = Some(DiagnosticCode::new("DOC0001", Some("docs".into())));
    diagnostics.push(diagnostic);
}

fn extend_symbol_path(path: &mut Vec<String>, name: &str) -> usize {
    let parts: Vec<_> = name
        .split(|ch| ch == '.' || ch == ':')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();
    let added = parts.len();
    path.extend(parts);
    added
}

fn qualified_name(scope: &[String], name: &str) -> String {
    if scope.is_empty() && name.is_empty() {
        return String::new();
    }
    if scope.is_empty() {
        return name.to_string();
    }
    if name.is_empty() {
        return scope.join("::");
    }
    format!("{}::{}", scope.join("::"), name)
}

fn visibility_in_scope(
    vis: crate::frontend::ast::items::Visibility,
    scope: DocEnforcementScope,
) -> bool {
    match scope {
        DocEnforcementScope::Public => matches!(
            vis,
            crate::frontend::ast::items::Visibility::Public
                | crate::frontend::ast::items::Visibility::Protected
                | crate::frontend::ast::items::Visibility::ProtectedInternal
                | crate::frontend::ast::items::Visibility::PrivateProtected
        ),
        DocEnforcementScope::PublicAndInternal => matches!(
            vis,
            crate::frontend::ast::items::Visibility::Public
                | crate::frontend::ast::items::Visibility::Protected
                | crate::frontend::ast::items::Visibility::ProtectedInternal
                | crate::frontend::ast::items::Visibility::PrivateProtected
                | crate::frontend::ast::items::Visibility::Internal
        ),
        DocEnforcementScope::All => true,
    }
}
