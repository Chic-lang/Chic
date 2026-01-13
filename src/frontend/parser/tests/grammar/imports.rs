use crate::frontend::ast::{ImportKind, Item};
use crate::frontend::diagnostics::Severity;
use crate::frontend::parser::parse_module;
use crate::frontend::parser::tests::fixtures::*;

#[test]
fn parses_import_directives() {
    let source = r"
import Std.Text;
import MyCompany.Core;
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.items.len(), 2);
    assert!(matches!(parse.module.items[0], Item::Import(_)));
}

#[test]
fn parses_import_alias_and_static_directives() {
    let source = r"
import Std.Text;
/// Preferred collections
import Collections = Std.Collections.Generic;
import static Std.Math;
";

    let Ok(parse) = parse_module(source) else {
        panic!("expected successful parse");
    };
    assert!(parse.diagnostics.is_empty());
    let module = parse.module;
    assert_eq!(module.items.len(), 3);

    match &module.items[0] {
        Item::Import(import) => match &import.kind {
            ImportKind::Namespace { path } => assert_eq!(path, "Std.Text"),
            other => panic!("expected namespace import, found {other:?}"),
        },
        other => panic!("expected import directive, found {other:?}"),
    }

    match &module.items[1] {
        Item::Import(import) => {
            match &import.kind {
                ImportKind::Alias { alias, target } => {
                    assert_eq!(alias, "Collections");
                    assert_eq!(target, "Std.Collections.Generic");
                }
                other => panic!("expected alias import, found {other:?}"),
            }
            let doc = import.doc.as_ref().expect("alias doc missing");
            assert_eq!(doc.lines, vec!["Preferred collections".to_string()]);
        }
        other => panic!("expected import alias, found {other:?}"),
    }

    match &module.items[2] {
        Item::Import(import) => match &import.kind {
            ImportKind::Static { target } => assert_eq!(target, "Std.Math"),
            other => panic!("expected static import, found {other:?}"),
        },
        other => panic!("expected import directive, found {other:?}"),
    }
}

#[test]
fn parses_file_scoped_global_import_directives() {
    let source = r"
global import Root.Shared;
global import Alias = Root.Services;

namespace Inner
{
    public struct Holder { }
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let module = parse.module;
    assert_eq!(module.items.len(), 3);

    match &module.items[0] {
        Item::Import(import) => {
            assert!(import.is_global);
            match &import.kind {
                ImportKind::Namespace { path } => assert_eq!(path, "Root.Shared"),
                other => panic!("expected namespace import, found {other:?}"),
            }
        }
        other => panic!("expected import directive, found {other:?}"),
    }

    match &module.items[1] {
        Item::Import(import) => {
            assert!(import.is_global);
            match &import.kind {
                ImportKind::Alias { alias, target } => {
                    assert_eq!(alias, "Alias");
                    assert_eq!(target, "Root.Services");
                }
                other => panic!("expected alias import, found {other:?}"),
            }
        }
        other => panic!("expected import alias, found {other:?}"),
    }

    match &module.items[2] {
        Item::Namespace(ns) => {
            assert_eq!(ns.items.len(), 1);
        }
        other => panic!("expected namespace item, found {other:?}"),
    }
}

#[test]
fn rejects_global_import_inside_namespace() {
    let source = r"
global import Root.Shared;

namespace Inner
{
    global import static Root.Math.Helpers;
    public struct Holder { }
}
";

    let Err(err) = parse_module(source) else {
        panic!("expected parsing to surface error");
    };
    let diagnostics = err.diagnostics();
    assert_eq!(
        diagnostics.len(),
        1,
        "expected a single diagnostic, got {diagnostics:?}"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(
        diagnostic.code.as_ref().map(|code| code.code.as_str()),
        Some("E0G02")
    );
    assert!(
        diagnostic
            .message
            .contains("global import directives are not allowed inside namespaces or types"),
        "unexpected message: {}",
        diagnostic.message
    );
}

#[test]
fn rejects_global_import_after_declaration() {
    let source = r"
namespace Root;

global import Std;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parsing to surface error");
    };
    let diagnostics = err.diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0G01")),
        "expected E0G01 diagnostic, got {diagnostics:?}"
    );
}

#[test]
fn reports_self_referential_import_alias_with_span() {
    let source = r"
import Alias = Alias;
";

    let Err(err) = parse_module(source) else {
        panic!("expected alias self-cycle to error");
    };
    let diagnostics = err.diagnostics();
    assert_eq!(diagnostics.len(), 1, "expected single diagnostic");
    let diagnostic = &diagnostics[0];
    assert!(
        matches!(diagnostic.severity, Severity::Error),
        "expected error severity"
    );
    assert!(
        diagnostic
            .message
            .contains("import alias cannot reference itself"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    let span = diagnostic
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("alias self-cycle diagnostic should carry span");
    let snippet = &source[span.start..span.end];
    assert_eq!(snippet, "Alias", "expected span to cover alias identifier");
}

#[test]
fn reports_import_alias_cycle_with_span() {
    let source = r"
import A = B;
import B = A;
";

    let Err(err) = parse_module(source) else {
        panic!("expected alias cycle to error");
    };
    let diagnostics = err.diagnostics();
    assert_eq!(diagnostics.len(), 1, "expected single diagnostic");
    let diagnostic = &diagnostics[0];
    assert!(
        matches!(diagnostic.severity, Severity::Error),
        "expected error severity"
    );
    assert!(
        diagnostic.message.contains("forms a cycle"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    let span = diagnostic
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("alias cycle diagnostic should carry span");
    let snippet = &source[span.start..span.end];
    assert_eq!(snippet, "B", "expected span to cover alias identifier");
}

#[test]
fn reports_conflicting_std_alias() {
    let source = r"
import Std = Other.Root;
";

    let Err(err) = parse_module(source) else {
        panic!("expected alias conflict to error");
    };
    let diagnostics = err.diagnostics();
    assert_eq!(diagnostics.len(), 1, "expected single diagnostic");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(
        diagnostic.code.as_ref().map(|code| code.code.as_str()),
        Some("IMPORT0002")
    );
    assert!(
        diagnostic.message.contains("implicitly imported")
            || diagnostic.message.contains("cannot be aliased"),
        "unexpected diagnostic message: {diagnostic:?}"
    );
}

#[test]
fn attributes_not_supported_on_import_directives() {
    let source = r"
@thread_safe
import Std.Text;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("attributes are not supported on import directives")),
        "expected attribute misuse diagnostic on import directive, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn module_import_after_items_reports_error() {
    let source = r"
namespace Demo
{
    public struct Widget { }
}

import Std.Text;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0G01")),
        "expected module-level ordering diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn namespace_import_after_items_reports_error() {
    let source = r"
namespace Demo
{
    public struct Widget { }

    import Std.Text;
}
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert!(
        err.diagnostics().iter().any(|diag| diag.message.contains(
            "import directives must appear before other declarations within a namespace"
        )),
        "expected namespace ordering diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn module_global_import_after_items_reports_error() {
    let source = r"
namespace Demo
{
    public struct Widget { }
}

global import Std.Text;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.code.as_ref().map(|code| code.code.as_str()) == Some("E0G01")),
        "expected module-level ordering diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn namespace_global_import_after_items_reports_error() {
    let source = r"
namespace Demo
{
    public struct Widget { }

    global import Std.Text;
}
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("global import directives are not allowed inside namespaces or types")),
        "expected namespace ordering diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn reports_global_keyword_without_import() {
    let source = r"
global namespace Demo { }
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("`global` keyword may only prefix an import directive")),
        "expected global keyword diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn using_directive_reports_error() {
    let source = "using Foo.Bar;";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert_eq!(err.diagnostics().len(), 1, "expected import error");
    let diagnostic = &err.diagnostics()[0];
    assert_eq!(
        diagnostic.code.as_ref().map(|code| code.code.as_str()),
        Some("IMPORT0001")
    );
    assert!(
        matches!(diagnostic.severity, Severity::Error),
        "expected error severity for unsupported using directive"
    );
    let span = diagnostic
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("import diagnostic should carry span");
    assert_eq!(&source[span.start..span.end], "using");
    assert!(
        diagnostic
            .suggestions
            .iter()
            .any(|suggestion| suggestion.replacement.as_deref() == Some("import"))
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("resource-management `using` statements are unchanged"))
    );
}

#[test]
fn global_using_directive_reports_error() {
    let source = r"
global using static Foo.Bar;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parse to fail");
    };
    assert_eq!(err.diagnostics().len(), 1, "expected import error");
    let diagnostic = &err.diagnostics()[0];
    assert_eq!(
        diagnostic.code.as_ref().map(|code| code.code.as_str()),
        Some("IMPORT0001")
    );
    assert!(
        diagnostic
            .suggestions
            .iter()
            .any(|suggestion| suggestion.replacement.as_deref() == Some("import")),
        "expected replacement suggestion"
    );
}
