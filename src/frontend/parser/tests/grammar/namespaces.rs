use crate::frontend::ast::{ImportKind, Item};
use crate::frontend::diagnostics::Severity;
use crate::frontend::parser::parse_module;
use crate::frontend::parser::tests::fixtures::*;

#[test]
fn parses_nested_namespace() {
    let source = r"
namespace Outer.Inner.Deep;

public struct Widget { }
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.namespace.as_deref(), Some("Outer.Inner.Deep"));
}

#[test]
fn parses_block_namespace_hierarchy() {
    let source = r"
import Std.Text;

namespace Utilities.Core
{
    public struct Point { public int X; }

    namespace Diagnostics
    {
        public enum Kind { Info, Warning, Error }
    }
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.namespace, None);
    assert_eq!(parse.module.items.len(), 2);

    match &parse.module.items[0] {
        Item::Import(import) => match &import.kind {
            ImportKind::Namespace { path } => assert_eq!(path, "Std.Text"),
            other => panic!("expected namespace import, found {other:?}"),
        },
        other => panic!("expected import directive, found {other:?}"),
    }

    let ns = match &parse.module.items[1] {
        Item::Namespace(ns) => ns,
        other => panic!("expected namespace item, found {other:?}"),
    };
    assert_eq!(ns.name, "Utilities.Core");
    assert_eq!(ns.items.len(), 2);
    assert!(matches!(ns.items[0], Item::Struct(_)));
    match &ns.items[1] {
        Item::Namespace(inner) => {
            assert_eq!(inner.name, "Utilities.Core.Diagnostics");
            assert!(matches!(inner.items[0], Item::Enum(_)));
        }
        other => panic!("expected nested namespace, found {other:?}"),
    }
}

#[test]
fn warns_on_public_namespace_visibility() {
    let source = r"
public namespace Core
{
}
";

    let Ok(parse) = parse_module(source) else {
        panic!("expected successful parse");
    };
    assert_eq!(parse.diagnostics.len(), 1, "expected single warning");
    let warning = &parse.diagnostics[0];
    assert!(
        matches!(warning.severity, Severity::Warning),
        "expected warning severity"
    );
    assert!(
        warning
            .message
            .contains("namespace declarations ignore visibility modifiers"),
        "unexpected warning message: {}",
        warning.message
    );
    let span = warning
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("namespace visibility warning should carry span");
    let snippet = &source[span.start..span.end];
    assert!(
        snippet.contains("namespace"),
        "expected span to highlight namespace; got {snippet:?}"
    );
}

#[test]
fn warns_on_namespace_modifiers() {
    let source = r"
partial namespace Core
{
}
";

    let Ok(parse) = parse_module(source) else {
        panic!("expected successful parse");
    };
    assert_eq!(parse.diagnostics.len(), 1, "expected single warning");
    let warning = &parse.diagnostics[0];
    assert!(
        matches!(warning.severity, Severity::Warning),
        "expected warning severity"
    );
    assert!(
        warning
            .message
            .contains("namespace declarations do not support modifiers"),
        "unexpected warning message: {}",
        warning.message
    );
    let span = warning
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .expect("namespace modifier warning should carry span");
    let snippet = &source[span.start..span.end];
    assert!(
        snippet.contains("namespace"),
        "expected span to highlight namespace; got {snippet:?}"
    );
}

#[test]
fn warns_on_doc_comment_before_file_scoped_namespace() {
    let source = r"
/// Top-level namespace docs.
namespace Sample.Core;

public class Widget { }
";

    let Ok(parse) = parse_module(source) else {
        panic!("expected successful parse");
    };
    assert_eq!(
        parse.diagnostics.len(),
        1,
        "expected single warning for doc comment before namespace"
    );
    assert!(matches!(parse.diagnostics[0].severity, Severity::Warning));
}

#[test]
fn file_scoped_namespace_after_imports() {
    let source = r"
import Std.Text;

namespace Imports;
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let namespace = parse.module.namespace.as_ref().expect("namespace missing");
    assert_eq!(namespace, "Imports");
}

#[test]
fn file_scoped_namespace_allows_following_imports_and_types() {
    let source = r"
namespace Sample.Numeric;

import Std.Numeric;

public struct Number { public int Value; }
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.namespace.as_deref(), Some("Sample.Numeric"));
    assert_eq!(parse.module.items.len(), 2, "expected import plus struct");
    assert!(matches!(parse.module.items[0], Item::Import(_)));
    assert!(matches!(parse.module.items[1], Item::Struct(_)));
}

#[test]
fn file_scoped_namespace_prefixes_block_namespaces() {
    let source = r"
namespace Root.Base;

namespace Services
{
    public struct Widget { }

    namespace Diagnostics
    {
        public enum Kind { Info, Warning }
    }
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    assert_eq!(parse.module.namespace.as_deref(), Some("Root.Base"));
    assert_eq!(
        parse.module.items.len(),
        1,
        "expected single namespace item"
    );

    let services = match &parse.module.items[0] {
        Item::Namespace(ns) => ns,
        other => panic!("expected namespace item, found {other:?}"),
    };
    assert_eq!(services.name, "Root.Base.Services");
    assert_eq!(services.items.len(), 2);

    let diagnostics = match &services.items[1] {
        Item::Namespace(ns) => ns,
        other => panic!("expected nested namespace, found {other:?}"),
    };
    assert_eq!(diagnostics.name, "Root.Base.Services.Diagnostics");
}

#[test]
fn reports_dangling_attribute_at_namespace_end() {
    let source = r"
namespace Sample
{
    public struct Widget { }
    @thread_safe
}
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    let error = err
        .diagnostics()
        .iter()
        .find(|diag| {
            diag.message
                .contains("dangling attribute at end of namespace")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected dangling attribute diagnostic, got {:?}",
                err.diagnostics()
            )
        });
    assert!(
        matches!(error.severity, Severity::Error),
        "expected error severity for dangling namespace attribute"
    );
}

#[test]
fn reports_unexpected_closing_brace_at_namespace_scope() {
    let source = r"
namespace Sample
{
}
}
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    assert!(
        err.diagnostics().iter().any(|diag| diag
            .message
            .contains("unexpected closing brace at namespace scope")),
        "expected unexpected closing brace diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_attributes_on_file_scoped_namespace() {
    let source = r"
@thread_safe
namespace Sample;
";

    let Err(err) = parse_module(source) else {
        panic!("expected parser to report an error");
    };
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("attributes are not supported")),
        "expected attribute misuse diagnostic, got {:?}",
        err.diagnostics()
    );
}
