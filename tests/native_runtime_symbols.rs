use std::path::Path;

use chic::manifest::Manifest;

#[test]
fn native_runtime_manifest_covers_zero_init_and_runtime_metadata() {
    let manifest_path = Path::new("packages/runtime.native/manifest.yaml");
    let manifest = Manifest::discover(manifest_path)
        .expect("load runtime manifest")
        .unwrap_or_else(|| panic!("missing runtime manifest at {}", manifest_path.display()));
    assert!(
        manifest.runtime().is_some(),
        "runtime.native manifest must declare toolchain.runtime for runtime selection"
    );
    assert!(
        manifest.runtime_provides().is_some(),
        "runtime.native manifest must declare runtime.provides metadata"
    );
    assert!(
        manifest
            .source_roots()
            .iter()
            .any(|root| root.path.ends_with("src")),
        "runtime.native manifest must include the src directory in sources"
    );
    assert!(
        Path::new("packages/runtime.native/src/zero_init.ch").exists(),
        "zero_init runtime helper source should exist"
    );
}
