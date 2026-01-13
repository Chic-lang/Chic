#![cfg(test)]

use crate::frontend::ast::Module;
use crate::frontend::diagnostics::Span;
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::{
    AutoTraitKind, ObjectSafetyViolation, ObjectSafetyViolationKind, TraitObjectSafety,
    check_module,
};

#[test]
fn auto_trait_display_names_match_kinds() {
    assert_eq!(AutoTraitKind::ThreadSafe.display_name(), "ThreadSafe");
    assert_eq!(AutoTraitKind::Shareable.display_name(), "Shareable");
    assert_eq!(AutoTraitKind::Copy.display_name(), "Copy");
}

#[test]
fn trait_object_safety_describe_includes_violation_counts() {
    let mut safety = TraitObjectSafety::default();
    safety.record(ObjectSafetyViolation {
        kind: ObjectSafetyViolationKind::ReturnsSelf,
        member: "ReturnsSelf".to_string(),
        span: Some(Span::new(1, 2)),
    });
    safety.record(ObjectSafetyViolation {
        kind: ObjectSafetyViolationKind::GenericMethod,
        member: "Generic".to_string(),
        span: None,
    });

    let description = safety
        .describe()
        .expect("expected description with violations");
    assert!(
        description.contains("method `ReturnsSelf` returns `Self`"),
        "missing primary violation: {description}"
    );
    assert!(
        description.contains("+1 more issue"),
        "missing trailing count: {description}"
    );
    assert_eq!(safety.violation_span(), Some(Span::new(1, 2)));
}

#[test]
fn trait_object_safety_describes_generic_methods() {
    let violation = ObjectSafetyViolation {
        kind: ObjectSafetyViolationKind::GenericMethod,
        member: "Generic".to_string(),
        span: None,
    };
    assert_eq!(
        violation.describe(),
        "method `Generic` declares its own generic parameters"
    );
}

#[test]
fn check_module_wrapper_handles_empty_module() {
    let module = Module::new(None);
    let result = check_module(&module, &[], &TypeLayoutTable::default());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics for empty module: {:?}",
        result.diagnostics
    );
}
