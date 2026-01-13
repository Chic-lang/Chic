use crate::frontend::parser::tests::fixtures::parse_ok;

#[test]
fn parses_package_directives_with_spans() {
    let source = r#"
@package("Core.Logging")
@package("ThirdParty.Analytics")
namespace App.Core;
"#
    .to_string();

    let parse = parse_ok(&source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    assert_eq!(parse.module.package_imports.len(), 2);

    let first = &parse.module.package_imports[0];
    assert_eq!(first.name, "Core.Logging");
    let first_span = first.span.expect("package directive should have a span");
    let first_snippet = &source[first_span.start..first_span.end];
    assert!(
        first_snippet.contains("Core.Logging"),
        "expected span to include directive text"
    );

    let second = &parse.module.package_imports[1];
    assert_eq!(second.name, "ThirdParty.Analytics");
    let second_span = second.span.expect("package directive should have a span");
    let second_snippet = &source[second_span.start..second_span.end];
    assert!(
        second_snippet.contains("ThirdParty.Analytics"),
        "expected span to include directive text"
    );
}
