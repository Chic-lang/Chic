#![cfg(test)]

use super::fixtures::{parse_and_check, parse_lower_and_check};

#[test]
fn unqualified_base_via_import_should_resolve() {
    let source = r#"
namespace A { public class Base { } }
namespace B { import A; public class Derived : Base { } }
"#;

    let (_, report) = parse_and_check(source);
    assert!(
        report.diagnostics.is_empty(),
        "expected base resolution via import to succeed, got {:?}",
        report.diagnostics
    );
}

#[test]
fn lowering_resolves_base_via_import() {
    let source = r#"
namespace A { public class Base { } }
namespace B { import A; public class Derived : Base { } }
"#;

    let (_, report) = parse_lower_and_check(source);
    assert!(
        report.diagnostics.is_empty(),
        "expected lowering/typeck to resolve base via import, got {:?}",
        report.diagnostics
    );
}

#[test]
fn ambiguous_base_type_reports_single_diagnostic() {
    let source = r#"
namespace X { public class Base { } }
namespace Y { public class Base { } }
namespace Z {
    import X;
    import Y;
    public class Derived : Base { }
}
"#;

    let (_, report) = parse_and_check(source);
    assert!(
        !report.diagnostics.is_empty(),
        "expected ambiguity diagnostic for Base, got none"
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| diag.message.to_lowercase().contains("ambig")),
        "expected ambiguity message mentioning `Base`, got {:?}",
        report.diagnostics
    );
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diag| diag.severity.is_error())
            .count(),
        1,
        "expected a single primary error for ambiguous base type"
    );
}
