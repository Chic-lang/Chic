use super::*;

fn lower_module_source(source: &str) -> LoweringResult {
    let parsed = parse_module(source).expect("module should parse");
    lower_module(&parsed.module)
}

#[test]
fn preserves_extern_metadata_during_lowering() {
    let source = r#"
namespace Interop {
    @extern(
        convention = "system",
        library = "user32",
        alias = "MessageBoxW",
        binding = "eager",
        optional = true,
        charset = "utf16"
    )
    public extern int MessageBox(string text);
}
"#;
    let lowering = lower_module_source(source);
    let func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Interop::MessageBox")
        .expect("message box function");
    let spec = func
        .extern_spec
        .as_ref()
        .expect("extern metadata should be present");
    assert_eq!(spec.convention, "system");
    assert_eq!(spec.library.as_deref(), Some("user32"));
    assert_eq!(spec.alias.as_deref(), Some("MessageBoxW"));
    assert_eq!(spec.binding, crate::frontend::ast::ExternBinding::Eager);
    assert!(spec.optional);
    assert_eq!(spec.charset.as_deref(), Some("utf16"));
}
