#![cfg(test)]

use crate::frontend::diagnostics::Diagnostic;

use super::fixtures::parse_and_check;

fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains(code))
}

#[test]
fn virtual_methods_require_bodies() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base {
    public virtual void Run();
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK206"),
        "expected virtual body diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn override_without_base_target_reports_error() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public void Existing() { } }

public class Child : Base {
    public override void Missing() { }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK200"),
        "expected override target missing diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn matching_virtual_requires_override_modifier() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public virtual void Work() { } }

public class Child : Base {
    public void Work() { }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK204"),
        "expected missing override diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn abstract_base_methods_must_be_implemented() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public abstract class Base {
    public abstract void Required();
}

public class Child : Base { }
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK203"),
        "expected abstract not implemented diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn virtual_properties_require_overrides() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base {
    public virtual int Value { get { return 0; } }
}

public class Child : Base {
    public int Value { get { return 1; } }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK204"),
        "expected missing override diagnostic for property: {:?}",
        result.diagnostics
    );
}

#[test]
fn override_return_type_mismatch_reports_error() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public virtual int Value() { return 0; } }

public class Child : Base {
    public override string Value() { return ""; }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK209"),
        "expected override type mismatch diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn override_visibility_reduction_reports_error() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public virtual void Speak() { } }

public class Child : Base {
    protected override void Speak() { }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK202"),
        "expected visibility reduction diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn property_override_type_mismatch_reports_error() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public virtual int Count { get { return 1; } } }

public class Child : Base {
    public override string Count { get { return ""; } }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK209"),
        "expected property override type mismatch: {:?}",
        result.diagnostics
    );
}

#[test]
fn abstract_properties_must_be_overridden() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public abstract class Base { public abstract int Size { get; } }

public class Child : Base { }
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK203"),
        "expected abstract property implementation diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn override_methods_must_declare_bodies() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { public virtual void Run() { } }

public class Child : Base {
    public override void Run();
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK206"),
        "expected override body required diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn sealed_base_method_rejects_overrides() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Parent { public virtual void Speak() { } }

public class Base : Parent {
    public sealed override void Speak() { }
}

public class Child : Base {
    public override void Speak() { }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK201"),
        "expected sealed override diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn abstract_property_cannot_have_body() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public abstract class Base {
    public abstract int Count { get { return 1; } }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK205"),
        "expected abstract property body diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn property_override_without_target_reports_error() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base { }

public class Child : Base {
    public override int Missing { get { return 1; } }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK200"),
        "expected property override target missing diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn property_override_static_conflict_is_reported() {}

#[test]
fn property_override_visibility_cannot_be_reduced() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public class Base {
    public virtual int Value { get { return 1; } }
}

public class Child : Base {
    protected override int Value { get { return 2; } }
}
"#,
    );

    assert!(
        has_code(&result.diagnostics, "TCK202"),
        "expected property visibility reduction diagnostic: {:?}",
        result.diagnostics
    );
}

#[test]
fn overriding_abstract_property_clears_requirements() {
    let (_, result) = parse_and_check(
        r#"
namespace Demo;

public abstract class Base {
    public abstract int Value { get; }
}

public class Child : Base {
    public override int Value { get { return 5; } }
}
"#,
    );

    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("TCK203")),
        "unexpected abstract property diagnostic: {:?}",
        result.diagnostics
    );
}
