use super::common::RequireExt;
use super::*;
use crate::mir::ConstEvalContext;
use crate::mir::builder::lower_module;
use crate::mir::builder::symbol_index::SymbolIndex;

fn get_field<'a>(fields: &'a [(String, ConstValue)], name: &str) -> &'a ConstValue {
    fields
        .iter()
        .find_map(|(field_name, value)| (field_name == name).then_some(value))
        .unwrap_or_else(|| panic!("missing field `{name}` in {fields:?}"))
}

fn collect_descriptor_list<'a>(value: &'a ConstValue) -> Vec<&'a ConstValue> {
    let mut items = Vec::new();
    let mut current = value;
    loop {
        match current {
            ConstValue::Struct { fields, .. } => {
                let is_empty = matches!(get_field(fields, "IsEmpty"), ConstValue::Bool(true));
                if is_empty {
                    break;
                }
                let head = get_field(fields, "Head");
                items.push(head);
                let tail = get_field(fields, "Tail");
                match tail {
                    ConstValue::Struct { .. } => {
                        current = tail;
                    }
                    ConstValue::Null => break,
                    other => panic!("unexpected descriptor list tail value {other:?}"),
                }
            }
            ConstValue::Null => break,
            other => panic!("expected descriptor list, found {other:?}"),
        }
    }
    items
}

#[test]
fn quote_const_evaluates_basic_metadata() {
    let source = r#"
namespace Sample;

public const Std.Meta.Quote Macro = quote(foo + bar);
"#;

    let parsed = parse_module(source).require("parse quote module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = lowering.module.type_layouts.clone();
    let mut context = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);

    let value = context
        .evaluate_const("Sample::Macro", None)
        .expect("quote constant should evaluate")
        .value;
    let ConstValue::Struct { type_name, fields } = value else {
        panic!("expected quote struct, found {value:?}");
    };
    assert_eq!(type_name, "Std::Meta::Quote");

    match get_field(&fields, "Source") {
        ConstValue::RawStr(text) => assert_eq!(text, "foo + bar"),
        other => panic!("unexpected source value {other:?}"),
    }
    match get_field(&fields, "Sanitized") {
        ConstValue::RawStr(text) => assert_eq!(text, "foo + bar"),
        other => panic!("unexpected sanitized value {other:?}"),
    }

    let captures = collect_descriptor_list(get_field(&fields, "Captures"));
    let capture_values = captures
        .iter()
        .map(|value| match value {
            ConstValue::RawStr(text) => text.clone(),
            other => panic!("unexpected capture value {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(capture_values, vec!["foo".to_string(), "bar".to_string()]);

    let interpolations = collect_descriptor_list(get_field(&fields, "Interpolations"));
    assert!(
        interpolations.is_empty(),
        "expected no interpolations, found {interpolations:?}"
    );

    let root = get_field(&fields, "Root");
    assert_node_kind(root, "Binary");
    let children = collect_descriptor_list(get_field(
        match root {
            ConstValue::Struct { fields, .. } => fields,
            other => panic!("expected quote node struct, found {other:?}"),
        },
        "Children",
    ));
    assert_eq!(children.len(), 2, "binary node should have two children");
    assert_node_value(children[0], "Identifier", Some("foo"));
    assert_node_value(children[1], "Identifier", Some("bar"));
}

#[test]
fn quote_const_includes_interpolations() {
    let source = r#"
namespace Sample;

public const Std.Meta.Quote Macro = quote(x + ${quote(y)} + z);
"#;

    let parsed = parse_module(source).require("parse quote module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = lowering.module.type_layouts.clone();
    let mut context = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
    let value = context
        .evaluate_const("Sample::Macro", None)
        .expect("quote constant should evaluate")
        .value;

    let ConstValue::Struct { fields, .. } = value else {
        panic!("expected quote struct");
    };
    let interpolations = collect_descriptor_list(get_field(&fields, "Interpolations"));
    assert_eq!(interpolations.len(), 1, "expected single interpolation");
    let entry = match interpolations[0] {
        ConstValue::Struct { fields, .. } => fields,
        other => panic!("unexpected interpolation value {other:?}"),
    };
    match get_field(entry, "Placeholder") {
        ConstValue::RawStr(text) => assert_eq!(text, "__chic_quote_slot0"),
        other => panic!("unexpected placeholder value {other:?}"),
    }
    match get_field(entry, "Value") {
        ConstValue::Struct { type_name, fields } => {
            assert_eq!(type_name, "Std::Meta::Quote");
            match get_field(fields, "Source") {
                ConstValue::RawStr(text) => assert_eq!(text, "y"),
                other => panic!("unexpected nested quote source {other:?}"),
            }
        }
        other => panic!("expected nested quote struct, found {other:?}"),
    }
}

#[test]
fn quote_interpolation_requires_quote_values() {
    let source = r#"
namespace Sample;

public const Std.Meta.Quote Macro = quote(foo + ${1} + bar);
"#;

    let parsed = parse_module(source).require("parse quote module");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {:?}",
        lowering.diagnostics
    );
    let diagnostic = &lowering.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("must evaluate to `Std.Meta.Quote`"),
        "unexpected diagnostic: {}",
        diagnostic.message
    );
}

fn assert_node_kind(node: &ConstValue, expected: &str) {
    let ConstValue::Struct { fields, .. } = node else {
        panic!("expected quote node struct, found {node:?}");
    };
    match get_field(fields, "Kind") {
        ConstValue::Enum { variant, .. } => assert_eq!(variant, expected),
        other => panic!("unexpected node kind {other:?}"),
    }
}

fn assert_node_value(node: &ConstValue, expected_kind: &str, expected_value: Option<&str>) {
    let ConstValue::Struct { fields, .. } = node else {
        panic!("expected quote node struct, found {node:?}");
    };
    match get_field(fields, "Kind") {
        ConstValue::Enum { variant, .. } => assert_eq!(variant, expected_kind),
        other => panic!("unexpected node kind {other:?}"),
    };
    match (get_field(fields, "Value"), expected_value) {
        (ConstValue::RawStr(value), Some(expected)) => assert_eq!(value, expected),
        (ConstValue::Null, None) => {}
        (other, expectation) => panic!("unexpected node value {other:?}, expected {expectation:?}"),
    }
}
