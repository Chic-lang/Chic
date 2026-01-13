use super::common::RequireExt;
use super::*;
use crate::mir::builder::SymbolIndex;

fn get_field<'a>(fields: &'a [(String, ConstValue)], name: &str) -> &'a ConstValue {
    fields
        .iter()
        .find_map(|(field_name, value)| {
            if field_name == name {
                Some(value)
            } else {
                None
            }
        })
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

fn assert_member_field(member: &ConstValue, expected_name: &str) {
    let ConstValue::Struct { fields, .. } = member else {
        panic!("expected member descriptor struct, found {member:?}");
    };
    let fields = fields.as_slice();
    match get_field(fields, "Name") {
        ConstValue::RawStr(value) => assert_eq!(value, expected_name),
        ConstValue::Str { value, .. } => assert_eq!(value, expected_name),
        other => panic!("unexpected member name value {other:?}"),
    }
    match get_field(fields, "Kind") {
        ConstValue::Enum { variant, .. } => assert_eq!(variant, "Field"),
        other => panic!("unexpected member kind {other:?}"),
    }
}

#[test]
fn reflect_returns_descriptor_for_public_struct() {
    let source = r#"
namespace Sample;

public struct Widget
{
    public int Id;
    public string Name;
}

public const Std.Meta.TypeDescriptor Descriptor = Std.Meta.Reflection.reflect<Widget>();
"#;

    let parsed = parse_module(source).require("parse reflect module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = lowering.module.type_layouts.clone();
    let mut context = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
    let descriptor_value = context
        .evaluate_const("Sample::Descriptor", None)
        .expect("expected descriptor constant")
        .value;

    let ConstValue::Struct { fields, .. } = descriptor_value else {
        panic!("descriptor constant should be a struct, found {descriptor_value:?}");
    };
    let fields = fields.as_slice();

    match get_field(fields, "Namespace") {
        ConstValue::RawStr(value) => assert_eq!(value, "Sample"),
        ConstValue::Str { value, .. } => assert_eq!(value, "Sample"),
        other => panic!("unexpected namespace value {other:?}"),
    }

    match get_field(fields, "Name") {
        ConstValue::RawStr(value) => assert_eq!(value, "Sample::Widget"),
        ConstValue::Str { value, .. } => assert_eq!(value, "Sample::Widget"),
        other => panic!("unexpected descriptor name value {other:?}"),
    }

    match get_field(fields, "FullName") {
        ConstValue::RawStr(value) => assert_eq!(value, "Sample::Widget"),
        ConstValue::Str { value, .. } => assert_eq!(value, "Sample::Widget"),
        other => panic!("unexpected full name value {other:?}"),
    }

    match get_field(fields, "Kind") {
        ConstValue::Enum {
            type_name, variant, ..
        } => {
            assert_eq!(type_name, "Std::Meta::TypeKind");
            assert_eq!(variant, "Struct");
        }
        other => panic!("unexpected descriptor kind {other:?}"),
    }

    match get_field(fields, "Visibility") {
        ConstValue::Enum {
            type_name, variant, ..
        } => {
            assert_eq!(type_name, "Std::Meta::VisibilityDescriptor");
            assert_eq!(variant, "Public");
        }
        other => panic!("unexpected descriptor visibility {other:?}"),
    }

    match get_field(fields, "IsGeneric") {
        ConstValue::Bool(value) => assert!(!value, "Widget should not be generic"),
        other => panic!("unexpected IsGeneric value {other:?}"),
    }

    let members = collect_descriptor_list(get_field(fields, "Members"));
    assert!(
        members.len() >= 2,
        "expected at least two members for Widget, found {}",
        members.len()
    );
    assert_member_field(members[0], "Id");
    assert_member_field(members[1], "Name");
}

#[test]
fn reflect_reports_error_for_non_public_type() {
    let source = r#"
namespace Sample;

internal struct Hidden
{
    public int Value;
}

public const Std.Meta.TypeDescriptor Descriptor = Std.Meta.Reflection.reflect<Hidden>();
"#;

    let parsed = parse_module(source).require("parse reflect module");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let diagnostic = &lowering.diagnostics[0];
    assert!(
        diagnostic.message.contains("public types"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
}

#[test]
fn reflect_rejects_value_arguments() {
    let source = r#"
namespace Sample;

public struct Widget
{
    public int Value;
}

public const Std.Meta.TypeDescriptor Descriptor = Std.Meta.Reflection.reflect<Widget>(0);
"#;

    let parsed = parse_module(source).require("parse reflect module");
    let lowering = lower_module(&parsed.module);
    assert_eq!(
        lowering.diagnostics.len(),
        1,
        "expected single diagnostic, found {0:?}",
        lowering.diagnostics
    );
    let diagnostic = &lowering.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("does not accept value arguments"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
}

#[test]
fn reflect_reports_layout_for_structs() {
    let source = r#"
namespace Sample;

public struct Pair
{
    public int Left;
    public int Right;
}

public const Std.Meta.TypeDescriptor Descriptor = Std.Meta.Reflection.reflect<Pair>();
"#;

    let parsed = parse_module(source).require("parse reflect module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = lowering.module.type_layouts.clone();
    let mut context = ConstEvalContext::new(&mut symbol_index, &mut layouts, None);
    let descriptor_value = context
        .evaluate_const("Sample::Descriptor", None)
        .expect("expected descriptor constant")
        .value;

    let ConstValue::Struct { fields, .. } = descriptor_value else {
        panic!("descriptor constant should be a struct, found {descriptor_value:?}");
    };
    let type_layout = get_field(fields.as_slice(), "Layout");
    let layout_fields = match type_layout {
        ConstValue::Struct { fields, .. } => fields,
        ConstValue::Null => panic!("TypeLayout should be populated"),
        other => panic!("unexpected TypeLayout value {other:?}"),
    };

    match get_field(layout_fields, "Size") {
        ConstValue::UInt(value) => assert_eq!(*value, 8),
        other => panic!("unexpected layout size {other:?}"),
    }
    match get_field(layout_fields, "Align") {
        ConstValue::UInt(value) => assert_eq!(*value, 4),
        other => panic!("unexpected layout align {other:?}"),
    }

    let fields = collect_descriptor_list(get_field(layout_fields, "Fields"));
    assert_eq!(fields.len(), 2, "expected two layout fields");
    let left = fields[0];
    let right = fields[1];

    let extract_offset = |value: &ConstValue| match value {
        ConstValue::Struct { fields, .. } => match get_field(fields, "Offset") {
            ConstValue::UInt(value) => *value,
            other => panic!("unexpected offset value {other:?}"),
        },
        other => panic!("unexpected field layout value {other:?}"),
    };
    let extract_name = |value: &ConstValue| match value {
        ConstValue::Struct { fields, .. } => match get_field(fields, "Name") {
            ConstValue::RawStr(value) => value.clone(),
            ConstValue::Str { value, .. } => value.clone(),
            other => panic!("unexpected field name value {other:?}"),
        },
        other => panic!("unexpected field layout value {other:?}"),
    };

    assert_eq!(extract_name(left), "Left");
    assert_eq!(extract_offset(left), 0);
    assert_eq!(extract_name(right), "Right");
    assert_eq!(extract_offset(right), 4);
}
