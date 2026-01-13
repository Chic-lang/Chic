use crate::frontend::ast::expressions::{
    Block, Expression, ForInitializer, Statement, StatementKind, UsingResource, VariableDeclaration,
};
use crate::frontend::ast::items::{
    Attribute, ClassDecl, ClassMember, ConstDeclaration, ConstructorDecl, EnumDecl, ExtensionDecl,
    ExtensionMember, FunctionDecl, ImplDecl, ImplMember, InterfaceDecl, InterfaceMember, Item,
    NamespaceDecl, PropertyAccessor, PropertyAccessorBody, PropertyDecl, StructDecl, TestCaseDecl,
    TraitDecl, TraitMember, TypeAliasDecl, UnionDecl, UnionMember,
};
use crate::frontend::diagnostics::Span;

/// Stamp generated items with the originating attribute's span so downstream diagnostics have a
/// stable location when macro-authored code fails type checking or lowering.
pub fn stamp_items_with_origin(items: &mut [Item], origin: Option<Span>) {
    for item in items {
        stamp_item(item, origin);
    }
}

fn stamp_item(item: &mut Item, origin: Option<Span>) {
    match item {
        Item::Namespace(ns) => stamp_namespace(ns, origin),
        Item::Struct(strct) => stamp_struct(strct, origin),
        Item::Union(union_def) => stamp_union(union_def, origin),
        Item::Enum(enm) => stamp_enum(enm, origin),
        Item::Class(class) => stamp_class(class, origin),
        Item::Interface(iface) => stamp_interface(iface, origin),
        Item::Extension(ext) => stamp_extension(ext, origin),
        Item::Function(func) => stamp_function(func, origin),
        Item::Impl(impl_decl) => stamp_impl(impl_decl, origin),
        Item::Trait(trait_decl) => stamp_trait(trait_decl, origin),
        Item::TestCase(testcase) => stamp_test_case(testcase, origin),
        Item::Delegate(delegate) => stamp_attributes(&mut delegate.attributes, origin),
        Item::TypeAlias(alias) => stamp_type_alias(alias, origin),
        Item::Import(_) | Item::Const(_) | Item::Static(_) => {}
    }
}

fn stamp_namespace(ns: &mut NamespaceDecl, origin: Option<Span>) {
    stamp_attributes(&mut ns.attributes, origin);
    for item in &mut ns.items {
        stamp_item(item, origin);
    }
}

fn stamp_test_case(testcase: &mut TestCaseDecl, origin: Option<Span>) {
    stamp_attributes(&mut testcase.attributes, origin);
    if let Some(signature) = testcase.signature.as_mut() {
        for parameter in &mut signature.parameters {
            stamp_attributes(&mut parameter.attributes, origin);
            if let Some(default) = parameter.default.as_mut() {
                stamp_expression(default, origin);
            }
        }
        if let Some(throws) = signature.throws.as_mut() {
            set_span_if_missing(&mut throws.span, origin);
        }
    }
    stamp_block(&mut testcase.body, origin);
}

fn stamp_struct(strct: &mut StructDecl, origin: Option<Span>) {
    stamp_attributes(&mut strct.attributes, origin);
    for ctor in &mut strct.constructors {
        stamp_constructor(ctor, origin);
    }
    for method in &mut strct.methods {
        stamp_function(method, origin);
    }
    for property in &mut strct.properties {
        stamp_property(property, origin);
    }
    for nested in &mut strct.nested_types {
        stamp_item(nested, origin);
    }
}

fn stamp_union(union_def: &mut UnionDecl, origin: Option<Span>) {
    stamp_attributes(&mut union_def.attributes, origin);
    for member in &mut union_def.members {
        match member {
            UnionMember::Field(field) => {
                stamp_attributes(&mut field.attributes, origin);
            }
            UnionMember::View(view) => {
                stamp_attributes(&mut view.attributes, origin);
                for field in &mut view.fields {
                    stamp_attributes(&mut field.attributes, origin);
                }
            }
        }
    }
}

fn stamp_enum(enm: &mut EnumDecl, origin: Option<Span>) {
    stamp_attributes(&mut enm.attributes, origin);
    for variant in &mut enm.variants {
        for field in &mut variant.fields {
            stamp_attributes(&mut field.attributes, origin);
        }
    }
}

fn stamp_class(class: &mut ClassDecl, origin: Option<Span>) {
    stamp_attributes(&mut class.attributes, origin);
    for member in &mut class.members {
        match member {
            ClassMember::Field(field) => stamp_attributes(&mut field.attributes, origin),
            ClassMember::Method(function) => stamp_function(function, origin),
            ClassMember::Property(property) => stamp_property(property, origin),
            ClassMember::Constructor(constructor) => stamp_constructor(constructor, origin),
            ClassMember::Const(constant) => stamp_const_decl(&mut constant.declaration, origin),
        }
    }
}

fn stamp_interface(iface: &mut InterfaceDecl, origin: Option<Span>) {
    stamp_attributes(&mut iface.attributes, origin);
    for member in &mut iface.members {
        match member {
            InterfaceMember::Method(function) => stamp_function(function, origin),
            InterfaceMember::Property(property) => stamp_property(property, origin),
            InterfaceMember::Const(constant) => stamp_const_decl(&mut constant.declaration, origin),
            InterfaceMember::AssociatedType(assoc) => set_span_if_missing(&mut assoc.span, origin),
        }
    }
}

fn stamp_extension(ext: &mut ExtensionDecl, origin: Option<Span>) {
    stamp_attributes(&mut ext.attributes, origin);
    for condition in &mut ext.conditions {
        set_span_if_missing(&mut condition.span, origin);
    }
    for member in &mut ext.members {
        match member {
            ExtensionMember::Method(method) => stamp_function(&mut method.function, origin),
        }
    }
}

fn stamp_impl(impl_decl: &mut ImplDecl, origin: Option<Span>) {
    stamp_attributes(&mut impl_decl.attributes, origin);
    set_span_if_missing(&mut impl_decl.span, origin);
    for member in &mut impl_decl.members {
        match member {
            ImplMember::Method(function) => stamp_function(function, origin),
            ImplMember::AssociatedType(assoc) => set_span_if_missing(&mut assoc.span, origin),
            ImplMember::Const(constant) => stamp_const_decl(&mut constant.declaration, origin),
        }
    }
}

fn stamp_trait(trait_decl: &mut TraitDecl, origin: Option<Span>) {
    stamp_attributes(&mut trait_decl.attributes, origin);
    set_span_if_missing(&mut trait_decl.span, origin);
    for member in &mut trait_decl.members {
        match member {
            TraitMember::Method(function) => stamp_function(function, origin),
            TraitMember::AssociatedType(assoc) => set_span_if_missing(&mut assoc.span, origin),
            TraitMember::Const(constant) => stamp_const_decl(&mut constant.declaration, origin),
        }
    }
}

fn stamp_type_alias(alias: &mut TypeAliasDecl, origin: Option<Span>) {
    stamp_attributes(&mut alias.attributes, origin);
    set_span_if_missing(&mut alias.span, origin);
    if let Some(generics) = alias.generics.as_mut() {
        set_span_if_missing(&mut generics.span, origin);
        for param in &mut generics.params {
            set_span_if_missing(&mut param.span, origin);
        }
    }
    set_span_if_missing(&mut alias.target.span, origin);
}

fn stamp_constructor(constructor: &mut ConstructorDecl, origin: Option<Span>) {
    stamp_attributes(&mut constructor.attributes, origin);
    set_span_if_missing(&mut constructor.span, origin);
    if let Some(body) = &mut constructor.body {
        stamp_block(body, origin);
    }
}

fn stamp_function(function: &mut FunctionDecl, origin: Option<Span>) {
    stamp_attributes(&mut function.attributes, origin);
    for parameter in &mut function.signature.parameters {
        stamp_attributes(&mut parameter.attributes, origin);
        if let Some(default) = parameter.default.as_mut() {
            stamp_expression(default, origin);
        }
    }
    if let Some(throws) = function.signature.throws.as_mut() {
        set_span_if_missing(&mut throws.span, origin);
    }
    if let Some(body) = &mut function.body {
        stamp_block(body, origin);
    }
    if let Some(operator) = function.operator.as_mut() {
        set_span_if_missing(&mut operator.span, origin);
    }
    if let Some(options) = function.extern_options.as_mut() {
        set_span_if_missing(&mut options.span, origin);
    }
    if let Some(generics) = function.generics.as_mut() {
        set_span_if_missing(&mut generics.span, origin);
        for param in &mut generics.params {
            set_span_if_missing(&mut param.span, origin);
        }
    }
}

fn stamp_property(property: &mut PropertyDecl, origin: Option<Span>) {
    stamp_attributes(&mut property.attributes, origin);
    set_span_if_missing(&mut property.span, origin);
    for parameter in &mut property.parameters {
        stamp_attributes(&mut parameter.attributes, origin);
        if let Some(default) = parameter.default.as_mut() {
            stamp_expression(default, origin);
        }
    }
    if let Some(initializer) = property.initializer.as_mut() {
        stamp_expression(initializer, origin);
    }
    for accessor in &mut property.accessors {
        stamp_property_accessor(accessor, origin);
    }
}

fn stamp_property_accessor(accessor: &mut PropertyAccessor, origin: Option<Span>) {
    if let Some(attributes) = accessor.attributes.as_mut() {
        stamp_attributes(attributes, origin);
    }
    set_span_if_missing(&mut accessor.span, origin);
    match &mut accessor.body {
        PropertyAccessorBody::Auto => {}
        PropertyAccessorBody::Block(block) => stamp_block(block, origin),
        PropertyAccessorBody::Expression(expr) => stamp_expression(expr, origin),
    }
}

fn stamp_const_decl(decl: &mut ConstDeclaration, origin: Option<Span>) {
    set_span_if_missing(&mut decl.span, origin);
    for declarator in &mut decl.declarators {
        set_span_if_missing(&mut declarator.span, origin);
        stamp_expression(&mut declarator.initializer, origin);
    }
}

fn stamp_block(block: &mut Block, origin: Option<Span>) {
    set_span_if_missing(&mut block.span, origin);
    for statement in &mut block.statements {
        stamp_statement(statement, origin);
    }
}

fn stamp_statement(statement: &mut Statement, origin: Option<Span>) {
    set_span_if_missing(&mut statement.span, origin);
    match &mut statement.kind {
        StatementKind::Block(inner) => stamp_block(inner, origin),
        StatementKind::If(if_stmt) => {
            stamp_expression(&mut if_stmt.condition, origin);
            stamp_statement(&mut if_stmt.then_branch, origin);
            if let Some(else_branch) = if_stmt.else_branch.as_mut() {
                stamp_statement(else_branch, origin);
            }
        }
        StatementKind::While { condition, body } => {
            stamp_expression(condition, origin);
            stamp_statement(body, origin);
        }
        StatementKind::DoWhile { body, condition } => {
            stamp_statement(body, origin);
            stamp_expression(condition, origin);
        }
        StatementKind::For(for_stmt) => {
            if let Some(initializer) = for_stmt.initializer.as_mut() {
                match initializer {
                    ForInitializer::Declaration(decl) => stamp_variable_declaration(decl, origin),
                    ForInitializer::Const(const_stmt) => {
                        stamp_const_decl(&mut const_stmt.declaration, origin)
                    }
                    ForInitializer::Expressions(exprs) => {
                        for expr in exprs {
                            stamp_expression(expr, origin);
                        }
                    }
                }
            }
            if let Some(condition) = for_stmt.condition.as_mut() {
                stamp_expression(condition, origin);
            }
            for expr in &mut for_stmt.iterator {
                stamp_expression(expr, origin);
            }
            stamp_statement(&mut for_stmt.body, origin);
        }
        StatementKind::Region { body, .. } => stamp_block(body, origin),
        StatementKind::Foreach(foreach_stmt) => {
            stamp_expression(&mut foreach_stmt.expression, origin);
            stamp_statement(&mut foreach_stmt.body, origin);
        }
        StatementKind::Switch(switch_stmt) => {
            stamp_expression(&mut switch_stmt.expression, origin);
            for section in &mut switch_stmt.sections {
                for stmt in &mut section.statements {
                    stamp_statement(stmt, origin);
                }
            }
        }
        StatementKind::Try(try_stmt) => {
            stamp_block(&mut try_stmt.body, origin);
            for catch in &mut try_stmt.catches {
                if let Some(filter) = catch.filter.as_mut() {
                    stamp_expression(filter, origin);
                }
                stamp_block(&mut catch.body, origin);
            }
            if let Some(finally_block) = try_stmt.finally.as_mut() {
                stamp_block(finally_block, origin);
            }
        }
        StatementKind::Using(using_stmt) => {
            match &mut using_stmt.resource {
                UsingResource::Expression(expr) => stamp_expression(expr, origin),
                UsingResource::Declaration(decl) => stamp_variable_declaration(decl, origin),
            }
            if let Some(body) = using_stmt.body.as_mut() {
                stamp_statement(body, origin);
            }
        }
        StatementKind::Lock { expression, body } => {
            stamp_expression(expression, origin);
            stamp_statement(body, origin);
        }
        StatementKind::Atomic { ordering, body } => {
            if let Some(ordering) = ordering.as_mut() {
                stamp_expression(ordering, origin);
            }
            stamp_block(body, origin);
        }
        StatementKind::Checked { body } | StatementKind::Unchecked { body } => {
            stamp_block(body, origin);
        }
        StatementKind::Fixed(fixed_stmt) => {
            stamp_variable_declaration(&mut fixed_stmt.declaration, origin);
            stamp_statement(&mut fixed_stmt.body, origin);
        }
        StatementKind::Unsafe { body } => stamp_statement(body, origin),
        StatementKind::LocalFunction(function) => {
            if let Some(body) = function.body.as_mut() {
                stamp_block(body, origin);
            }
        }
        StatementKind::Labeled {
            statement: inner, ..
        } => {
            stamp_statement(inner, origin);
        }
        StatementKind::YieldReturn { expression } => stamp_expression(expression, origin),
        StatementKind::Return { expression } => {
            if let Some(expr) = expression {
                stamp_expression(expr, origin);
            }
        }
        StatementKind::Throw { expression } => {
            if let Some(expr) = expression {
                stamp_expression(expr, origin);
            }
        }
        StatementKind::Expression(expr) => stamp_expression(expr, origin),
        StatementKind::VariableDeclaration(var) => {
            stamp_variable_declaration(var, origin);
        }
        StatementKind::ConstDeclaration(const_decl) => {
            stamp_const_decl(&mut const_decl.declaration, origin);
        }
        StatementKind::YieldBreak
        | StatementKind::Break
        | StatementKind::Continue
        | StatementKind::Goto(_)
        | StatementKind::Empty => {}
    }
}

fn stamp_variable_declaration(var: &mut VariableDeclaration, origin: Option<Span>) {
    for declarator in &mut var.declarators {
        if let Some(initializer) = declarator.initializer.as_mut() {
            stamp_expression(initializer, origin);
        }
    }
}

fn stamp_expression(expr: &mut Expression, origin: Option<Span>) {
    set_span_if_missing(&mut expr.span, origin);
}

fn stamp_attributes(attrs: &mut Vec<Attribute>, origin: Option<Span>) {
    if let Some(span) = origin {
        for attr in attrs {
            set_span_if_missing(&mut attr.span, Some(span));
        }
    }
}

fn set_span_if_missing(target: &mut Option<Span>, origin: Option<Span>) {
    if target.is_none() {
        *target = origin;
    }
}
