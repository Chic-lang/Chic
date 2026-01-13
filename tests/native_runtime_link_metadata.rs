use std::path::Path;

#[test]
fn native_runtime_link_metadata_present_when_enabled() {
    let enabled = std::env::var("CHIC_NATIVE_RUNTIME_LINK_TEST").ok();
    if enabled.as_deref() != Some("1") {
        eprintln!("skipping link metadata test (set CHIC_NATIVE_RUNTIME_LINK_TEST=1)");
        return;
    }

    let linked = option_env!("CHIC_NATIVE_RUNTIME_LINKED");
    assert_eq!(
        linked,
        Some("1"),
        "CHIC_NATIVE_RUNTIME_LINKED should be set when the native archive is linked"
    );
    assert!(
        cfg!(chic_native_runtime),
        "chic_native_runtime cfg flag should be set when the native archive is linked"
    );

    let archive = option_env!("CHIC_NATIVE_RUNTIME_ARCHIVE")
        .expect("CHIC_NATIVE_RUNTIME_ARCHIVE not set by build.rs");
    assert!(
        Path::new(archive).exists(),
        "expected native runtime archive at {}",
        archive
    );
}
