use super::helpers::*;
use crate::frontend::ast::Item;
use crate::frontend::parser::attributes::ParsedAttributeValue;

#[test]
fn attributes_require_balanced_parentheses() {
    let source = r"
public void Example()
{
    @pin(value
    var buffer = Allocate();
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    let has_balanced_error = messages(&diagnostics).any(|msg| {
        msg.contains("unterminated delimiter") || msg.contains("expected `)` to close expression")
    });
    assert!(
        has_balanced_error,
        "expected unterminated delimiter diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn attributes_not_supported_on_import_directives() {
    let source = r"
@thread_safe import Std.Text;
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("attributes are not supported on import directives")),
        "expected import directive diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn invalid_attribute_names_report_errors() {
    let source = r"
public void Example()
{
    @() var first = 0;
    @pin. var second = 1;
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    let mut expected = 0;
    for message in messages(&diagnostics) {
        if message.contains("expected attribute name") {
            expected |= 1;
        }
        if message.contains("expected identifier after '.' in attribute name") {
            expected |= 2;
        }
    }
    assert_eq!(
        expected, 3,
        "expected attribute name diagnostics not emitted: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_missing_comma_reports_error() {
    let (_, diagnostics) =
        collect_attributes_from_source("@StructLayout(LayoutKind.Sequential Pack = 1)");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("expected ','")),
        "expected missing comma diagnostic for struct layout: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_missing_equals_reports_error() {
    let (_, diagnostics) =
        collect_attributes_from_source("@StructLayout(LayoutKind.Sequential, Pack 4)");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("expected '=' after `Pack`")),
        "expected missing '=' diagnostic for struct layout: {diagnostics:?}"
    );
}

#[test]
fn split_attribute_arguments_respects_nested_generics() {
    let parser = parser_fixture("");
    let args =
        parser.split_attribute_arguments("first = Map<string, List<int>>, second: [1, 2, 3]", 10);
    assert_eq!(args.len(), 2, "expected to retain both attribute arguments");
    assert_eq!(args[0].name.as_deref(), Some("first"));
    assert_eq!(args[1].name.as_deref(), Some("second"));
}

#[test]
fn parse_attribute_arguments_from_raw_returns_empty_when_unclosed() {
    let parser = parser_fixture("@macro(value");
    let args = parser.parse_attribute_arguments_from_raw(Some("@macro(value"), 0);
    assert!(
        args.is_empty(),
        "expected unterminated attributes to yield no parsed arguments"
    );
}

#[test]
fn parse_attribute_kv_arguments_allows_empty_list() {
    let mut parser = parser_fixture("()");
    let args = parser
        .parse_attribute_kv_arguments("empty")
        .expect("expected empty arg list");
    assert!(args.is_empty(), "expected no arguments to be present");
}

#[test]
fn parse_attribute_value_coerces_identifier_to_string() {
    let mut parser = parser_fixture("Identifier");
    let (value, _) = parser
        .parse_attribute_value("test", "key")
        .expect("expected identifier value");
    match value {
        ParsedAttributeValue::Str(text) => assert_eq!(text, "Identifier"),
        other => panic!("expected string value, found {other:?}"),
    }
}

#[test]
fn parse_attribute_value_rejects_punctuation_tokens() {
    let mut parser = parser_fixture("%");
    assert!(
        parser.parse_attribute_value("test", "key").is_none(),
        "expected unsupported punctuation to be rejected"
    );
    let (diagnostics, _) = parser.finish();
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("unsupported value for `key` in `@test` attribute")),
        "expected unsupported value diagnostic for punctuation: {diagnostics:?}"
    );
}

#[test]
fn parse_attribute_arguments_from_raw_handles_absent_text() {
    let parser = parser_fixture("");
    let args = parser.parse_attribute_arguments_from_raw(None, 0);
    assert!(args.is_empty(), "expected None raw to return no arguments");
}

#[test]
fn parse_attribute_kv_arguments_require_equals_sign() {
    let mut parser = parser_fixture("(key value)");
    assert!(
        parser.parse_attribute_kv_arguments("attr").is_none(),
        "expected kv parsing to fail when '=' is missing"
    );
    let (diagnostics, _) = parser.finish();
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("expected '=' after `key` in `@attr` attribute")),
        "expected missing '=' diagnostic: {diagnostics:?}"
    );
}

#[test]
fn split_attribute_arguments_ignores_empty_segments() {
    let parser = parser_fixture("");
    let args = parser.split_attribute_arguments("first, , second,", 0);
    assert_eq!(args.len(), 2, "expected empty segments to be dropped");
    assert_eq!(args[0].name.as_deref(), None);
    assert_eq!(args[1].name.as_deref(), None);
}

#[test]
fn complex_attribute_name_and_arguments_parse() {
    let source = r#"
@outer.inner.attribute(first = 1, second = foo(32), third = "value")
public struct Fancy {}
"#;

    let (parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected complex attribute to parse: {diagnostics:?}"
    );
    let module = parse.expect("expected parse result").module;
    let attrs = match &module.items[0] {
        Item::Struct(def) => &def.attributes,
        other => panic!("expected struct, found {other:?}"),
    };
    assert_eq!(
        attrs.first().map(|attr| attr.name.as_str()),
        Some("outer.inner.attribute")
    );
    assert_eq!(attrs.first().map(|attr| attr.arguments.len()), Some(3));
}
