use super::helpers::*;
use super::*;

#[test]
fn enum_rejects_pin_attribute() {
    let source = r"
@pin
public enum State { Ready }
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@pin") && diag.message.contains("variable")),
        "expected enum pin diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn enum_records_variants_and_flags() {
    let source = r"
@flags
@thread_safe
@shareable
@copy
public enum Mode
{
    None,
    Read = 1,
    Write = 2,
    Exec { public int Depth; }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    let Item::Enum(decl) = &parse.module.items[0] else {
        panic!("expected enum declaration");
    };
    assert_eq!(decl.variants.len(), 4);
    assert_eq!(decl.thread_safe_override, Some(true));
    assert_eq!(decl.shareable_override, Some(true));
    assert_eq!(decl.copy_override, Some(true));
    assert!(decl.is_flags, "flags attribute should be recorded");
    assert_eq!(
        decl.variants
            .iter()
            .find(|variant| variant.name == "Exec")
            .and_then(|variant| variant.fields.first())
            .map(|field| field.name.as_str()),
        Some("Depth")
    );
}

#[test]
fn enum_rejects_discriminant_on_data_carrying_variant() {
    let source = r"
public enum Status
{
    Pending,
    Finished { public int Code; } = 2,
}
";

    let (module, diagnostics) = parse_module_allowing_errors(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("data-carrying enum variants cannot specify explicit discriminants")),
        "expected discriminant diagnostic, found {:?}",
        diagnostics
    );
    let Item::Enum(decl) = &module.items[0] else {
        panic!("expected enum declaration");
    };
    assert_eq!(decl.variants.len(), 2);
    assert!(
        decl.variants
            .iter()
            .any(|variant| variant.name == "Finished" && variant.discriminant.is_none()),
        "data-carrying variant should not keep discriminant"
    );
}

#[test]
fn enum_defaults_underlying_type_when_unspecified() {
    let source = r"
public enum Color
{
    Red,
    Green,
}
";

    let parse = parse_ok(source);
    let Item::Enum(decl) = &parse.module.items[0] else {
        panic!("expected enum declaration");
    };
    assert!(
        decl.underlying_type.is_none(),
        "underlying type should be implicit when clause is absent"
    );
}

#[test]
fn enum_records_underlying_type_clause() {
    let source = r"
public enum Status : Int16
{
    Pending = 0,
    Active,
}
";

    let parse = parse_ok(source);
    let Item::Enum(decl) = &parse.module.items[0] else {
        panic!("expected enum declaration");
    };
    let Some(underlying) = &decl.underlying_type else {
        panic!("expected underlying type to be parsed");
    };
    assert_eq!(underlying.name, "Int16");
}
