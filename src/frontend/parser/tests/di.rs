use crate::frontend::ast::{ClassMember, DiInjectAttr, DiLifetime, Item};
use crate::frontend::parser::tests::fixtures::{parse_fail, parse_ok};

fn expect_inject(attr: &Option<DiInjectAttr>) -> &DiInjectAttr {
    attr.as_ref().expect("expected di inject metadata")
}

#[test]
fn service_attribute_records_metadata() {
    let parse = parse_ok(
        r#"
@service(lifetime: Singleton, named: "UserRepo")
public class Repository { }
"#,
    );
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        other => panic!("expected class, found {other:?}"),
    };
    let service = class
        .di_service
        .as_ref()
        .expect("expected service metadata");
    assert_eq!(service.lifetime, Some(DiLifetime::Singleton));
    assert_eq!(service.named.as_deref(), Some("UserRepo"));
    assert!(!class.di_module, "module flag should be false");
}

#[test]
fn module_attribute_sets_flag() {
    let parse = parse_ok(
        r#"
@module
public class Registrations { }
"#,
    );
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        _ => panic!("expected class"),
    };
    assert!(class.di_module, "expected module flag");
    assert!(class.di_service.is_none());
}

#[test]
fn inject_attributes_attach_metadata() {
    let parse = parse_ok(
        r#"
public class Handler
{
    @inject(lifetime: Scoped, optional: true)
    public init(@inject(named: "client") HttpClient client)
    {
    }

    @inject
    public HttpClient Client { get; init; }
}
"#,
    );
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );

    let class = match &parse.module.items[0] {
        Item::Class(class) => class,
        _ => panic!("expected class"),
    };

    let constructor = class
        .members
        .iter()
        .find_map(|member| match member {
            ClassMember::Constructor(ctor) => Some(ctor),
            _ => None,
        })
        .expect("expected constructor");
    let ctor_inject = expect_inject(&constructor.di_inject);
    assert_eq!(ctor_inject.lifetime, Some(DiLifetime::Scoped));
    assert!(ctor_inject.named.is_none());
    assert!(ctor_inject.optional);

    let ctor_param = constructor
        .parameters
        .first()
        .expect("expected constructor parameter");
    assert_eq!(ctor_param.name, "client");
    let param_inject = expect_inject(&ctor_param.di_inject);
    assert_eq!(param_inject.named.as_deref(), Some("client"));
    assert_eq!(param_inject.lifetime, None);
    assert!(!param_inject.optional);

    let property = class
        .members
        .iter()
        .find_map(|member| match member {
            ClassMember::Property(prop) => Some(prop),
            _ => None,
        })
        .expect("expected property");
    let prop_inject = expect_inject(&property.di_inject);
    assert!(prop_inject.named.is_none());
    assert_eq!(prop_inject.lifetime, None);
    assert!(!prop_inject.optional);
}

#[test]
fn invalid_di_attributes_report_diagnostics() {
    let diagnostics = parse_fail(
        r#"
@service(lifetime: Unknown)
@service
public class Broken { }
"#,
    );
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("unsupported DI lifetime")),
        "expected unsupported lifetime diagnostic, got {:?}",
        diagnostics
    );
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("duplicate `@service`")),
        "expected duplicate service diagnostic, got {:?}",
        diagnostics
    );
}
