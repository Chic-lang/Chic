//! Apply `@cfg` conditional compilation attributes to AST nodes.

use crate::frontend::ast::expressions::{
    Block, CatchClause, ForStatement, ForeachStatement, Statement, StatementKind, SwitchSection,
};
use crate::frontend::ast::items::{
    Attribute, ClassDecl, ClassMember, EnumDecl, ExtensionDecl, ExtensionMember, FunctionDecl,
    ImplDecl, ImplMember, InterfaceDecl, InterfaceMember, Item, Module, NamespaceDecl,
    PropertyAccessor, PropertyDecl, StructDecl, TestCaseDecl, TraitDecl, TraitMember, UnionDecl,
    UnionMember,
};
use crate::frontend::conditional::{ConditionalDefines, evaluate_condition_with_diagnostics};
use crate::frontend::diagnostics::Diagnostic;

/// Evaluate and apply all `@cfg` attributes in the module, pruning inactive items and statements.
pub(crate) fn apply_cfg(module: &mut Module, defines: &ConditionalDefines) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !cfg_allows(&mut module.namespace_attributes, defines, &mut diagnostics) {
        module.items.clear();
        module.rebuild_overloads();
        return diagnostics;
    }

    prune_items(&mut module.items, defines, &mut diagnostics);
    module.rebuild_overloads();
    diagnostics
}

fn prune_items(
    items: &mut Vec<Item>,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut index = 0;
    while index < items.len() {
        let keep = match &mut items[index] {
            Item::Namespace(ns) => prune_namespace(ns, defines, diagnostics),
            Item::Struct(strct) => prune_struct(strct, defines, diagnostics),
            Item::Union(union_def) => prune_union(union_def, defines, diagnostics),
            Item::Enum(enm) => prune_enum(enm, defines, diagnostics),
            Item::Class(class) => prune_class(class, defines, diagnostics),
            Item::Interface(iface) => prune_interface(iface, defines, diagnostics),
            Item::Extension(ext) => prune_extension(ext, defines, diagnostics),
            Item::Function(func) => prune_function(func, defines, diagnostics),
            Item::TestCase(test) => prune_testcase(test, defines, diagnostics),
            Item::Trait(trait_decl) => prune_trait(trait_decl, defines, diagnostics),
            Item::Impl(impl_decl) => prune_impl(impl_decl, defines, diagnostics),
            Item::Delegate(_) => true,
            Item::Import(_) | Item::Const(_) | Item::Static(_) | Item::TypeAlias(_) => true,
        };
        if keep {
            index += 1;
        } else {
            items.remove(index);
        }
    }
}

fn prune_namespace(
    namespace: &mut NamespaceDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut namespace.attributes, defines, diagnostics) {
        return false;
    }
    prune_items(&mut namespace.items, defines, diagnostics);
    true
}

fn prune_struct(
    strct: &mut StructDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut strct.attributes, defines, diagnostics) {
        return false;
    }

    strct
        .fields
        .retain_mut(|field| cfg_allows(&mut field.attributes, defines, diagnostics));
    strct
        .properties
        .retain_mut(|prop| prune_property(prop, defines, diagnostics));
    strct
        .constructors
        .retain_mut(|ctor| prune_constructor(ctor, defines, diagnostics));
    strct
        .methods
        .retain_mut(|func| prune_function(func, defines, diagnostics));
    prune_items(&mut strct.nested_types, defines, diagnostics);
    true
}

fn prune_union(
    union_def: &mut UnionDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut union_def.attributes, defines, diagnostics) {
        return false;
    }
    union_def.members.retain_mut(|member| match member {
        UnionMember::Field(field) => cfg_allows(&mut field.attributes, defines, diagnostics),
        UnionMember::View(view) => cfg_allows(&mut view.attributes, defines, diagnostics),
    });
    true
}

fn prune_enum(
    enm: &mut EnumDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut enm.attributes, defines, diagnostics) {
        return false;
    }
    true
}

fn prune_class(
    class: &mut ClassDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut class.attributes, defines, diagnostics) {
        return false;
    }

    class.members.retain_mut(|member| match member {
        ClassMember::Constructor(ctor) => prune_constructor(ctor, defines, diagnostics),
        ClassMember::Method(func) => prune_function(func, defines, diagnostics),
        ClassMember::Property(prop) => prune_property(prop, defines, diagnostics),
        ClassMember::Field(field) => cfg_allows(&mut field.attributes, defines, diagnostics),
        ClassMember::Const(_) => true,
    });
    prune_items(&mut class.nested_types, defines, diagnostics);
    true
}

fn prune_interface(
    iface: &mut InterfaceDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut iface.attributes, defines, diagnostics) {
        return false;
    }
    iface.members.retain_mut(|member| match member {
        InterfaceMember::Method(func) => prune_function(func, defines, diagnostics),
        InterfaceMember::Property(prop) => prune_property(prop, defines, diagnostics),
        InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => true,
    });
    true
}

fn prune_extension(
    ext: &mut ExtensionDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut ext.attributes, defines, diagnostics) {
        return false;
    }
    ext.members.retain_mut(|member| match member {
        ExtensionMember::Method(method) => {
            prune_function(&mut method.function, defines, diagnostics)
        }
    });
    true
}

fn prune_trait(
    trait_decl: &mut TraitDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut trait_decl.attributes, defines, diagnostics) {
        return false;
    }
    trait_decl.members.retain_mut(|member| match member {
        TraitMember::Method(func) => prune_function(func, defines, diagnostics),
        TraitMember::AssociatedType(_) => true,
        TraitMember::Const(_) => true,
    });
    true
}

fn prune_impl(
    impl_decl: &mut ImplDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut impl_decl.attributes, defines, diagnostics) {
        return false;
    }
    impl_decl.members.retain_mut(|member| match member {
        ImplMember::Method(func) => prune_function(func, defines, diagnostics),
        ImplMember::AssociatedType(_) => true,
        ImplMember::Const(_) => true,
    });
    true
}

fn prune_function(
    func: &mut FunctionDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut func.attributes, defines, diagnostics) {
        return false;
    }
    if let Some(body) = &mut func.body {
        prune_block(body, defines, diagnostics);
    }
    true
}

fn prune_constructor(
    ctor: &mut crate::frontend::ast::ConstructorDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut ctor.attributes, defines, diagnostics) {
        return false;
    }
    if let Some(body) = &mut ctor.body {
        prune_block(body, defines, diagnostics);
    }
    true
}

fn prune_property(
    prop: &mut PropertyDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut prop.attributes, defines, diagnostics) {
        return false;
    }
    prop.parameters
        .retain_mut(|param| cfg_allows(&mut param.attributes, defines, diagnostics));
    prop.accessors
        .retain_mut(|accessor| prune_accessor(accessor, defines, diagnostics));
    true
}

fn prune_accessor(
    accessor: &mut PropertyAccessor,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows_opt(&mut accessor.attributes, defines, diagnostics) {
        return false;
    }
    match &mut accessor.body {
        crate::frontend::ast::items::PropertyAccessorBody::Block(block) => {
            prune_block(block, defines, diagnostics);
        }
        crate::frontend::ast::items::PropertyAccessorBody::Expression(_) => {}
        crate::frontend::ast::items::PropertyAccessorBody::Auto => {}
    }
    true
}

fn prune_testcase(
    testcase: &mut TestCaseDecl,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows(&mut testcase.attributes, defines, diagnostics) {
        return false;
    }
    prune_block(&mut testcase.body, defines, diagnostics);
    true
}

fn prune_block(block: &mut Block, defines: &ConditionalDefines, diagnostics: &mut Vec<Diagnostic>) {
    let mut index = 0;
    while index < block.statements.len() {
        if prune_statement(&mut block.statements[index], defines, diagnostics) {
            index += 1;
        } else {
            block.statements.remove(index);
        }
    }
}

fn prune_statement(
    statement: &mut Statement,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if !cfg_allows_opt(&mut statement.attributes, defines, diagnostics) {
        return false;
    }

    match &mut statement.kind {
        StatementKind::Block(block) => prune_block(block, defines, diagnostics),
        StatementKind::If(if_stmt) => {
            let then_keep = prune_child_statement(&mut if_stmt.then_branch, defines, diagnostics);
            let else_keep = if let Some(else_stmt) = &mut if_stmt.else_branch {
                prune_child_statement(else_stmt, defines, diagnostics)
            } else {
                false
            };
            if !then_keep && else_keep {
                if let Some(else_branch) = if_stmt.else_branch.take() {
                    *statement = *else_branch;
                    return true;
                }
            } else if !then_keep && !else_keep {
                return false;
            } else if let Some(_else_branch) = &mut if_stmt.else_branch {
                if !else_keep {
                    if_stmt.else_branch = None;
                }
            }
        }
        StatementKind::While { body, .. }
        | StatementKind::DoWhile { body, .. }
        | StatementKind::Lock { body, .. }
        | StatementKind::Unsafe { body, .. } => {
            if !prune_child_statement(body, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::For(ForStatement { body, .. }) => {
            if !prune_child_statement(body, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::Foreach(ForeachStatement { body, .. }) => {
            if !prune_child_statement(body, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::Switch(switch_stmt) => {
            switch_stmt.sections.retain_mut(|section| {
                prune_switch_section(section, defines, diagnostics);
                !section.statements.is_empty()
            });
            if switch_stmt.sections.is_empty() {
                return false;
            }
        }
        StatementKind::Try(try_stmt) => {
            prune_block(&mut try_stmt.body, defines, diagnostics);
            try_stmt
                .catches
                .retain_mut(|catch| prune_catch_clause(catch, defines, diagnostics));
            if let Some(finally) = &mut try_stmt.finally {
                prune_block(finally, defines, diagnostics);
            }
            if try_stmt.catches.is_empty()
                && try_stmt.finally.is_none()
                && try_stmt.body.statements.is_empty()
            {
                return false;
            }
        }
        StatementKind::Using(using_stmt) => {
            if let Some(body) = &mut using_stmt.body {
                if !prune_child_statement(body, defines, diagnostics) {
                    using_stmt.body = None;
                }
            }
        }
        StatementKind::Region { body, .. }
        | StatementKind::Checked { body }
        | StatementKind::Atomic { body, .. }
        | StatementKind::Unchecked { body } => {
            prune_block(body, defines, diagnostics);
        }
        StatementKind::Labeled {
            statement: inner, ..
        } => {
            if !prune_child_statement(inner, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::LocalFunction(func) => {
            if !prune_function(func, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::Fixed(fixed) => {
            if !prune_child_statement(&mut fixed.body, defines, diagnostics) {
                return false;
            }
        }
        StatementKind::Goto(_) => {}
        StatementKind::VariableDeclaration(_) => {}
        StatementKind::ConstDeclaration(_) => {}
        StatementKind::Expression(_)
        | StatementKind::Return { .. }
        | StatementKind::Break
        | StatementKind::Continue
        | StatementKind::Throw { .. }
        | StatementKind::YieldReturn { .. }
        | StatementKind::YieldBreak
        | StatementKind::Empty => {}
    }
    true
}

fn prune_child_statement(
    statement: &mut Statement,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    prune_statement(statement, defines, diagnostics)
}

fn prune_switch_section(
    section: &mut SwitchSection,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) {
    section
        .statements
        .retain_mut(|stmt| prune_statement(stmt, defines, diagnostics));
}

fn prune_catch_clause(
    clause: &mut CatchClause,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    prune_block(&mut clause.body, defines, diagnostics);
    !clause.body.statements.is_empty()
}

fn cfg_allows_opt(
    attrs: &mut Option<Vec<Attribute>>,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(attr_list) = attrs else {
        return true;
    };
    let keep = cfg_allows(attr_list, defines, diagnostics);
    if attr_list.is_empty() {
        *attrs = None;
    }
    keep
}

fn cfg_allows(
    attrs: &mut Vec<Attribute>,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut active = true;
    attrs.retain(|attr| {
        if attr.name.eq_ignore_ascii_case("cfg") {
            match cfg_allows_single(attr, defines, diagnostics) {
                true => {}
                false => active = false,
            }
            false
        } else {
            true
        }
    });
    active
}

fn cfg_allows_single(
    attr: &Attribute,
    defines: &ConditionalDefines,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some((expr, offset)) = extract_cfg_expression(attr) else {
        diagnostics.push(Diagnostic::error(
            "`@cfg` attribute requires a condition in parentheses",
            attr.span,
        ));
        return false;
    };

    match evaluate_condition_with_diagnostics(&expr, offset, defines) {
        Ok(value) => value,
        Err(diag) => {
            diagnostics.push(diag);
            false
        }
    }
}

fn extract_cfg_expression(attr: &Attribute) -> Option<(String, usize)> {
    let raw = attr.raw.as_ref()?;
    let span = attr.span?;
    let open = raw.find('(')?;
    let mut depth = 0usize;
    let mut in_string: Option<char> = None;
    let mut escape = false;
    let mut close_idx = None;
    for (idx, ch) in raw.char_indices().skip(open + 1) {
        if let Some(delim) = in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == delim {
                in_string = None;
            }
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => {
                close_idx = Some(idx);
                break;
            }
            ')' => depth -= 1,
            '"' | '\'' => in_string = Some(ch),
            _ => {}
        }
    }
    let close = close_idx?;
    let expr_start = open + 1;
    let expr = raw[expr_start..close].trim();
    if expr.is_empty() {
        return None;
    }
    let global_offset = span.start + expr_start;
    Some((expr.to_string(), global_offset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::Item;
    use crate::frontend::conditional::ConditionalDefines;
    use crate::frontend::parser::parse_module_with_defines;

    fn struct_names(module: &Module) -> Vec<String> {
        module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Struct(def) => Some(def.name.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn cfg_prunes_inactive_items() {
        let mut defines = ConditionalDefines::default();
        defines.set_string("target_arch", "x86_64");
        let source = r#"
            @cfg(target_arch = "x86_64")
            public struct Active {}
            @cfg(target_arch = "arm64")
            public struct Inactive {}
        "#;
        let parsed = parse_module_with_defines(source, &defines).expect("parse module");
        assert!(
            parsed.diagnostics.is_empty(),
            "expected clean parse, found {parsed:?}"
        );
        let names = struct_names(&parsed.module);
        assert!(
            names.contains(&"Active".to_string()),
            "expected Active struct to remain"
        );
        assert!(
            !names.contains(&"Inactive".to_string()),
            "cfg should prune inactive struct"
        );
    }

    #[test]
    fn cfg_prunes_statements() {
        let mut defines = ConditionalDefines::default();
        defines.set_bool("DEBUG", true);
        defines.set_bool("RELEASE", false);
        let source = r#"
            public void run() {
                @cfg(DEBUG)
                var a = 1;
                @cfg(RELEASE)
                var b = 2;
            }
        "#;
        let parsed = parse_module_with_defines(source, &defines).expect("parse module");
        let func = parsed
            .module
            .items
            .iter()
            .find_map(|item| match item {
                Item::Function(func) => Some(func),
                _ => None,
            })
            .expect("function present");
        let body = func.body.as_ref().expect("function body");
        let kinds = body
            .statements
            .iter()
            .map(|stmt| std::mem::discriminant(&stmt.kind))
            .collect::<Vec<_>>();
        assert_eq!(
            kinds.len(),
            1,
            "only the DEBUG-guarded statement should remain"
        );
    }

    #[test]
    fn cfg_honours_boolean_combinations() {
        let mut defines = ConditionalDefines::default();
        defines.set_bool("DEBUG", true);
        defines.set_string("target_os", "linux");
        let source = r#"
            @cfg(DEBUG && target_os == "linux")
            public struct LinuxDebug {}
            @cfg(DEBUG && target_os == "macos")
            public struct MacDebug {}
            @cfg(!DEBUG || target_os == "linux")
            public struct Always {}
        "#;
        let parsed = parse_module_with_defines(source, &defines).expect("parse module");
        let names = struct_names(&parsed.module);
        assert!(names.contains(&"LinuxDebug".to_string()));
        assert!(names.contains(&"Always".to_string()));
        assert!(
            !names.contains(&"MacDebug".to_string()),
            "cfg should evaluate boolean expressions"
        );
    }
}
