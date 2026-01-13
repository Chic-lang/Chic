//! Helpers for interpreting surface attributes into semantic hints.

use crate::frontend::ast::{
    Attribute, AttributeArgument, ClassDecl, ClassMember, ConstructorDecl, DiInjectAttr,
    DiLifetime, DiServiceAttr, ExtensionMember, FunctionDecl, InterfaceDecl, InterfaceMember, Item,
    Module, Parameter, PropertyDecl, StructDecl, TestCaseDecl,
};
use crate::frontend::diagnostics::{Diagnostic, Span};

/// Layout-related hints derived from `@repr`/`@align` attributes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LayoutHints {
    pub repr_c: bool,
    pub packing: Option<PackingHint>,
    pub align: Option<AlignHint>,
}

impl LayoutHints {
    #[must_use]
    pub fn has_packing(&self) -> bool {
        self.packing.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackingHint {
    pub value: Option<u32>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignHint {
    pub value: u32,
    pub span: Option<Span>,
}

/// Trace annotation extracted from `@trace`.
#[derive(Debug, Clone)]
pub struct TraceAttr {
    pub label: Option<String>,
    pub level: Option<String>,
    pub span: Option<Span>,
}

/// Cost budgets extracted from `@cost`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostAttr {
    pub cpu_budget_us: Option<u64>,
    pub gpu_budget_us: Option<u64>,
    pub mem_budget_bytes: Option<u64>,
    pub span: Option<Span>,
}

/// Conditional compilation marker extracted from `@conditional`.
#[derive(Debug, Clone)]
pub struct ConditionalAttribute {
    pub symbol: String,
    pub span: Option<Span>,
}

/// Code generation hints applied to a function or testcase.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OptimizationHints {
    pub hot: bool,
    pub cold: bool,
    pub always_inline: bool,
    pub never_inline: bool,
}

impl OptimizationHints {
    #[must_use]
    pub fn is_empty(self) -> bool {
        !(self.hot || self.cold || self.always_inline || self.never_inline)
    }
}

#[derive(Debug, Clone)]
pub struct AttributeError {
    pub message: String,
    pub span: Option<Span>,
}

impl AttributeError {
    #[must_use]
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

/// Apply staged attribute semantics (dependency injection, module flags, etc.) after parsing.
#[must_use]
pub fn stage_builtin_attributes(module: &mut Module) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    stage_items(&mut module.items, &mut diagnostics);
    diagnostics
}

fn stage_items(items: &mut [Item], diagnostics: &mut Vec<Diagnostic>) {
    for item in items {
        stage_item(item, diagnostics);
    }
}

fn stage_item(item: &mut Item, diagnostics: &mut Vec<Diagnostic>) {
    match item {
        Item::Namespace(namespace) => stage_items(&mut namespace.items, diagnostics),
        Item::Class(class) => stage_class(class, diagnostics),
        Item::Struct(strct) => stage_struct(strct, diagnostics),
        Item::Interface(interface) => stage_interface(interface, diagnostics),
        Item::Extension(extension) => {
            for member in &mut extension.members {
                match member {
                    ExtensionMember::Method(method) => {
                        stage_function(&mut method.function, diagnostics);
                    }
                }
            }
        }
        Item::Function(function) => stage_function(function, diagnostics),
        Item::TestCase(testcase) => stage_testcase(testcase, diagnostics),
        Item::Trait(_)
        | Item::Impl(_)
        | Item::Enum(_)
        | Item::Union(_)
        | Item::Delegate(_)
        | Item::TypeAlias(_)
        | Item::Import(_)
        | Item::Const(_)
        | Item::Static(_) => {}
    }
}

fn stage_class(class: &mut ClassDecl, diagnostics: &mut Vec<Diagnostic>) {
    class.di_service = None;
    class.di_module = false;

    let (service_attr, errors) = extract_service_attribute(&class.attributes);
    push_attribute_errors(diagnostics, errors);
    class.di_service = service_attr;

    let (is_module, errors) = extract_module_attribute(&class.attributes);
    push_attribute_errors(diagnostics, errors);
    class.di_module = is_module;

    for member in &mut class.members {
        stage_class_member(member, diagnostics);
    }
}

fn stage_struct(strct: &mut StructDecl, diagnostics: &mut Vec<Diagnostic>) {
    for constructor in &mut strct.constructors {
        stage_constructor(constructor, diagnostics);
    }
    for method in &mut strct.methods {
        stage_function(method, diagnostics);
    }
    for property in &mut strct.properties {
        stage_property(property, diagnostics);
    }
    stage_items(&mut strct.nested_types, diagnostics);
}

fn stage_interface(interface: &mut InterfaceDecl, diagnostics: &mut Vec<Diagnostic>) {
    for member in &mut interface.members {
        match member {
            InterfaceMember::Method(function) => stage_function(function, diagnostics),
            InterfaceMember::Property(property) => stage_property(property, diagnostics),
            InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => {}
        }
    }
}

fn stage_class_member(member: &mut ClassMember, diagnostics: &mut Vec<Diagnostic>) {
    match member {
        ClassMember::Constructor(constructor) => stage_constructor(constructor, diagnostics),
        ClassMember::Method(function) => stage_function(function, diagnostics),
        ClassMember::Property(property) => stage_property(property, diagnostics),
        ClassMember::Field(_) | ClassMember::Const(_) => {}
    }
}

fn stage_constructor(constructor: &mut ConstructorDecl, diagnostics: &mut Vec<Diagnostic>) {
    let (inject, errors) = extract_inject_attribute(&constructor.attributes);
    push_attribute_errors(diagnostics, errors);
    constructor.di_inject = inject;

    stage_parameters(&mut constructor.parameters, diagnostics);
}

fn stage_property(property: &mut PropertyDecl, diagnostics: &mut Vec<Diagnostic>) {
    let (inject, errors) = extract_inject_attribute(&property.attributes);
    push_attribute_errors(diagnostics, errors);
    property.di_inject = inject;
    stage_parameters(&mut property.parameters, diagnostics);
}

fn stage_function(function: &mut FunctionDecl, diagnostics: &mut Vec<Diagnostic>) {
    stage_parameters(&mut function.signature.parameters, diagnostics);
}

fn stage_testcase(testcase: &mut TestCaseDecl, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(signature) = &mut testcase.signature {
        stage_parameters(&mut signature.parameters, diagnostics);
    }
}

fn stage_parameters(parameters: &mut [Parameter], diagnostics: &mut Vec<Diagnostic>) {
    for parameter in parameters {
        let (inject, errors) = extract_inject_attribute(&parameter.attributes);
        push_attribute_errors(diagnostics, errors);
        parameter.di_inject = inject;
    }
}

fn push_attribute_errors(diagnostics: &mut Vec<Diagnostic>, errors: Vec<AttributeError>) {
    for error in errors {
        diagnostics.push(Diagnostic::error(error.message, error.span));
    }
}

/// Extract layout hints (`@repr`, `@align`) from a declaration's attributes.
#[must_use]
pub fn collect_layout_hints(attrs: &[Attribute]) -> (LayoutHints, Vec<AttributeError>) {
    let mut hints = LayoutHints::default();
    let mut errors = Vec::new();

    for attr in attrs {
        let lowered = attr.name.to_ascii_lowercase();
        match lowered.as_str() {
            "repr" => parse_repr_attribute(attr, &mut hints, &mut errors),
            "align" => parse_align_attribute(attr, &mut hints, &mut errors),
            _ => {}
        }
    }

    (hints, errors)
}

#[must_use]
pub fn has_fallible_attr(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.name.eq_ignore_ascii_case("fallible"))
}

/// Extract codegen/optimization hints (`@hot`, `@cold`, `@always_inline`, `@never_inline`).
#[must_use]
pub fn collect_optimization_hints(attrs: &[Attribute]) -> (OptimizationHints, Vec<AttributeError>) {
    let mut hints = OptimizationHints::default();
    let mut errors = Vec::new();
    let mut hot_span = None;
    let mut cold_span = None;
    let mut always_inline_span = None;
    let mut never_inline_span = None;

    for attr in attrs {
        let lowered = attr.name.to_ascii_lowercase();
        match lowered.as_str() {
            "hot" => {
                if hints.hot {
                    errors.push(AttributeError::new("duplicate `@hot` attribute", attr.span));
                }
                hints.hot = true;
                hot_span = hot_span.or(attr.span);
            }
            "cold" => {
                if hints.cold {
                    errors.push(AttributeError::new(
                        "duplicate `@cold` attribute",
                        attr.span,
                    ));
                }
                hints.cold = true;
                cold_span = cold_span.or(attr.span);
            }
            "always_inline" | "alwaysinline" => {
                if hints.always_inline {
                    errors.push(AttributeError::new(
                        "duplicate `@always_inline` attribute",
                        attr.span,
                    ));
                }
                hints.always_inline = true;
                always_inline_span = always_inline_span.or(attr.span);
            }
            "never_inline" | "neverinline" => {
                if hints.never_inline {
                    errors.push(AttributeError::new(
                        "duplicate `@never_inline` attribute",
                        attr.span,
                    ));
                }
                hints.never_inline = true;
                never_inline_span = never_inline_span.or(attr.span);
            }
            _ => {}
        }
    }

    if hints.hot && hints.cold {
        errors.push(AttributeError::new(
            "cannot combine `@hot` and `@cold` on the same function",
            hot_span.or(cold_span),
        ));
    }
    if hints.always_inline && hints.never_inline {
        errors.push(AttributeError::new(
            "cannot combine `@always_inline` and `@never_inline` on the same function",
            always_inline_span.or(never_inline_span),
        ));
    }

    (hints, errors)
}

/// Resolved export directive extracted from `@export`.
#[derive(Debug, Clone)]
pub struct ExportAttr {
    pub symbol: String,
    pub span: Option<Span>,
}

/// Extract explicit export directives attached to a declaration.
#[must_use]
pub fn collect_export_attributes(attrs: &[Attribute]) -> (Vec<ExportAttr>, Vec<AttributeError>) {
    let mut exports = Vec::new();
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("export") {
            continue;
        }

        if attr.arguments.len() != 1 {
            errors.push(AttributeError::new(
                "`@export` requires exactly one argument specifying the export symbol",
                attr.span,
            ));
            continue;
        }

        let argument = &attr.arguments[0];
        if let Some(name) = &argument.name {
            if !name.eq_ignore_ascii_case("name") && !name.is_empty() {
                errors.push(AttributeError::new(
                    format!("unsupported named argument `{name}` for `@export`"),
                    argument.span.or(attr.span),
                ));
                continue;
            }
        }

        let trimmed = trim_quotes(argument.value.trim());
        if trimmed.is_empty() {
            errors.push(AttributeError::new(
                "`@export` argument must not be empty",
                argument.span.or(attr.span),
            ));
            continue;
        }

        exports.push(ExportAttr {
            symbol: trimmed.to_string(),
            span: argument.span.or(attr.span),
        });
    }

    (exports, errors)
}

/// Extract trace annotations from `@trace` attributes attached to a declaration.
#[must_use]
pub fn collect_trace_attribute(attrs: &[Attribute]) -> (Option<TraceAttr>, Vec<AttributeError>) {
    let mut trace: Option<TraceAttr> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("trace") {
            continue;
        }
        if trace.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@trace` attribute",
                attr.span,
            ));
            continue;
        }

        let mut label: Option<String> = None;
        let mut level: Option<String> = None;

        for (index, argument) in attr.arguments.iter().enumerate() {
            let value = trim_quotes(argument.value.trim());
            if value.is_empty() {
                errors.push(AttributeError::new(
                    "`@trace` arguments must not be empty",
                    argument.span.or(attr.span),
                ));
                continue;
            }
            match argument
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase())
            {
                Some(name) if name == "label" => {
                    label = Some(value.to_string());
                }
                Some(name) if name == "level" => {
                    level = Some(value.to_string());
                }
                Some(other) => errors.push(AttributeError::new(
                    format!("unsupported named argument `{other}` for `@trace`"),
                    argument.span.or(attr.span),
                )),
                None => {
                    if index == 0 {
                        label = Some(value.to_string());
                    } else if index == 1 {
                        level = Some(value.to_string());
                    } else {
                        errors.push(AttributeError::new(
                            "`@trace` accepts at most two positional arguments (label, level)`",
                            argument.span.or(attr.span),
                        ));
                    }
                }
            }
        }

        trace = Some(TraceAttr {
            label,
            level,
            span: attr.span,
        });
    }

    (trace, errors)
}

/// Extract `@cost` annotations describing static budgets for a declaration.
#[must_use]
pub fn collect_cost_attribute(attrs: &[Attribute]) -> (Option<CostAttr>, Vec<AttributeError>) {
    let mut cost: Option<CostAttr> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("cost") {
            continue;
        }
        if cost.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@cost` attribute",
                attr.span,
            ));
            continue;
        }
        let mut cpu_budget_us = None;
        let mut gpu_budget_us = None;
        let mut mem_budget_bytes = None;

        for argument in &attr.arguments {
            let key = argument
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase());
            let value = trim_quotes(argument.value.trim());
            if value.is_empty() {
                errors.push(AttributeError::new(
                    "`@cost` arguments must not be empty",
                    argument.span.or(attr.span),
                ));
                continue;
            }
            match key.as_deref() {
                Some("cpu") => match parse_microseconds(value) {
                    Ok(parsed) => cpu_budget_us = Some(parsed),
                    Err(message) => {
                        errors.push(AttributeError::new(message, argument.span.or(attr.span)))
                    }
                },
                Some("gpu") => match parse_microseconds(value) {
                    Ok(parsed) => gpu_budget_us = Some(parsed),
                    Err(message) => {
                        errors.push(AttributeError::new(message, argument.span.or(attr.span)))
                    }
                },
                Some("mem" | "memory") => match parse_bytes(value) {
                    Ok(parsed) => mem_budget_bytes = Some(parsed),
                    Err(message) => {
                        errors.push(AttributeError::new(message, argument.span.or(attr.span)))
                    }
                },
                Some(other) => errors.push(AttributeError::new(
                    format!("unsupported named argument `{other}` for `@cost`"),
                    argument.span.or(attr.span),
                )),
                None => errors.push(AttributeError::new(
                    "positional arguments are not supported for `@cost`",
                    argument.span.or(attr.span),
                )),
            }
        }

        cost = Some(CostAttr {
            cpu_budget_us,
            gpu_budget_us,
            mem_budget_bytes,
            span: attr.span,
        });
    }

    (cost, errors)
}

fn parse_di_lifetime_value(value: &str) -> Option<DiLifetime> {
    match value.to_ascii_lowercase().as_str() {
        "transient" => Some(DiLifetime::Transient),
        "scoped" => Some(DiLifetime::Scoped),
        "singleton" => Some(DiLifetime::Singleton),
        "threadlocal" | "thread_local" | "thread-local" => Some(DiLifetime::ThreadLocal),
        _ => None,
    }
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_numeric(value: &str) -> Option<u64> {
    let mut cleaned = value.replace('_', "");
    cleaned = cleaned.trim().to_string();
    cleaned.parse::<u64>().ok()
}

fn parse_microseconds(value: &str) -> Result<u64, String> {
    let trimmed = value.trim().to_ascii_lowercase();
    let numeric = trimmed
        .strip_suffix("us")
        .or_else(|| trimmed.strip_suffix("µs"))
        .unwrap_or(&trimmed);
    parse_numeric(numeric)
        .ok_or_else(|| format!("`@cost` expected a microsecond value, received `{value}`"))
}

fn parse_bytes(value: &str) -> Result<u64, String> {
    let trimmed = value.trim().to_ascii_lowercase();
    let numeric = trimmed
        .strip_suffix("bytes")
        .or_else(|| trimmed.strip_suffix('b'))
        .unwrap_or(&trimmed);
    parse_numeric(numeric)
        .ok_or_else(|| format!("`@cost` expected a byte value, received `{value}`"))
}

/// Extracts dependency-injection metadata from `@service` attributes.
#[must_use]
pub fn extract_service_attribute(
    attrs: &[Attribute],
) -> (Option<DiServiceAttr>, Vec<AttributeError>) {
    let mut service: Option<DiServiceAttr> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("service") {
            continue;
        }

        if service.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@service` attribute",
                attr.span,
            ));
            continue;
        }

        let mut lifetime: Option<DiLifetime> = None;
        let mut named: Option<String> = None;

        for argument in &attr.arguments {
            let key = argument
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase());
            let trimmed = trim_quotes(argument.value.trim());
            if trimmed.is_empty() {
                errors.push(AttributeError::new(
                    "`@service` attribute argument must not be empty",
                    argument.span.or(attr.span),
                ));
                continue;
            }

            match key.as_deref() {
                Some("lifetime") => match parse_di_lifetime_value(trimmed) {
                    Some(value) => lifetime = Some(value),
                    None => errors.push(AttributeError::new(
                        format!("unsupported DI lifetime `{trimmed}`"),
                        argument.span.or(attr.span),
                    )),
                },
                Some("named") => {
                    named = Some(trimmed.to_string());
                }
                Some(other) => {
                    errors.push(AttributeError::new(
                        format!("unsupported named argument `{other}` for `@service`"),
                        argument.span.or(attr.span),
                    ));
                }
                None => {
                    if lifetime.is_none() {
                        match parse_di_lifetime_value(trimmed) {
                            Some(value) => lifetime = Some(value),
                            None => named = Some(trimmed.to_string()),
                        }
                    } else if named.is_none() {
                        named = Some(trimmed.to_string());
                    } else {
                        errors.push(AttributeError::new(
                            "`@service` accepts at most one unnamed argument",
                            argument.span.or(attr.span),
                        ));
                    }
                }
            }
        }

        service = Some(DiServiceAttr::new(lifetime, named));
    }

    (service, errors)
}

/// Detects presence of `@module` attribute.
#[must_use]
pub fn extract_module_attribute(attrs: &[Attribute]) -> (bool, Vec<AttributeError>) {
    let mut is_module = false;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("module") {
            continue;
        }

        if is_module {
            errors.push(AttributeError::new(
                "duplicate `@module` attribute",
                attr.span,
            ));
        }
        is_module = true;

        if !attr.arguments.is_empty() {
            errors.push(AttributeError::new(
                "`@module` does not accept arguments",
                attr.span,
            ));
        }
    }

    (is_module, errors)
}

/// Extracts dependency-injection metadata from `@inject` attributes.
#[must_use]
pub fn extract_inject_attribute(
    attrs: &[Attribute],
) -> (Option<DiInjectAttr>, Vec<AttributeError>) {
    let mut inject: Option<DiInjectAttr> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("inject") {
            continue;
        }

        if inject.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@inject` attribute",
                attr.span,
            ));
            continue;
        }

        let mut lifetime: Option<DiLifetime> = None;
        let mut named: Option<String> = None;
        let mut optional = false;

        for argument in &attr.arguments {
            let key = argument
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase());
            let trimmed = trim_quotes(argument.value.trim());
            if trimmed.is_empty() {
                errors.push(AttributeError::new(
                    "`@inject` attribute argument must not be empty",
                    argument.span.or(attr.span),
                ));
                continue;
            }

            match key.as_deref() {
                Some("lifetime") => match parse_di_lifetime_value(trimmed) {
                    Some(value) => lifetime = Some(value),
                    None => errors.push(AttributeError::new(
                        format!("unsupported DI lifetime `{trimmed}`"),
                        argument.span.or(attr.span),
                    )),
                },
                Some("named") => {
                    named = Some(trimmed.to_string());
                }
                Some("optional") => match parse_bool_value(trimmed) {
                    Some(flag) => optional = flag,
                    None => errors.push(AttributeError::new(
                        format!("expected `true` or `false` for `optional`, found `{trimmed}`"),
                        argument.span.or(attr.span),
                    )),
                },
                Some(other) => {
                    errors.push(AttributeError::new(
                        format!("unsupported named argument `{other}` for `@inject`"),
                        argument.span.or(attr.span),
                    ));
                }
                None => {
                    if lifetime.is_none() {
                        match parse_di_lifetime_value(trimmed) {
                            Some(value) => lifetime = Some(value),
                            None => named = Some(trimmed.to_string()),
                        }
                    } else if named.is_none() {
                        named = Some(trimmed.to_string());
                    } else {
                        errors.push(AttributeError::new(
                            "`@inject` accepts at most one unnamed argument",
                            argument.span.or(attr.span),
                        ));
                    }
                }
            }
        }

        inject = Some(DiInjectAttr::new(lifetime, named, optional));
    }

    (inject, errors)
}

/// Extracts the conditional symbol from `@conditional("SYMBOL")` attributes.
#[must_use]
pub fn extract_conditional_attribute(
    attrs: &[Attribute],
) -> (Option<ConditionalAttribute>, Vec<AttributeError>) {
    let mut conditional: Option<ConditionalAttribute> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr.name.eq_ignore_ascii_case("conditional") {
            continue;
        }

        if conditional.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@conditional` attribute",
                attr.span,
            ));
            continue;
        }

        if attr.arguments.is_empty() {
            errors.push(AttributeError::new(
                "`@conditional` requires a symbol string argument",
                attr.span,
            ));
            continue;
        }

        if attr.arguments.len() > 1 {
            errors.push(AttributeError::new(
                "`@conditional` accepts exactly one argument",
                attr.arguments.get(1).and_then(|arg| arg.span).or(attr.span),
            ));
        }

        let arg = &attr.arguments[0];
        if arg.name.is_some() {
            errors.push(AttributeError::new(
                "`@conditional` does not support named arguments",
                arg.span.or(attr.span),
            ));
        }
        let value = trim_quotes(arg.value.trim());
        if value.is_empty() {
            errors.push(AttributeError::new(
                "`@conditional` symbol must not be empty",
                arg.span.or(attr.span),
            ));
            continue;
        }
        conditional = Some(ConditionalAttribute {
            symbol: value.to_ascii_uppercase(),
            span: arg.span.or(attr.span),
        });
    }

    (conditional, errors)
}

/// Detects presence of `@no_std` attribute and returns its span if present.
#[must_use]
pub fn extract_no_std(attrs: &[Attribute]) -> (Option<Span>, Vec<AttributeError>) {
    let mut span: Option<Span> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !(attr.name.eq_ignore_ascii_case("no_std") || attr.name.eq_ignore_ascii_case("nostd")) {
            continue;
        }

        if !attr.arguments.is_empty() {
            errors.push(AttributeError::new(
                "`@no_std` does not accept arguments",
                attr.span,
            ));
            continue;
        }

        if span.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@no_std` attribute",
                attr.span,
            ));
        } else {
            span = attr.span;
        }
    }

    (span, errors)
}

/// Detects presence of `@suppress_startup_descriptor` attribute.
#[must_use]
pub fn extract_suppress_startup_descriptor(
    attrs: &[Attribute],
) -> (Option<Span>, Vec<AttributeError>) {
    let mut span: Option<Span> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !attr
            .name
            .eq_ignore_ascii_case("suppress_startup_descriptor")
        {
            continue;
        }

        if !attr.arguments.is_empty() {
            errors.push(AttributeError::new(
                "`@suppress_startup_descriptor` does not accept arguments",
                attr.span,
            ));
            continue;
        }

        if span.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@suppress_startup_descriptor` attribute",
                attr.span,
            ));
        } else {
            span = attr.span;
        }
    }

    (span, errors)
}

/// Attribute describing the requested global allocator.
#[derive(Debug, Clone)]
pub struct GlobalAllocatorAttr {
    pub target: Option<String>,
    pub span: Option<Span>,
}

/// Extracts `@global_allocator` declarations.
#[must_use]
pub fn extract_global_allocator(
    attrs: &[Attribute],
) -> (Option<GlobalAllocatorAttr>, Vec<AttributeError>) {
    let mut result: Option<GlobalAllocatorAttr> = None;
    let mut errors = Vec::new();

    for attr in attrs {
        if !(attr.name.eq_ignore_ascii_case("global_allocator")
            || attr.name.eq_ignore_ascii_case("globalallocator"))
        {
            continue;
        }

        if result.is_some() {
            errors.push(AttributeError::new(
                "duplicate `@global_allocator` attribute",
                attr.span,
            ));
            continue;
        }

        if attr.arguments.len() > 1 {
            errors.push(AttributeError::new(
                "`@global_allocator` accepts at most one argument",
                attr.span,
            ));
            continue;
        }

        let mut attr_error = false;
        let target = if let Some(argument) = attr.arguments.first() {
            if let Some(name) = &argument.name {
                if !name.eq_ignore_ascii_case("type")
                    && !name.eq_ignore_ascii_case("target")
                    && !name.is_empty()
                {
                    errors.push(AttributeError::new(
                        format!("unsupported named argument `{name}` for `@global_allocator`"),
                        argument.span.or(attr.span),
                    ));
                    attr_error = true;
                }
            }

            let trimmed = trim_quotes(argument.value.trim());
            if !attr_error && trimmed.is_empty() {
                errors.push(AttributeError::new(
                    "`@global_allocator` argument must not be empty",
                    argument.span.or(attr.span),
                ));
                attr_error = true;
            }

            if attr_error {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            None
        };

        if attr_error {
            continue;
        }

        result = Some(GlobalAllocatorAttr {
            target,
            span: attr
                .arguments
                .first()
                .and_then(|arg| arg.span)
                .or(attr.span),
        });
    }

    (result, errors)
}

fn parse_repr_attribute(
    attr: &Attribute,
    hints: &mut LayoutHints,
    errors: &mut Vec<AttributeError>,
) {
    if attr.arguments.is_empty() {
        errors.push(AttributeError::new(
            "`@repr` requires one or more arguments",
            attr.span,
        ));
        return;
    }

    for argument in &attr.arguments {
        match interpret_repr_argument(argument) {
            Ok(ReprItem::C) => hints.repr_c = true,
            Ok(ReprItem::Packed { value }) => {
                if let Some(existing) = hints.packing {
                    if existing.value != value {
                        errors.push(AttributeError::new(
                            "conflicting `@repr(packed(...))` values",
                            argument.span.or(existing.span).or(attr.span),
                        ));
                    }
                    continue;
                }
                hints.packing = Some(PackingHint {
                    value,
                    span: argument.span.or(attr.span),
                });
            }
            Err(err) => errors.push(err),
        }
    }
}

fn parse_align_attribute(
    attr: &Attribute,
    hints: &mut LayoutHints,
    errors: &mut Vec<AttributeError>,
) {
    if attr.arguments.is_empty() {
        errors.push(AttributeError::new(
            "`@align` requires an integer argument",
            attr.span,
        ));
        return;
    }
    if attr.arguments.len() > 1 {
        errors.push(AttributeError::new(
            "`@align` accepts exactly one argument",
            attr.span,
        ));
        return;
    }

    let argument = &attr.arguments[0];
    match parse_integer(argument) {
        Ok(value) => {
            if value == 0 {
                errors.push(AttributeError::new(
                    "`@align` requires a non-zero power-of-two value",
                    argument.span.or(attr.span),
                ));
                return;
            }
            if !is_power_of_two(value) {
                errors.push(AttributeError::new(
                    "`@align` requires a power-of-two integer",
                    argument.span.or(attr.span),
                ));
                return;
            }
            let value_u32 = match u32::try_from(value) {
                Ok(v) => v,
                Err(_) => {
                    errors.push(AttributeError::new(
                        "`@align` value exceeds supported range",
                        argument.span.or(attr.span),
                    ));
                    return;
                }
            };
            if let Some(existing) = hints.align {
                if existing.value != value_u32 {
                    errors.push(AttributeError::new(
                        "multiple `@align` attributes with differing values",
                        argument.span.or(existing.span).or(attr.span),
                    ));
                }
                return;
            }
            hints.align = Some(AlignHint {
                value: value_u32,
                span: argument.span.or(attr.span),
            });
        }
        Err(err) => errors.push(err),
    }
}

enum ReprItem {
    C,
    Packed { value: Option<u32> },
}

fn interpret_repr_argument(arg: &AttributeArgument) -> Result<ReprItem, AttributeError> {
    if let Some(name) = &arg.name {
        if name.eq_ignore_ascii_case("packed") {
            let trimmed = arg.value.trim();
            if trimmed.is_empty() {
                return Ok(ReprItem::Packed { value: None });
            }
            return parse_packed_value(trimmed, arg.span);
        }
        if name.eq_ignore_ascii_case("c") {
            return Ok(ReprItem::C);
        }
        return Err(AttributeError::new(
            format!("unsupported `@repr` argument `{name}`"),
            arg.span,
        ));
    }

    let trimmed = arg.value.trim();
    if trimmed.is_empty() {
        return Err(AttributeError::new("empty `@repr` argument", arg.span));
    }

    let normalized = trim_quotes(trimmed);
    if normalized.eq_ignore_ascii_case("c") {
        return Ok(ReprItem::C);
    }
    if normalized.eq_ignore_ascii_case("packed") {
        return Ok(ReprItem::Packed { value: None });
    }
    if normalized.to_ascii_lowercase().starts_with("packed") {
        let remainder = &normalized["packed".len()..];
        return parse_packed_suffix(remainder, arg.span);
    }

    Err(AttributeError::new(
        format!("unsupported `@repr` argument `{normalized}`"),
        arg.span,
    ))
}

fn parse_packed_value(text: &str, span: Option<Span>) -> Result<ReprItem, AttributeError> {
    if text.eq_ignore_ascii_case("true") || text.is_empty() {
        return Ok(ReprItem::Packed { value: None });
    }
    let value = parse_integer_value(text, span)?;
    interpret_packed_value(value, span)
}

fn parse_packed_suffix(remainder: &str, span: Option<Span>) -> Result<ReprItem, AttributeError> {
    let trimmed = remainder.trim();
    if trimmed.is_empty() {
        return Ok(ReprItem::Packed { value: None });
    }
    if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
        return Err(AttributeError::new(
            "expected parentheses for `@repr(packed(...))`",
            span,
        ));
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Ok(ReprItem::Packed { value: None });
    }
    let value = parse_integer_value(inner, span)?;
    interpret_packed_value(value, span)
}

fn parse_integer(argument: &AttributeArgument) -> Result<u64, AttributeError> {
    parse_integer_value(argument.value.trim(), argument.span)
}

fn parse_integer_value(text: &str, span: Option<Span>) -> Result<u64, AttributeError> {
    let cleaned = text.replace('_', "");
    if cleaned.is_empty() {
        return Err(AttributeError::new("expected integer literal", span));
    }

    let (base, digits) = if let Some(rest) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (16, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (2, rest)
    } else if let Some(rest) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (8, rest)
    } else {
        (10, cleaned.as_str())
    };

    if digits.is_empty() {
        return Err(AttributeError::new(
            "expected digits in integer literal",
            span,
        ));
    }

    u64::from_str_radix(digits, base)
        .map_err(|_| AttributeError::new("failed to parse integer literal in attribute", span))
}

fn interpret_packed_value(value: u64, span: Option<Span>) -> Result<ReprItem, AttributeError> {
    if value == 0 {
        return Err(AttributeError::new(
            "`@repr(packed(N))` requires a non-zero value",
            span,
        ));
    }
    if !is_power_of_two(value) {
        return Err(AttributeError::new(
            "`@repr(packed(N))` requires `N` to be a power of two",
            span,
        ));
    }
    let value_u32 = u32::try_from(value).map_err(|_| {
        AttributeError::new("`@repr(packed(N))` value exceeds supported range", span)
    })?;
    Ok(ReprItem::Packed {
        value: Some(value_u32),
    })
}

fn is_power_of_two(value: u64) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

fn trim_quotes(text: &str) -> &str {
    if (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('\'') && text.ends_with('\''))
    {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::AttributeKind;

    fn attr(name: &str) -> Attribute {
        Attribute::new(
            name,
            Vec::new(),
            Some(Span::new(0, name.len())),
            Some(format!("@{name}")),
            AttributeKind::Builtin,
        )
    }

    #[test]
    fn collects_optimization_hints() {
        let attrs = vec![attr("hot"), attr("always_inline")];
        let (hints, errors) = collect_optimization_hints(&attrs);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        assert!(hints.hot);
        assert!(hints.always_inline);
        assert!(!hints.cold);
        assert!(!hints.never_inline);
    }

    #[test]
    fn rejects_conflicting_hot_and_cold() {
        let attrs = vec![attr("hot"), attr("cold")];
        let (_hints, errors) = collect_optimization_hints(&attrs);
        assert!(
            errors
                .iter()
                .any(|err| err.message.contains("cannot combine `@hot` and `@cold`")),
            "expected conflict diagnostic, got {errors:?}"
        );
    }

    #[test]
    fn rejects_duplicate_and_conflicting_inline_hints() {
        let attrs = vec![
            attr("always_inline"),
            attr("never_inline"),
            attr("always_inline"),
        ];
        let (_hints, errors) = collect_optimization_hints(&attrs);
        assert!(
            errors.iter().any(|err| err
                .message
                .contains("cannot combine `@always_inline` and `@never_inline`")),
            "expected inline conflict diagnostic"
        );
        assert!(
            errors
                .iter()
                .any(|err| err.message.contains("duplicate `@always_inline`")),
            "expected duplicate always_inline diagnostic"
        );
    }
}
