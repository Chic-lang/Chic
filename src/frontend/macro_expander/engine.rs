use crate::frontend::ast::{
    Block, ClassDecl, ClassMember, EnumDecl, ExtensionDecl, ExtensionMember, FunctionDecl,
    InterfaceDecl, InterfaceMember, Item, Module, NamespaceDecl, Statement, StatementKind,
    StructDecl, UnionDecl, UnionMember,
};
use crate::frontend::attributes::stage_builtin_attributes;
use crate::frontend::diagnostics::Diagnostic;
use std::collections::HashSet;

use super::cache::{CachedExpansion, ExpansionCache};
use super::collector::{HygieneTracker, collect_invocations};
use super::diagnostics::{runaway_macros, unknown_macro, unsupported_macro};
use super::handlers::{build_equatable_extension, build_hashable_extension};
use super::model::{InvocationCacheKey, MacroInvocation, MacroInvocationKind};
use super::origin::stamp_items_with_origin;
use super::registry::{
    AttributeInput, AttributeOutput, AttributeTarget, DeriveInput, DeriveOutput, DeriveTarget,
    MacroRegistry,
};

const MAX_MACRO_PASSES: usize = 32;

pub struct MacroExpansionResult {
    pub diagnostics: Vec<Diagnostic>,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub passes: usize,
}

pub fn expand_module(module: &mut Module, registry: &MacroRegistry) -> MacroExpansionResult {
    let mut expander = MacroExpander::new(registry);
    let mut passes = 0usize;
    loop {
        passes += 1;
        expander.start_pass(passes);
        expander.progress = false;
        expander.expand_items(&mut module.items, Vec::new());
        if !expander.progress {
            break;
        }
        if passes >= MAX_MACRO_PASSES {
            expander.diagnostics.push(runaway_macros(MAX_MACRO_PASSES));
            break;
        }
    }

    let mut diagnostics = expander.diagnostics;
    let metrics = expander.cache.metrics();

    let mut staging = stage_builtin_attributes(module);
    diagnostics.append(&mut staging);

    MacroExpansionResult {
        diagnostics,
        cache_hits: metrics.hits,
        cache_misses: metrics.misses,
        passes,
    }
}

struct MacroExpander<'a> {
    registry: &'a MacroRegistry,
    cache: ExpansionCache,
    diagnostics: Vec<Diagnostic>,
    progress: bool,
    hygiene: HygieneTracker,
    processed_records: HashSet<String>,
}

impl<'a> MacroExpander<'a> {
    fn new(registry: &'a MacroRegistry) -> Self {
        Self {
            registry,
            cache: ExpansionCache::new(),
            diagnostics: Vec::new(),
            progress: false,
            hygiene: HygieneTracker::default(),
            processed_records: HashSet::new(),
        }
    }

    fn start_pass(&mut self, pass: usize) {
        self.hygiene.start_pass(pass);
    }

    fn expand_items(&mut self, items: &mut Vec<Item>, scope: Vec<String>) {
        let mut index = 0;
        while index < items.len() {
            let generated = match &mut items[index] {
                Item::Namespace(namespace) => self.process_namespace(namespace, &scope),
                Item::Struct(strct) => self.process_struct(strct, &scope),
                Item::Union(union_def) => self.process_union(union_def, &scope),
                Item::Enum(enm) => self.process_enum(enm, &scope),
                Item::Class(class) => self.process_class(class, &scope),
                Item::Interface(iface) => self.process_interface(iface, &scope),
                Item::Extension(ext) => self.process_extension(ext, &scope),
                Item::Function(func) => self.process_function(func, None, &scope),
                Item::TestCase(testcase) => {
                    self.expand_block(&mut testcase.body, &scope);
                    Vec::new()
                }
                Item::Trait(_)
                | Item::Delegate(_)
                | Item::Impl(_)
                | Item::Import(_)
                | Item::Const(_)
                | Item::Static(_)
                | Item::TypeAlias(_) => Vec::new(),
            };

            if !generated.is_empty() {
                for (offset, item) in generated.into_iter().enumerate() {
                    items.insert(index + 1 + offset, item);
                }
                self.progress = true;
            }

            if let Item::Namespace(namespace) = &mut items[index] {
                let mut nested_scope = scope.clone();
                nested_scope.push(namespace.name.clone());
                self.expand_items(&mut namespace.items, nested_scope);
            }

            index += 1;
        }
    }

    fn process_namespace(&mut self, namespace: &mut NamespaceDecl, scope: &[String]) -> Vec<Item> {
        let (invocations, errors) =
            collect_invocations(&mut namespace.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        let context = format!("namespace `{}`", qualified_name(scope, &namespace.name));
        for invocation in &invocations {
            self.diagnostics
                .push(unsupported_macro(invocation, &context));
        }
        Vec::new()
    }

    fn process_struct(&mut self, strct: &mut StructDecl, scope: &[String]) -> Vec<Item> {
        let struct_name = qualified_name(scope, &strct.name);
        let (invocations, errors) = collect_invocations(&mut strct.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        let mut generated = Vec::new();
        let mut derive_calls = Vec::new();

        for invocation in invocations {
            match invocation.kind {
                MacroInvocationKind::Derive => derive_calls.push(invocation),
                MacroInvocationKind::Attribute => {
                    self.diagnostics.push(unsupported_macro(
                        &invocation,
                        &format!("struct `{struct_name}`"),
                    ));
                }
            }
        }

        for invocation in &derive_calls {
            let Some(handler) = self.registry.get_derive(&invocation.name) else {
                self.diagnostics.push(unknown_macro(
                    invocation,
                    &format!("struct `{struct_name}`"),
                ));
                continue;
            };
            let items = self.execute_derive(invocation, &struct_name, || {
                handler(DeriveInput {
                    invocation,
                    target: DeriveTarget::Struct(strct),
                })
            });
            generated.extend(items);
        }

        if strct.is_record {
            let qualified = struct_name.clone();
            let already_processed = !self.processed_records.insert(qualified.clone());
            if strct
                .generics
                .as_ref()
                .is_some_and(|params| !params.params.is_empty())
            {
                if !already_processed {
                    self.diagnostics.push(Diagnostic::error(
                        "`record` auto-generated equality/hash does not yet support generic parameters",
                        None,
                    ));
                }
            } else if !already_processed {
                let requested: Vec<String> = derive_calls
                    .iter()
                    .map(|call| call.name.to_ascii_lowercase())
                    .collect();
                let fields = strct
                    .fields
                    .iter()
                    .map(|field| field.name.clone())
                    .collect::<Vec<_>>();
                if !requested.iter().any(|name| name == "equatable") {
                    generated.push(Item::Extension(build_equatable_extension(
                        &strct.name,
                        strct.visibility,
                        &fields,
                    )));
                }
                if !requested.iter().any(|name| name == "hashable") {
                    generated.push(Item::Extension(build_hashable_extension(
                        &strct.name,
                        strct.visibility,
                        &fields,
                    )));
                }
            }
        }

        self.expand_struct_members(strct, scope);
        generated
    }

    fn process_union(
        &mut self,
        union_def: &mut crate::frontend::ast::UnionDecl,
        scope: &[String],
    ) -> Vec<Item> {
        let union_name = qualified_name(scope, &union_def.name);
        let (invocations, errors) =
            collect_invocations(&mut union_def.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        for invocation in &invocations {
            self.diagnostics.push(unsupported_macro(
                invocation,
                &format!("union `{union_name}`"),
            ));
        }
        self.expand_union_members(union_def, scope);
        Vec::new()
    }

    fn process_enum(&mut self, enm: &mut EnumDecl, scope: &[String]) -> Vec<Item> {
        let enum_name = qualified_name(scope, &enm.name);
        let (invocations, errors) = collect_invocations(&mut enm.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        let mut generated = Vec::new();
        let mut derive_calls = Vec::new();

        for invocation in invocations {
            match invocation.kind {
                MacroInvocationKind::Derive => derive_calls.push(invocation),
                MacroInvocationKind::Attribute => {
                    self.diagnostics.push(unsupported_macro(
                        &invocation,
                        &format!("enum `{enum_name}`"),
                    ));
                }
            }
        }

        for invocation in &derive_calls {
            let Some(handler) = self.registry.get_derive(&invocation.name) else {
                self.diagnostics
                    .push(unknown_macro(invocation, &format!("enum `{enum_name}`")));
                continue;
            };
            let items = self.execute_derive(invocation, &enum_name, || {
                handler(DeriveInput {
                    invocation,
                    target: DeriveTarget::Enum(enm),
                })
            });
            generated.extend(items);
        }

        self.expand_enum_members(enm, scope);
        generated
    }

    fn process_class(&mut self, class: &mut ClassDecl, scope: &[String]) -> Vec<Item> {
        let class_name = qualified_name(scope, &class.name);
        let (invocations, errors) = collect_invocations(&mut class.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        let mut generated = Vec::new();
        let mut derive_calls = Vec::new();

        for invocation in invocations {
            match invocation.kind {
                MacroInvocationKind::Derive => derive_calls.push(invocation),
                MacroInvocationKind::Attribute => {
                    self.diagnostics.push(unsupported_macro(
                        &invocation,
                        &format!("class `{class_name}`"),
                    ));
                }
            }
        }

        for invocation in &derive_calls {
            let Some(handler) = self.registry.get_derive(&invocation.name) else {
                self.diagnostics
                    .push(unknown_macro(invocation, &format!("class `{class_name}`")));
                continue;
            };
            let items = self.execute_derive(invocation, &class_name, || {
                handler(DeriveInput {
                    invocation,
                    target: DeriveTarget::Class(class),
                })
            });
            generated.extend(items);
        }

        generated.extend(self.expand_class_members(class, scope));
        generated
    }

    fn process_interface(&mut self, iface: &mut InterfaceDecl, scope: &[String]) -> Vec<Item> {
        let iface_name = qualified_name(scope, &iface.name);
        let (invocations, errors) = collect_invocations(&mut iface.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        for invocation in &invocations {
            self.diagnostics.push(unsupported_macro(
                invocation,
                &format!("interface `{iface_name}`"),
            ));
        }
        self.expand_interface_members(iface, scope);
        Vec::new()
    }

    fn process_extension(&mut self, ext: &mut ExtensionDecl, scope: &[String]) -> Vec<Item> {
        let target_name = qualified_name(scope, &ext.target.name);
        let (invocations, errors) = collect_invocations(&mut ext.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        for invocation in &invocations {
            self.diagnostics.push(unsupported_macro(
                invocation,
                &format!("extension `{target_name}`"),
            ));
        }
        self.expand_extension_members(ext, scope)
    }

    fn process_function(
        &mut self,
        function: &mut FunctionDecl,
        owner: Option<&str>,
        scope: &[String],
    ) -> Vec<Item> {
        let (invocations, errors) =
            collect_invocations(&mut function.attributes, &mut self.hygiene);
        self.diagnostics.extend(errors);
        let mut generated = Vec::new();
        let mut derive_calls = Vec::new();
        let mut attribute_calls = Vec::new();

        for invocation in invocations {
            match invocation.kind {
                MacroInvocationKind::Derive => derive_calls.push(invocation),
                MacroInvocationKind::Attribute => attribute_calls.push(invocation),
            }
        }

        let context = match owner {
            Some(owner_name) => format!("method `{owner_name}::{}`", function.name),
            None => format!("function `{}`", qualified_name(scope, &function.name)),
        };

        for invocation in &derive_calls {
            self.diagnostics
                .push(unsupported_macro(invocation, &context));
        }

        let owner_string = owner.map(|value| value.to_string());
        for invocation in &attribute_calls {
            let Some(handler) = self.registry.get_attribute(&invocation.name) else {
                self.diagnostics.push(unknown_macro(invocation, &context));
                continue;
            };
            let target = match owner_string.as_ref() {
                Some(owner_name) => AttributeTarget::Method {
                    owner: owner_name.clone(),
                    function,
                },
                None => AttributeTarget::Function(function),
            };
            let items = self.execute_attribute(invocation, &context, || {
                handler(AttributeInput { invocation, target })
            });
            generated.extend(items);
        }

        if let Some(body) = &mut function.body {
            self.expand_block(body, scope);
        }

        generated
    }

    fn expand_struct_members(&mut self, strct: &mut StructDecl, scope: &[String]) {
        for field in &mut strct.fields {
            let (invocations, errors) =
                collect_invocations(&mut field.attributes, &mut self.hygiene);
            self.diagnostics.extend(errors);
            for invocation in &invocations {
                self.diagnostics.push(unsupported_macro(
                    invocation,
                    &format!("field `{}` of struct `{}`", field.name, strct.name),
                ));
            }
        }
        if !strct.nested_types.is_empty() {
            let mut nested_scope = scope.to_vec();
            nested_scope.push(strct.name.clone());
            self.expand_items(&mut strct.nested_types, nested_scope);
        }
    }

    fn expand_union_members(&mut self, union_def: &mut UnionDecl, scope: &[String]) {
        let union_name = qualified_name(scope, &union_def.name);
        for member in &mut union_def.members {
            match member {
                UnionMember::Field(field) => {
                    let (invocations, errors) =
                        collect_invocations(&mut field.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    for invocation in &invocations {
                        self.diagnostics.push(unsupported_macro(
                            invocation,
                            &format!("field `{}` of union `{union_name}`", field.name),
                        ));
                    }
                }
                UnionMember::View(view) => {
                    let (invocations, errors) =
                        collect_invocations(&mut view.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    for invocation in &invocations {
                        self.diagnostics.push(unsupported_macro(
                            invocation,
                            &format!("view `{}` of union `{union_name}`", view.name),
                        ));
                    }
                }
            }
        }
    }

    fn expand_enum_members(&mut self, enm: &mut EnumDecl, scope: &[String]) {
        let enum_name = qualified_name(scope, &enm.name);
        for variant in &mut enm.variants {
            for field in &mut variant.fields {
                let (invocations, errors) =
                    collect_invocations(&mut field.attributes, &mut self.hygiene);
                self.diagnostics.extend(errors);
                for invocation in &invocations {
                    self.diagnostics.push(unsupported_macro(
                        invocation,
                        &format!(
                            "field `{}` of enum `{enum_name}` variant `{}`",
                            field.name, variant.name
                        ),
                    ));
                }
            }
        }
    }

    fn expand_class_members(&mut self, class: &mut ClassDecl, scope: &[String]) -> Vec<Item> {
        let mut generated = Vec::new();
        let class_name = qualified_name(scope, &class.name);
        for member in &mut class.members {
            match member {
                ClassMember::Field(field) => {
                    let (invocations, errors) =
                        collect_invocations(&mut field.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    let context = format!("field `{}` of class `{class_name}`", field.name);
                    for invocation in &invocations {
                        self.diagnostics
                            .push(unsupported_macro(invocation, &context));
                    }
                }
                ClassMember::Method(method) => {
                    generated.extend(self.process_function(method, Some(&class_name), scope));
                }
                ClassMember::Property(prop) => {
                    let (invocations, errors) =
                        collect_invocations(&mut prop.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    let context = format!("property `{}` of class `{class_name}`", prop.name);
                    for invocation in &invocations {
                        self.diagnostics
                            .push(unsupported_macro(invocation, &context));
                    }
                    for accessor in &mut prop.accessors {
                        if let Some(attrs) = accessor.attributes.as_mut() {
                            let (invocations, errors) =
                                collect_invocations(attrs, &mut self.hygiene);
                            self.diagnostics.extend(errors);
                            for invocation in &invocations {
                                self.diagnostics.push(unsupported_macro(
                                    invocation,
                                    &format!(
                                        "property accessor `{}` of class `{class_name}`",
                                        prop.name
                                    ),
                                ));
                            }
                        }
                    }
                }
                ClassMember::Constructor(_) | ClassMember::Const(_) => {}
            }
        }
        generated
    }

    fn expand_interface_members(&mut self, iface: &mut InterfaceDecl, _scope: &[String]) {
        let iface_name = &iface.name;
        for member in &mut iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    let (invocations, errors) =
                        collect_invocations(&mut method.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    let context = format!("method `{}` of interface `{iface_name}`", method.name);
                    for invocation in &invocations {
                        self.diagnostics
                            .push(unsupported_macro(invocation, &context));
                    }
                }
                InterfaceMember::Property(prop) => {
                    let (invocations, errors) =
                        collect_invocations(&mut prop.attributes, &mut self.hygiene);
                    self.diagnostics.extend(errors);
                    let context = format!("property `{}` of interface `{iface_name}`", prop.name);
                    for invocation in &invocations {
                        self.diagnostics
                            .push(unsupported_macro(invocation, &context));
                    }
                }
                InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => {}
            }
        }
    }

    fn expand_extension_members(&mut self, ext: &mut ExtensionDecl, scope: &[String]) -> Vec<Item> {
        let mut generated = Vec::new();
        for member in &mut ext.members {
            match member {
                ExtensionMember::Method(method) => {
                    generated.extend(self.process_function(
                        &mut method.function,
                        Some(&ext.target.name),
                        scope,
                    ));
                }
            }
        }
        generated
    }

    fn expand_block(&mut self, block: &mut Block, scope: &[String]) {
        for statement in &mut block.statements {
            if let Some(attrs) = statement.attributes.as_mut() {
                let (invocations, errors) = collect_invocations(attrs, &mut self.hygiene);
                self.diagnostics.extend(errors);
                for invocation in &invocations {
                    self.diagnostics
                        .push(unsupported_macro(invocation, "statement"));
                }
            }
            self.expand_statement(statement, scope);
        }
    }

    fn expand_statement(&mut self, statement: &mut Statement, scope: &[String]) {
        match &mut statement.kind {
            StatementKind::Block(inner) => self.expand_block(inner, scope),
            StatementKind::If(if_stmt) => {
                self.expand_statement(&mut if_stmt.then_branch, scope);
                if let Some(else_branch) = if_stmt.else_branch.as_mut() {
                    self.expand_statement(else_branch, scope);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::DoWhile { body, .. }
            | StatementKind::Lock { body, .. }
            | StatementKind::Unsafe { body } => {
                self.expand_statement(body, scope);
            }
            StatementKind::For(for_stmt) => {
                self.expand_statement(&mut for_stmt.body, scope);
            }
            StatementKind::Foreach(foreach_stmt) => {
                self.expand_statement(&mut foreach_stmt.body, scope);
            }
            StatementKind::Region { body, .. } => self.expand_block(body, scope),
            StatementKind::Switch(switch_stmt) => {
                for section in &mut switch_stmt.sections {
                    for stmt in &mut section.statements {
                        self.expand_statement(stmt, scope);
                    }
                }
            }
            StatementKind::Try(try_stmt) => {
                self.expand_block(&mut try_stmt.body, scope);
                for catch in &mut try_stmt.catches {
                    self.expand_block(&mut catch.body, scope);
                }
                if let Some(finally_block) = try_stmt.finally.as_mut() {
                    self.expand_block(finally_block, scope);
                }
            }
            StatementKind::Using(using_stmt) => {
                if let Some(body) = using_stmt.body.as_mut() {
                    self.expand_statement(body, scope);
                }
            }
            StatementKind::Atomic { body, .. } => {
                self.expand_block(body, scope);
            }
            StatementKind::Checked { body } | StatementKind::Unchecked { body } => {
                self.expand_block(body, scope);
            }
            StatementKind::Fixed(fixed_stmt) => {
                self.expand_statement(&mut fixed_stmt.body, scope);
            }
            StatementKind::LocalFunction(function) => {
                if let Some(body) = function.body.as_mut() {
                    self.expand_block(body, scope);
                }
            }
            StatementKind::Labeled {
                statement: inner, ..
            } => {
                self.expand_statement(inner, scope);
            }
            StatementKind::YieldReturn { .. }
            | StatementKind::YieldBreak
            | StatementKind::Return { .. }
            | StatementKind::Break
            | StatementKind::Continue
            | StatementKind::Goto(_)
            | StatementKind::Throw { .. }
            | StatementKind::Expression(_)
            | StatementKind::VariableDeclaration(_)
            | StatementKind::ConstDeclaration(_)
            | StatementKind::Empty => {}
        }
    }

    fn execute_derive<F>(&mut self, invocation: &MacroInvocation, target: &str, f: F) -> Vec<Item>
    where
        F: FnOnce() -> DeriveOutput,
    {
        let key = InvocationCacheKey::new(invocation, target);
        self.progress = true;
        if let Some(cached) = self.cache.lookup(&key) {
            self.diagnostics.extend(cached.diagnostics.clone());
            return cached.items;
        }

        let mut output = f();
        stamp_items_with_origin(&mut output.new_items, invocation.span);
        self.cache.store(
            key,
            CachedExpansion::new(output.new_items.clone(), output.diagnostics.clone()),
        );
        self.diagnostics.append(&mut output.diagnostics);
        output.new_items
    }

    fn execute_attribute<F>(
        &mut self,
        invocation: &MacroInvocation,
        context: &str,
        f: F,
    ) -> Vec<Item>
    where
        F: FnOnce() -> AttributeOutput,
    {
        let key = InvocationCacheKey::new(invocation, context);
        self.progress = true;
        if let Some(cached) = self.cache.lookup(&key) {
            self.diagnostics.extend(cached.diagnostics.clone());
            return cached.items;
        }

        let mut output = f();
        stamp_items_with_origin(&mut output.new_items, invocation.span);
        self.cache.store(
            key,
            CachedExpansion::new(output.new_items.clone(), output.diagnostics.clone()),
        );
        self.diagnostics.append(&mut output.diagnostics);
        output.new_items
    }
}

fn qualified_name(scope: &[String], name: &str) -> String {
    if scope.is_empty() {
        name.to_string()
    } else {
        let mut parts = scope.to_vec();
        parts.push(name.to_string());
        parts.join("::")
    }
}
