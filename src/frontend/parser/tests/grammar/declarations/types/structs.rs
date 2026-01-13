use super::helpers::*;
use super::*;

#[test]
fn struct_required_field_is_tracked() {
    let source = r"
public struct Sample
{
    public required int Field;
    public int Other;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let structure = match &parse.module.items[0] {
        Item::Struct(strct) => strct,
        other => panic!("expected struct, found {other:?}"),
    };

    assert_eq!(structure.fields.len(), 2);
    assert!(structure.fields[0].is_required);
    assert!(!structure.fields[1].is_required);
}

#[test]
fn struct_field_recovery_continues_after_error() {
    let source = r"
public struct Example
{
    public int First;
    public ??? Broken;
    public int Third;
}
";

    let lex_output = lex_tokens(source);
    let mut parser = Parser::new(source, lex_output);
    let module = parser.parse_module();
    let (diagnostics, _) = parser.finish();
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("expected type name")),
        "expected type name diagnostic, found {:?}",
        diagnostics
    );

    let structure = match &module.items[0] {
        Item::Struct(strct) => strct,
        other => panic!("expected struct, found {other:?}"),
    };

    let field_names: Vec<&str> = structure.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        ["First", "Third"],
        "unexpected fields: {field_names:?}"
    );
}

#[test]
fn struct_field_accepts_ref_qualifier() {
    let source = r#"
public struct Sample
{
    public ref int Value;
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let structure = match &parse.module.items[0] {
        Item::Struct(strct) => strct,
        other => panic!("expected struct, found {other:?}"),
    };
    let field = structure
        .fields
        .first()
        .expect("struct should contain one field");
    assert_eq!(
        field.ty.ref_kind,
        Some(RefKind::Ref),
        "field type should record ref qualifier"
    );
}

#[test]
fn struct_field_rejects_async_constexpr_extern_and_unsafe_modifiers() {
    let source = r"
public struct Sample
{
    public async constexpr extern unsafe int Value;
}
";

    let diagnostics = parse_fail(source);
    for expected in [
        "`async` modifier is not supported on struct fields",
        "`constexpr` modifier is not supported on struct fields",
        "`extern` modifier is not supported on struct fields",
        "`unsafe` modifier is not supported on struct fields",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains(expected)),
            "expected struct field diagnostic containing `{expected}`, found {:?}",
            diagnostics
        );
    }
}

#[test]
fn struct_field_duplicate_readonly_reports_error() {
    let source = r"
public struct Sample
{
    public readonly readonly int Value;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate `readonly` modifier on field")),
        "expected duplicate readonly diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_field_duplicate_required_reports_error() {
    let source = r"
public struct Sample
{
    public required required int Value;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate `required` modifier on field")),
        "expected duplicate required field diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_field_register_attribute_requires_mmio() {
    let source = r"
public struct Device
{
    @register(offset = 0, width = 32)
    public int Control;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`@register` attribute is only supported inside `@mmio` structs")),
        "expected register diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_const_register_attribute_is_rejected() {
    let source = r"
public struct Device
{
    @register(offset = 0, width = 32)
    public const int Control = 0;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`@register` attribute is only supported on fields inside `@mmio` structs")),
        "expected register const diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_inline_cross_attribute_is_recorded() {
    let source = r#"
@inline(cross)
public struct Inlineable
{
    public int Value;
}
"#;

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let structure = match &parse.module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected struct, found {other:?}"),
    };

    assert!(matches!(structure.inline_attr, Some(InlineAttr::Cross)));
}

#[test]
fn parses_struct_with_primitive_types() {
    let source = r"
namespace Geometry;

public struct Primitives
{
    public bool IsReady;
    public int Count;
    public uint Capacity;
    public long Total;
    public ulong Mask;
    public float Ratio;
    public double Precise;
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    let structure = match &module.items[0] {
        Item::Struct(def) => def,
        other => panic!("expected Item::Struct, found {other:?}"),
    };
    let types: Vec<&str> = structure
        .fields
        .iter()
        .map(|field| field.ty.name.as_str())
        .collect();
    assert_eq!(
        types,
        &["bool", "int", "uint", "long", "ulong", "float", "double"]
    );
}

#[test]
fn parses_readonly_struct_with_intrinsic_layout() {
    let source = r"
@Intrinsic
@StructLayout(LayoutKind.Sequential, Pack=2)
public readonly struct NativeValue
{
    public int Data;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct, found {:?}", parse.module.items[0]);
    };
    assert!(structure.is_readonly, "expected readonly struct flag");
    assert!(structure.is_intrinsic, "expected intrinsic flag");
    let layout = structure.layout.as_ref().expect("expected layout hints");
    assert!(layout.repr_c, "expected sequential layout to set repr_c");
    let pack_value = layout.packing.as_ref().and_then(|hint| hint.value);
    assert_eq!(pack_value, Some(2), "expected Pack=2 hint");
}

#[test]
fn parses_readonly_field_modifier_in_struct() {
    let source = r"
public struct Counter
{
    public readonly int Value;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct declaration");
    };
    let field = structure
        .fields
        .iter()
        .find(|field| field.name == "Value")
        .expect("readonly field missing");
    assert!(field.is_readonly, "expected readonly field flag");
}

#[test]
fn parses_generic_class_following_struct() {
    let source = r"
namespace Sample;

public struct AtomicU64
{
    private ulong Value;
}

public sealed class Mutex<T>
{
    private T Value;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
}

#[test]
fn struct_rejects_unknown_modifiers_on_declaration() {
    let source = r"
public sealed struct Invalid { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("modifier")
                && diag.message.contains("sealed")
                && diag.message.contains("struct")),
        "expected struct modifier diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_pin_attribute_is_rejected() {
    let source = r"
@pin
public struct Invalid { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@pin") && diag.message.contains("variable")),
        "expected struct @pin diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_rejects_flags_attribute() {
    let source = r"
@flags
public struct Invalid { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@flags") && diag.message.contains("enum")),
        "expected struct flags diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_duplicate_readonly_modifier_on_declaration_reports_error() {
    let source = r"
public readonly readonly struct Invalid { }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate `readonly` modifier on struct declarations")),
        "expected duplicate readonly diagnostic on struct declaration, found {:?}",
        diagnostics
    );
}

#[test]
fn nested_record_struct_reports_unsupported_modifiers() {
    let source = r"
public struct Container
{
    internal unsafe record struct Inner { }
}
";

    let (module, diagnostics) = parse_module_allowing_errors(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`unsafe` modifier is not supported on nested struct declarations")),
        "expected nested record struct modifier diagnostic, found {:?}",
        diagnostics
    );
    let Item::Struct(container) = &module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(
        container.nested_types.len(),
        1,
        "nested record struct was not recorded"
    );
}

#[test]
fn record_struct_semicolon_form_generates_constructor_and_positional_fields() {
    let source = r"
public record struct Point(int X, int Y);
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Struct(strct) = &parse.module.items[0] else {
        panic!("expected record struct declaration");
    };
    assert!(strct.is_record, "record flag should be set");
    let positional: Vec<&str> = strct
        .record_positional_fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(
        positional,
        ["X", "Y"],
        "positional metadata should mirror ctor"
    );
    assert_eq!(strct.fields.len(), 2);
    assert!(
        strct
            .fields
            .iter()
            .all(|field| field.is_readonly && field.is_required),
        "record positional fields should be readonly + required: {:?}",
        strct.fields
    );
    assert_eq!(strct.constructors.len(), 1, "record should synthesize ctor");
    let ctor = &strct.constructors[0];
    assert_eq!(ctor.parameters.len(), 2);
    let body = ctor.body.as_ref().expect("record ctor body");
    let assignments: Vec<&str> = body
        .statements
        .iter()
        .filter_map(|stmt| {
            if let crate::frontend::ast::StatementKind::Expression(expr) = &stmt.kind {
                Some(expr.text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(assignments, ["self.X = X", "self.Y = Y"]);
}

#[test]
fn record_struct_body_marks_fields_readonly() {
    let source = r"
public record struct Counter
{
    public int Count;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let Item::Struct(strct) = &parse.module.items[0] else {
        panic!("expected record struct declaration");
    };
    assert!(
        strct.fields.iter().all(|field| field.is_readonly),
        "record body fields should be readonly: {:?}",
        strct.fields
    );
    assert!(
        strct.constructors.is_empty(),
        "record body should rely on implicit parameterless construction"
    );
}

#[test]
fn record_primary_constructor_rejects_ref_binding() {
    let source = r"
public record struct Buffer(ref int Size);
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("record primary constructor parameters must use value or `in` binding")),
        "expected binding diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn record_primary_constructor_rejects_this_parameter() {
    let source = r"
public record struct Buffer(this int Size);
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`this` parameter is not valid in record primary constructors")),
        "expected `this` parameter diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_constructor_rejects_builtin_attributes() {
    let source = r"
public struct Service
{
    @thread_safe
    public init() { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("unsupported built-in attribute on struct constructors")),
        "expected constructor attribute diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_constructor_using_type_name_reports_error() {
    let source = r"
public struct Counter
{
    public Counter() { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0C01")),
        "expected type-named constructor diagnostic code E0C01, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_constructor_is_recorded() {
    let source = r"
public struct Counter
{
    public int Value;
    public init(int value)
    {
        Value = value;
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.constructors.len(), 1);
    let constructor = &structure.constructors[0];
    assert_eq!(constructor.kind, ConstructorKind::Designated);
    assert_eq!(constructor.parameters.len(), 1);
    assert_eq!(constructor.parameters[0].name, "value");
}

#[test]
fn struct_method_matching_type_name_reports_error() {
    let source = r"
public struct Counter
{
    public int Counter() { return 0; }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0C02")),
        "expected struct method type-name diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_constructor_rejects_async_modifier() {
    let source = r"
public struct Counter
{
    public async init() { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("struct constructors cannot be marked `async`")),
        "expected async constructor diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_constructor_duplicate_required_reports_error() {
    let source = r"
public struct Counter
{
    public required required init(int value) { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("duplicate `required` modifier")),
        "expected duplicate required diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_const_member_is_recorded() {
    let source = r"
public struct Constants
{
    public const int Size = 4;
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.consts.len(), 1);
    let const_decl = &structure.consts[0].declaration;
    assert_eq!(const_decl.declarators.len(), 1);
    assert_eq!(const_decl.declarators[0].name, "Size");
}

#[test]
fn struct_const_member_rejects_async_modifier() {
    let source = r"
public struct Constants
{
    public async const int Mask = 0xff;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`async` modifier is not supported on struct constants")),
        "expected const modifier diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_const_member_rejects_additional_modifiers() {
    let source = r"
public struct Constants
{
    public delegate const int Mask = 0xff;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("modifier")
                && diag.message.contains("delegate")
                && diag.message.contains("struct constants")),
        "expected const modifier diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_method_is_recorded() {
    let source = r"
public struct Math
{
    public int Value;
    public int Increment(int delta)
    {
        return delta;
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.methods.len(), 1);
    assert_eq!(structure.methods[0].name, "Increment");
}

#[test]
fn struct_method_rejects_async_constexpr_and_extern_modifiers() {
    let source = r"
public struct Math
{
    public async constexpr extern delegate int Compute(int value) => value;
}
";

    let diagnostics = parse_fail(source);
    let mut hits = 0;
    for needle in [
        "`async` modifier is not supported on struct methods",
        "`constexpr` modifier is not supported on struct methods",
        "`extern` modifier is not supported on struct methods",
        "modifier `delegate` is not supported on struct methods",
    ] {
        if diagnostics.iter().any(|diag| diag.message.contains(needle)) {
            hits += 1;
        }
    }
    assert_eq!(
        hits, 4,
        "expected all struct method modifier diagnostics, got {:?}",
        diagnostics
    );
}

#[test]
fn struct_method_duplicate_required_reports_error() {
    let source = r"
public struct Math
{
    public required required void Compute(int value) { }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("duplicate `required` modifier on field")),
        "expected duplicate required method diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_fields_only_allow_generics_on_methods() {
    let source = r"
public struct Weird
{
    public int Value<T>;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("generic parameter list is only supported on struct methods")),
        "expected field generics diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_view_of_clause_requires_view_field_type() {
    let source = r"
public struct Views
{
    public int Count of Owner;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`of` clause requires a `view` field type")),
        "expected view-of clause diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn struct_required_static_field_reports_error() {
    let source = r"
public struct Weird
{
    public required static int Value;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`required` modifier is not supported on struct fields")),
        "expected required static diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn nested_struct_reports_unsupported_modifiers() {
    let source = r"
public struct Container
{
    public delegate record struct Inner
    {
        public int Value;
    }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("modifier `delegate` is not supported on nested struct declarations")),
        "expected nested struct modifier diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn nested_struct_rejects_async_constexpr_extern_and_unsafe_modifiers() {
    let source = r"
public struct Container
{
    public required async constexpr extern unsafe struct Inner
    {
        public int Value;
    }
}
";

    let diagnostics = parse_fail(source);
    for expected in [
        "`required` modifier is not supported on nested struct declarations",
        "`async` modifier is not supported on nested struct declarations",
        "`constexpr` modifier is not supported on nested struct declarations",
        "`extern` modifier is not supported on nested struct declarations",
        "`unsafe` modifier is not supported on nested struct declarations",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains(expected)),
            "expected nested struct diagnostic containing `{expected}`, found {:?}",
            diagnostics
        );
    }
}

#[test]
fn struct_property_is_recorded() {
    let source = r"
public struct Container
{
    public int Value { get; set; }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let Item::Struct(structure) = &parse.module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.properties.len(), 1);
}

#[test]
fn struct_property_rejects_generic_parameters() {
    let source = r"
public struct Container
{
    public int Value<T> { get; }
}
";

    let (module, diagnostics) = parse_module_allowing_errors(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("properties cannot declare generic parameter lists")),
        "expected property generics diagnostic, found {:?}",
        diagnostics
    );
    let Item::Struct(structure) = &module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.properties.len(), 1);
}

#[test]
fn struct_property_rejects_async_modifier() {
    let source = r"
public struct Container
{
    public async int Value { get; }
}
";

    let (module, diagnostics) = parse_module_allowing_errors(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("properties cannot be marked `async`")),
        "expected async property diagnostic, found {:?}",
        diagnostics
    );
    let Item::Struct(structure) = &module.items[0] else {
        panic!("expected struct declaration");
    };
    assert_eq!(structure.properties.len(), 1);
}

#[test]
fn struct_property_duplicate_required_reports_error() {
    let source = r"
public struct Container
{
    public required required int Value { get; }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("duplicate `required` modifier")),
        "expected duplicate required property diagnostic, found {:?}",
        diagnostics
    );
}
