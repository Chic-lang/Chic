use super::common::RequireExt;
use super::*;

#[test]
fn di_manifest_records_services_and_dependencies() {
    let source = r#"
@service(lifetime: Singleton, named: "Cache")
public class Cache { }

@service
public class Consumer
{
    @inject
    public init(Cache cache) { }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let manifest = &lowering.module.attributes.di_manifest;
    assert_eq!(manifest.services.len(), 2, "expected two services");

    let cache = manifest
        .services
        .iter()
        .find(|svc| svc.name.ends_with("Cache"))
        .expect("missing Cache service");
    assert_eq!(cache.named.as_deref(), Some("Cache"));

    let consumer = manifest
        .services
        .iter()
        .find(|svc| svc.name.ends_with("Consumer"))
        .expect("missing Consumer service");
    assert_eq!(consumer.dependencies.len(), 1);
    let dependency = &consumer.dependencies[0];
    assert!(
        dependency.target.ends_with("Cache"),
        "expected dependency on Cache"
    );
}
