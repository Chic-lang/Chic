use super::registry::{AttributeInput, AttributeOutput, AttributeTarget};
use super::{MacroRegistry, handlers};
use crate::frontend::ast::expressions::StatementKind;
use crate::frontend::ast::{
    ExtensionMember, FunctionDecl, Item, MemberDispatch, Module, Signature, TypeExpr, Visibility,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Token, TokenKind};
use crate::frontend::macro_expander::engine::expand_module;
use crate::frontend::macro_expander::handlers::macro_attribute;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static TOKEN_SINK: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static HYGIENE_SINK: OnceLock<Mutex<Vec<u64>>> = OnceLock::new();

static LOOPING_ATTRIBUTE_RENAMES: AtomicUsize = AtomicUsize::new(0);

fn token_sink() -> &'static Mutex<Vec<String>> {
    TOKEN_SINK.get_or_init(|| Mutex::new(Vec::new()))
}

fn hygiene_sink() -> &'static Mutex<Vec<u64>> {
    HYGIENE_SINK.get_or_init(|| Mutex::new(Vec::new()))
}

fn recording_attribute(input: AttributeInput<'_>) -> AttributeOutput {
    token_sink()
        .lock()
        .unwrap()
        .extend(input.invocation.tokens.iter().map(|tok| tok.lexeme.clone()));
    hygiene_sink()
        .lock()
        .unwrap()
        .push(input.invocation.hygiene.value());
    AttributeOutput::empty()
}

fn module_with_struct(name: &str, fields: &[(&str, &str)]) -> Module {
    let mut module = Module::new(Some("Geometry".into()));
    let strct = handlers::struct_with_fields(name, fields);
    module.push_item(Item::Struct(strct));
    module
}

#[test]
fn derive_equatable_on_struct_generates_extension() {
    let mut module = module_with_struct("Point", &[("X", "int"), ("Y", "int")]);
    if let Item::Struct(ref mut point) = module.items[0] {
        point
            .attributes
            .push(macro_attribute("derive", &["Equatable"]));
    }

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(module.items.len(), 2);
}

#[test]
fn derive_hashable_on_struct_generates_extension() {
    let mut module = module_with_struct("Point", &[("X", "int"), ("Y", "int"), ("Z", "int")]);
    if let Item::Struct(ref mut point) = module.items[0] {
        point
            .attributes
            .push(macro_attribute("derive", &["Hashable"]));
    }

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(module.items.len(), 2);
}

#[test]
fn derive_clone_on_struct_generates_impl() {
    let mut module = module_with_struct("Point", &[("X", "int"), ("Y", "int")]);
    if let Item::Struct(ref mut point) = module.items[0] {
        point.attributes.push(macro_attribute("derive", &["Clone"]));
    }

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        module
            .items
            .iter()
            .any(|item| matches!(item, Item::Impl(_))),
        "expected Clone impl to be generated"
    );
}

#[test]
fn unknown_derive_reports_error() {
    let mut module = module_with_struct("Point", &[("X", "int")]);
    if let Item::Struct(ref mut point) = module.items[0] {
        point
            .attributes
            .push(macro_attribute("derive", &["Comparable"]));
    }
    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown derive macro")),
        "expected unknown derive diagnostic"
    );
}

#[test]
fn memoize_generates_cache_for_parameterless_function() {
    let mut module = Module::new(None);
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Value".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(handlers::make_return_statement_block("1")),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: vec![macro_attribute("memoize", &[])],
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

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        module.items.len() >= 3,
        "memoize should inject cache statics and helper function"
    );
    let function = module
        .items
        .iter()
        .find_map(|item| match item {
            Item::Function(func) if func.name == "Value" => Some(func),
            _ => None,
        })
        .expect("expected original function retained");
    let body = function
        .body
        .as_ref()
        .expect("memoized function has a body");
    let first = body
        .statements
        .first()
        .expect("memoized body contains statements");
    assert!(
        matches!(first.kind, StatementKind::If(_)),
        "first statement should check cache presence"
    );
}

#[test]
fn duplicate_derive_uses_cache() {
    let mut module = module_with_struct("Point", &[("X", "int")]);
    if let Item::Struct(ref mut point) = module.items[0] {
        point
            .attributes
            .push(macro_attribute("derive", &["Equatable", "Equatable"]));
    }
    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert_eq!(result.cache_misses, 1);
    assert!(
        result.cache_hits >= 1,
        "expected at least one cache hit, got {}",
        result.cache_hits
    );
}

#[test]
fn record_struct_generates_equality_and_hash_extensions() {
    let parsed = crate::frontend::parser::parse_module("public record struct Point(int X, int Y);")
        .expect("parse record module");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut module = parsed.module;

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected expansion diagnostics: {:?}",
        result.diagnostics
    );

    let mut method_names = Vec::new();
    for item in module.items.iter() {
        if let Item::Extension(ext) = item {
            for member in &ext.members {
                let ExtensionMember::Method(method) = member;
                method_names.push(method.function.name.clone());
            }
        }
    }
    assert!(
        method_names.contains(&"op_Equality".to_string())
            && method_names.contains(&"op_Inequality".to_string())
            && method_names.contains(&"GetHashCode".to_string()),
        "expected equality and hash methods to be generated for records, got {method_names:?}"
    );
}

#[test]
fn generic_record_struct_skips_auto_derives() {
    let parsed = crate::frontend::parser::parse_module("public record struct Box<T>(T Value);")
        .expect("parse record module");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let mut module = parsed.module;

    let registry = MacroRegistry::with_builtins();
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.iter().any(|diag| diag
            .message
            .contains("auto-generated equality/hash does not yet support generic parameters")),
        "expected generic record diagnostic, found {:?}",
        result.diagnostics
    );
    assert_eq!(
        module
            .items
            .iter()
            .filter(|item| matches!(item, Item::Extension(_)))
            .count(),
        0,
        "generic record should not receive auto-generated extensions"
    );
}

#[test]
fn generated_items_receive_follow_up_expansion_pass() {
    let mut module = Module::new(None);
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Run".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(handlers::make_return_statement_block("0")),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: vec![macro_attribute("chain", &[])],
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

    let mut registry = MacroRegistry::with_builtins();
    registry.register_attribute("chain", chain_attribute);
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        module
            .items
            .iter()
            .any(|item| matches!(item, Item::Extension(_))),
        "expected derive expansion on generated struct"
    );
}

#[test]
fn runaway_macro_expansion_reports_error() {
    let mut module = Module::new(None);
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Loop".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(handlers::make_return_statement_block("0")),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: vec![macro_attribute("loop", &[])],
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
    LOOPING_ATTRIBUTE_RENAMES.store(0, Ordering::SeqCst);

    let mut registry = MacroRegistry::with_builtins();
    registry.register_attribute("loop", looping_attribute);
    let result = expand_module(&mut module, &registry);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("macro expansion exceeded")),
        "expected runaway diagnostic, found {:?}",
        result.diagnostics
    );
}

#[test]
fn attribute_invocation_exposes_tokens_and_hygiene() {
    let mut module = Module::new(None);
    let mut first = macro_attribute("memoize", &["seed"]);
    first.span = Some(Span::new(0, 12));
    first.macro_metadata.tokens = vec![
        Token {
            kind: TokenKind::Punctuation('@'),
            lexeme: "@".into(),
            span: Span::new(0, 1),
        },
        Token {
            kind: TokenKind::Identifier,
            lexeme: "memoize".into(),
            span: Span::new(1, 9),
        },
        Token {
            kind: TokenKind::Identifier,
            lexeme: "seed".into(),
            span: Span::new(10, 14),
        },
    ];
    let mut second = macro_attribute("memoize", &["other"]);
    second.span = Some(Span::new(15, 30));
    second.macro_metadata.tokens = vec![
        Token {
            kind: TokenKind::Punctuation('@'),
            lexeme: "@".into(),
            span: Span::new(15, 16),
        },
        Token {
            kind: TokenKind::Identifier,
            lexeme: "memoize".into(),
            span: Span::new(16, 24),
        },
        Token {
            kind: TokenKind::Identifier,
            lexeme: "other".into(),
            span: Span::new(25, 30),
        },
    ];
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Value".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(handlers::make_return_statement_block("1")),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: vec![first, second],
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

    let mut registry = MacroRegistry::with_builtins();
    token_sink().lock().unwrap().clear();
    hygiene_sink().lock().unwrap().clear();
    registry.register_attribute("memoize", recording_attribute);
    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let tokens = token_sink().lock().unwrap();
    assert!(
        tokens.iter().any(|lex| lex == "memoize"),
        "expected to capture macro name tokens, found {tokens:?}"
    );
    assert!(
        tokens.iter().any(|lex| lex == "seed") && tokens.iter().any(|lex| lex == "other"),
        "expected to capture macro argument tokens, found {tokens:?}"
    );

    let hygiene = hygiene_sink().lock().unwrap();
    assert_eq!(
        hygiene.len(),
        2,
        "expected hygiene ids for each invocation, found {hygiene:?}"
    );
    assert_ne!(
        hygiene[0], hygiene[1],
        "distinct invocations should have unique hygiene identifiers"
    );
}

#[test]
fn generated_items_are_stamped_with_attribute_span() {
    let mut module = Module::new(None);
    let mut generator_attr = macro_attribute("generate", &[]);
    let origin = Span::new(40, 50);
    generator_attr.span = Some(origin);
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Source".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(handlers::make_return_statement_block("0")),
        is_async: false,
        is_constexpr: false,
        doc: None,
        modifiers: Vec::new(),
        is_unsafe: false,
        attributes: vec![generator_attr],
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

    let mut registry = MacroRegistry::with_builtins();
    registry.register_attribute("generate", |_input| {
        let generated = handlers::struct_with_fields("Payload", &[("Value", "int")]);
        let function_body = handlers::make_return_statement_block("1");
        AttributeOutput {
            new_items: vec![
                Item::Struct(generated),
                Item::Function(FunctionDecl {
                    visibility: Visibility::Public,
                    name: "Generated".into(),
                    name_span: None,
                    signature: Signature {
                        parameters: Vec::new(),
                        return_type: TypeExpr::simple("int"),
                        lends_to_return: None,
                        variadic: false,
                        throws: None,
                    },
                    body: Some(function_body),
                    is_async: false,
                    is_constexpr: false,
                    doc: None,
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
                }),
            ],
            diagnostics: Vec::new(),
        }
    });

    let result = expand_module(&mut module, &registry);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let generated_fn = module
        .items
        .iter()
        .find_map(|item| match item {
            Item::Function(func) if func.name == "Generated" => Some(func),
            _ => None,
        })
        .expect("expected generated function");

    let body = generated_fn
        .body
        .as_ref()
        .expect("expected generated body to be present");
    assert_eq!(
        body.span,
        Some(origin),
        "expected generated block span to inherit attribute span"
    );
    let statement = body
        .statements
        .first()
        .expect("expected generated statement");
    assert_eq!(
        statement.span,
        Some(origin),
        "expected generated statement span to inherit attribute span"
    );
    match &statement.kind {
        StatementKind::Expression(expr) => assert_eq!(
            expr.span,
            Some(origin),
            "expected generated expression span to inherit attribute span"
        ),
        StatementKind::Throw {
            expression: Some(expr),
        } => assert_eq!(
            expr.span,
            Some(origin),
            "expected generated expression span to inherit attribute span"
        ),
        StatementKind::Return {
            expression: Some(expr),
        } => assert_eq!(
            expr.span,
            Some(origin),
            "expected generated expression span to inherit attribute span"
        ),
        _ => {}
    }
}

fn chain_attribute(_input: AttributeInput<'_>) -> AttributeOutput {
    let mut generated_struct = handlers::struct_with_fields("Generated", &[("Value", "int")]);
    generated_struct
        .attributes
        .push(macro_attribute("derive", &["Equatable"]));
    AttributeOutput {
        new_items: vec![Item::Struct(generated_struct)],
        diagnostics: Vec::new(),
    }
}

fn looping_attribute(input: AttributeInput<'_>) -> AttributeOutput {
    let function = match input.target {
        AttributeTarget::Function(function) => function,
        AttributeTarget::Method { function, .. } => function,
    };
    let suffix = LOOPING_ATTRIBUTE_RENAMES.fetch_add(1, Ordering::Relaxed) + 1;
    let mut generated = function.clone();
    generated.name = format!("LoopGenerated{suffix}");
    generated.attributes.clear();
    let mut attr = macro_attribute("loop", &[]);
    attr.raw = Some(format!("loop({suffix})"));
    function.attributes.push(attr);
    AttributeOutput {
        new_items: vec![Item::Function(generated)],
        diagnostics: Vec::new(),
    }
}
