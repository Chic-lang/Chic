use super::*;

#[test]
fn parses_auto_property_with_init_accessor() {
    let source = r"
public class Counter
{
    public int Value { get; init; }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let property = match &class.members[0] {
        ClassMember::Property(property) => property,
        other => panic!("expected property, found {other:?}"),
    };
    assert_eq!(property.name, "Value");
    assert!(property.is_auto());
    assert!(property.accessor(PropertyAccessorKind::Get).is_some());
    assert!(property.accessor(PropertyAccessorKind::Init).is_some());
}

#[test]
fn parses_static_property_modifier() {
    let source = r"
public class Config
{
    public static string Name { get; }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let property = match &class.members[0] {
        ClassMember::Property(property) => property,
        other => panic!("expected property, found {other:?}"),
    };
    assert_eq!(property.name, "Name");
    assert!(property.is_static, "expected property to be marked static");
    assert!(property.accessor(PropertyAccessorKind::Get).is_some());
}

#[test]
fn parses_expression_bodied_property_with_accessor_visibility() {
    let source = r"
public class Person
{
    private string _name;
    public string Name
    {
        get => _name;
        private set => _name = value;
    }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let property = match &class.members[1] {
        ClassMember::Property(property) => property,
        other => panic!("expected property, found {other:?}"),
    };
    assert_eq!(property.name, "Name");
    assert!(!property.is_auto());
    let getter = property
        .accessor(PropertyAccessorKind::Get)
        .expect("missing getter");
    assert!(matches!(getter.body, PropertyAccessorBody::Expression(_)));
    let setter = property
        .accessor(PropertyAccessorKind::Set)
        .expect("missing setter");
    assert_eq!(setter.visibility, Some(Visibility::Private));
    assert!(matches!(setter.body, PropertyAccessorBody::Expression(_)));
}

#[test]
fn parses_required_field_and_property() {
    let source = r"
public class Sample
{
    public required int Field;
    public required int Value { get; init; }
}
";

    let parse = parse_ok(source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };

    let field = match &class.members[0] {
        ClassMember::Field(field) => field,
        other => panic!("expected field, found {other:?}"),
    };
    assert!(field.is_required, "expected field to be required");

    let property = match &class.members[1] {
        ClassMember::Property(property) => property,
        other => panic!("expected property, found {other:?}"),
    };
    assert!(property.is_required, "expected property to be required");
}

#[test]
fn rejects_mixed_auto_and_manual_property_accessors() {
    let source = r"
public class Sample
{
    public int Value
    {
        get;
        set => value = value;
    }
}
";

    let diagnostics = match parse_module(source) {
        Ok(parsed) => parsed.diagnostics,
        Err(err) => err.diagnostics().to_vec(),
    };
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("auto-property accessors may not mix")),
        "expected diagnostic for mixed accessor bodies, found {:?}",
        diagnostics
    );
}
