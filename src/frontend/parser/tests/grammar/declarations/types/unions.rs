use super::*;

#[test]
fn parses_union_with_views_and_fields() {
    let parse = parse_ok(PIXEL_SOURCE);
    assert!(parse.diagnostics.is_empty());

    let module = parse.module;
    assert_pixel_union(&module);
}

#[test]
fn union_field_accepts_ref_qualifier() {
    let source = r#"
public union PointOrRef
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

    let union = match &parse.module.items[0] {
        Item::Union(union) => union,
        other => panic!("expected union, found {other:?}"),
    };
    let field = union
        .members
        .iter()
        .find_map(|member| match member {
            UnionMember::Field(field) => Some(field),
            _ => None,
        })
        .expect("union should contain a field");
    assert_eq!(
        field.ty.ref_kind,
        Some(RefKind::Ref),
        "field type should record ref qualifier"
    );
}

#[test]
fn union_rejects_pin_attribute() {
    let source = r"
@pin
public union Pixel
{
    public int R;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@pin") && diag.message.contains("variable")),
        "expected union pin diagnostic, found {:?}",
        diagnostics
    );
}

#[test]
fn union_rejects_flags_attribute() {
    let source = r"
@flags
public union Pixel
{
    public int R;
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("@flags") && diag.message.contains("enum")),
        "expected union flags diagnostic, found {:?}",
        diagnostics
    );
}
