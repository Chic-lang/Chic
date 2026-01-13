use super::helpers::*;
use crate::frontend::ast::{Item, VectorizeHint};
use crate::frontend::parser::parse_module;

#[test]
fn macro_metadata_records_tokens_and_expandable_flag() {
    let source = r"
@derive(Equatable, Hashable)
public struct Point { public int X; }
";

    let parse = parse_module(source).expect("expected parse to succeed");
    let derive = match &parse.module.items[0] {
        Item::Struct(def) => def
            .attributes
            .first()
            .expect("expected derive attribute on struct"),
        other => panic!("expected struct item, found {other:?}"),
    };

    assert!(
        derive.macro_metadata.expandable,
        "expected macro attribute to be marked expandable"
    );
    let lexemes: Vec<&str> = derive
        .macro_metadata
        .tokens
        .iter()
        .map(|token| token.lexeme.as_str())
        .collect();
    assert!(
        lexemes.iter().any(|lex| *lex == "derive"),
        "expected to capture derive identifier, found {lexemes:?}"
    );
    assert!(
        lexemes.iter().any(|lex| *lex == "Equatable")
            && lexemes.iter().any(|lex| *lex == "Hashable"),
        "expected to capture macro arguments, found {lexemes:?}"
    );
}

#[test]
fn builtin_attributes_are_not_expandable() {
    let source = r"
@thread_safe
public class Service {}
";

    let parse = parse_module(source).expect("expected parse to succeed");
    let attr = match &parse.module.items[0] {
        Item::Class(class) => class
            .attributes
            .first()
            .expect("expected attribute on class"),
        other => panic!("expected class item, found {other:?}"),
    };

    assert!(
        !attr.macro_metadata.expandable,
        "builtin attributes should not be scheduled for macro expansion"
    );
    assert!(
        !attr.macro_metadata.tokens.is_empty(),
        "expected to retain raw tokens for diagnostics even when non-expandable"
    );
}

#[test]
fn pin_attribute_requires_variable_declaration() {
    let source = r"
public void Example()
{
    @pin return;
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("`@pin` attribute is only supported on variable declarations")),
        "expected `@pin` misuse diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn thread_safe_attribute_not_allowed_on_statements() {
    let source = r"
public void Example()
{
    @thread_safe var count = 1;
}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains(
            "`@thread_safe`/`@shareable` attributes are only supported on type declarations"
        )),
        "expected thread-safe misuse diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn duplicate_pin_attribute_reports_error() {
    let source = r"
@pin
@pin
public struct Buffer {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@pin` attribute")),
        "expected duplicate pin diagnostic: {diagnostics:?}"
    );
}

#[test]
fn thread_safe_duplicate_reports_error() {
    let source = diagnostic_fixture(&["@thread_safe", "@thread_safe"]);
    let (_parse, diagnostics) = parse_with_diagnostics(&source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@thread_safe` attribute is repeated")),
        "expected duplicate thread-safety diagnostic: {diagnostics:?}"
    );
}

#[test]
fn thread_safe_conflicts_are_reported() {
    let source = r"
@thread_safe
@not_thread_safe
public class Service {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("conflicting thread-safety attributes")),
        "expected thread-safety conflict diagnostic: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_duplicate_pack_reports_error() {
    let source = r"
@StructLayout(LayoutKind.Sequential, Pack = 4, Pack = 8)
public struct Packet { public int Value; }
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("duplicate `Pack` argument in `@StructLayout` attribute")),
        "expected struct layout duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_duplicate_align_reports_error() {
    let (_, diagnostics) = collect_attributes_from_source(
        "@StructLayout(LayoutKind.Sequential, Align = 4, Align = 8)",
    );
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `Align` argument")),
        "expected duplicate align diagnostic: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_records_pack_and_align_hints() {
    let source = layout_fixture("LayoutKind.Sequential, Pack = 8, Align = 16");
    let (parse, diagnostics) = parse_with_diagnostics(&source);
    assert!(
        diagnostics.is_empty(),
        "expected struct layout arguments to parse cleanly: {diagnostics:?}"
    );
    let module = parse.expect("expected parse result").module;
    let layout = match &module.items[0] {
        Item::Struct(def) => def.layout,
        other => panic!("expected struct item, found {other:?}"),
    }
    .expect("expected struct layout hints to be recorded");
    assert_eq!(layout.packing.and_then(|pack| pack.value), Some(8));
    assert_eq!(layout.align.map(|align| align.value), Some(16));
    assert!(layout.repr_c, "expected repr_c flag for Sequential layout");
}

#[test]
fn struct_layout_invalid_kind_reports_error() {
    let source = r"
@StructLayout(LayoutKind.Packed)
public struct Invalid {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("unsupported `@StructLayout` kind")),
        "expected invalid layout kind diagnostic: {diagnostics:?}"
    );
}

#[test]
fn extern_duplicate_attribute_reports_error() {
    let source = r#"
@extern("C")
@extern(library = "example")
public extern void Native();
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@extern` attribute")),
        "expected duplicate extern diagnostic: {diagnostics:?}"
    );
}

#[test]
fn export_attribute_requires_string_literal() {
    let source = codegen_fixture("export(123)");
    let (_parse, diagnostics) = parse_with_diagnostics(&source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("expected string literal for `@export` attribute")),
        "expected export string literal diagnostic: {diagnostics:?}"
    );
}

#[test]
fn extern_optional_rejects_non_bool_strings() {
    let source = codegen_fixture("extern(optional = \"maybe\")");
    let (_parse, diagnostics) = parse_with_diagnostics(&source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("`optional` argument for `@extern` expects `true` or `false`")),
        "expected optional argument diagnostic: {diagnostics:?}"
    );
}

#[test]
fn extern_optional_rejects_numeric_literal() {
    let (_, diagnostics) = collect_attributes_from_source("@extern(optional = 1)");
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("`optional` argument for `@extern` expects `true` or `false`")),
        "expected numeric optional diagnostic: {diagnostics:?}"
    );
}

#[test]
fn extern_attribute_reports_duplicate_alias_argument() {
    let (_, diagnostics) = collect_attributes_from_source("@extern(alias = \"A\", alias = \"B\")");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `alias` argument")),
        "expected duplicate alias diagnostic for extern: {diagnostics:?}"
    );
}

#[test]
fn extern_attribute_reports_duplicate_charset_argument() {
    let (_, diagnostics) =
        collect_attributes_from_source("@extern(charset = \"utf8\", charset = \"utf16\")");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `charset` argument")),
        "expected duplicate charset diagnostic for extern: {diagnostics:?}"
    );
}

#[test]
fn vectorize_rejects_non_decimal_targets() {
    let source = r"
@vectorize(foo)
public struct Example {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("only supports `decimal`")),
        "expected vectorize target diagnostic: {diagnostics:?}"
    );
}

#[test]
fn vectorize_requires_closing_parenthesis() {
    let source = r"
@vectorize(decimal
public struct MissingParen {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    let has_paren_error = messages(&diagnostics).any(|msg| msg.contains("expected ')'"));
    assert!(
        has_paren_error,
        "expected closing parenthesis diagnostic for vectorize: {diagnostics:?}"
    );
}

#[test]
fn vectorize_rejects_non_identifier_literals() {
    let source = "@vectorize(123)";
    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("requires an identifier target like")),
        "expected type-mismatch diagnostic for vectorize target: {diagnostics:?}"
    );
}

#[test]
fn inline_attribute_parses_cross_and_rejects_unknown() {
    let source = r#"
@inline(cross)
public void Fast() {}

@inline(always)
public void Slow() {}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("inline attribute must be `local` or `cross`")),
        "expected inline diagnostic for unknown strategy"
    );
}

#[test]
fn fallible_attribute_rejects_arguments() {
    let source = r"
@fallible(true)
public void MaybeFails() {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    let has_argument_error =
        messages(&diagnostics).any(|msg| msg.contains("does not accept arguments"));
    assert!(
        has_argument_error,
        "expected fallible argument diagnostic: {diagnostics:?}"
    );
}

#[test]
fn fallible_duplicate_reports_error() {
    let (_, diagnostics) = collect_attributes_from_source("@fallible\n@fallible");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@fallible` attribute")),
        "expected fallible duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn shareable_duplicate_reports_error() {
    let source = r"
@shareable
@shareable
public class Shared {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@shareable` attribute is repeated")),
        "expected shareable duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn not_shareable_duplicate_reports_error() {
    let (_, diagnostics) = collect_attributes_from_source("@not_shareable\n@not_shareable");
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@not_shareable` attribute is repeated")),
        "expected not_shareable duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn extern_attribute_accepts_full_argument_set() {
    let source = r#"
@extern(convention = "C", library = "math", alias = "Foo", binding = "lazy", optional = true, charset = "utf8")
public extern void Foo();
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected extern attribute with full argument set to parse cleanly: {diagnostics:?}"
    );
}

#[test]
fn link_attribute_requires_non_empty_library() {
    let source = r#"
@link("")
public extern void Foo();
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("requires a non-empty library name")),
        "expected link diagnostic for empty name: {diagnostics:?}"
    );
}

#[test]
fn vectorize_decimal_literal_target_parses() {
    let source = r#"
@vectorize("decimal")
public struct DecimalTarget {}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected string literal vectorize target to succeed: {diagnostics:?}"
    );
}

#[test]
fn vectorize_accepts_mixed_case_decimal_literal() {
    let source = r#"
@vectorize("Decimal")
public decimal Dot(decimal lhs, decimal rhs) { return lhs; }
"#;

    let (parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected mixed-case vectorize target to succeed: {diagnostics:?}"
    );
    let module = parse.expect("expected parse result").module;
    let function = match &module.items[0] {
        Item::Function(func) => func,
        other => panic!("expected function, found {other:?}"),
    };
    assert!(matches!(
        function.vectorize_hint,
        Some(VectorizeHint::Decimal)
    ));
}

#[test]
fn vectorize_attribute_populates_function_hint() {
    let source = r#"
@vectorize(decimal)
public decimal Dot(decimal lhs, decimal rhs)
{
    return lhs;
}
"#;

    let (parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let module = parse.expect("expected parse result").module;
    let function = match &module.items[0] {
        Item::Function(func) => func,
        other => panic!("expected function, found {other:?}"),
    };
    assert!(matches!(
        function.vectorize_hint,
        Some(VectorizeHint::Decimal)
    ));
}

#[test]
fn vectorize_attribute_requires_decimal_argument() {
    let source = r#"
@vectorize(foo)
public decimal Dot(decimal lhs, decimal rhs)
{
    return lhs;
}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@vectorize` only supports `decimal`")),
        "expected invalid target diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn vectorize_attribute_not_allowed_on_statements() {
    let source = r#"
public void Example()
{
    @vectorize(decimal) var value = 0;
}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg
                .contains("`@vectorize` attribute is only supported on function declarations")),
        "expected statement misuse diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn extern_attribute_shorthand_and_binding_errors() {
    let source = r#"
@extern "C"
public extern void Foo();

@extern(binding = "invalid")
public extern void Bar();
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("binding must be `lazy`, `eager`, or `static`")),
        "expected binding diagnostic for invalid value: {diagnostics:?}"
    );
}

#[test]
fn cimport_attribute_accumulates_headers() {
    let source = r#"
@cimport("alpha.h")
@cimport("beta.h")
public extern void Foo();
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected cimport attributes to parse cleanly: {diagnostics:?}"
    );
}

#[test]
fn no_std_and_global_allocator_report_errors() {
    let source = r"
@no_std
@global_allocator
public class Config {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("`@no_std` is not supported"))
            && messages(&diagnostics)
                .any(|msg| msg.contains("`@global_allocator` is not supported")),
        "expected diagnostics for unsupported configuration attributes: {diagnostics:?}"
    );
}

#[test]
fn intrinsic_duplicate_reports_error() {
    let source = r"
@intrinsic
@intrinsic
public extern void Helper();
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@Intrinsic` attribute")),
        "expected intrinsic duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_invalid_numeric_arguments_report_error() {
    let source = r#"
@StructLayout(LayoutKind.Sequential, Pack = foo)
public struct BadPack {}

@StructLayout(LayoutKind.Sequential, Align = bar)
public struct BadAlign {}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    let has_pack_error =
        messages(&diagnostics).any(|msg| msg.contains("`pack`") && msg.contains("StructLayout"));
    let has_align_error =
        messages(&diagnostics).any(|msg| msg.contains("`align`") && msg.contains("StructLayout"));
    assert!(
        has_pack_error && has_align_error,
        "expected pack/align diagnostics: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_accepts_double_colon_kind_separator() {
    let (_, diagnostics) = collect_attributes_from_source("@StructLayout(LayoutKind::Sequential)");
    assert!(
        diagnostics.is_empty(),
        "expected double-colon kind separator to parse: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_reports_single_colon_separator() {
    let (_, diagnostics) = collect_attributes_from_source("@StructLayout(LayoutKind:Sequential)");
    assert!(
        messages(&diagnostics)
            .any(|msg| msg.contains("unsupported `@StructLayout` kind `LayoutKind`")),
        "expected single-colon separator to be rejected: {diagnostics:?}"
    );
}

#[test]
fn struct_layout_kind_override_parses() {
    let source = r#"
@StructLayout(LayoutKind.Sequential, Kind = LayoutKind.Sequential, Align = 4)
public struct Sized {}
"#;

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected struct layout override to parse cleanly: {diagnostics:?}"
    );
}

#[test]
fn align_and_repr_attributes_are_consumed() {
    let source = r"
@repr(C)
@align(32)
public struct Sized { public int Value; }
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        diagnostics.is_empty(),
        "expected repr/align attributes to parse without diagnostics: {diagnostics:?}"
    );
}

#[test]
fn copy_attribute_conflicts_report_error() {
    let source = r"
@copy
@not_copy
public struct Resource {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("copy semantics")),
        "expected copy conflict diagnostic: {diagnostics:?}"
    );
}

#[test]
fn flags_attribute_duplicate_reports_error() {
    let source = r"
@flags
@flags
public enum Bits { A, B }
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("duplicate `@flags` attribute")),
        "expected flags duplicate diagnostic: {diagnostics:?}"
    );
}

#[test]
fn shareable_conflict_reports_error() {
    let source = r"
@shareable
@not_shareable
public class Sharing {}
";

    let (_parse, diagnostics) = parse_with_diagnostics(source);
    assert!(
        messages(&diagnostics).any(|msg| msg.contains("conflicting shareability attributes")),
        "expected shareability conflict diagnostic: {diagnostics:?}"
    );
}
