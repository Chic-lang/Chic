use crate::frontend::parser::tests::fixtures::parse_ok;

#[test]
fn parses_friend_directives_with_spans() {
    let source = r#"
@friend("Compat.Legacy")
@friend("Utilities.Helpers")
namespace Sample.Core;

public struct Widget {}
"#
    .to_string();

    let parse = parse_ok(&source);
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parse.diagnostics
    );
    assert_eq!(
        parse.module.friend_declarations.len(),
        2,
        "expected two friend directives"
    );

    let first = &parse.module.friend_declarations[0];
    assert_eq!(first.prefix, "Compat.Legacy");
    let first_span = first.span.expect("friend directive should record span");
    let first_snippet = &source[first_span.start..first_span.end];
    assert!(
        first_snippet.contains("@friend(\"Compat.Legacy\")"),
        "expected span to capture friend directive, got {first_snippet:?}"
    );

    let second = &parse.module.friend_declarations[1];
    assert_eq!(second.prefix, "Utilities.Helpers");
    let second_span = second.span.expect("friend directive should record span");
    let second_snippet = &source[second_span.start..second_span.end];
    assert!(
        second_snippet.contains("@friend(\"Utilities.Helpers\")"),
        "expected span to capture second friend directive, got {second_snippet:?}"
    );
}
