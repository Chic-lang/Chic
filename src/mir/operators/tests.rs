use super::*;

fn make_conversion(
    kind: OperatorKind,
    source: &str,
    target: &str,
    function: &str,
) -> OperatorOverload {
    OperatorOverload {
        kind,
        params: vec![source.to_string()],
        result: target.to_string(),
        function: function.to_string(),
    }
}

#[test]
fn resolves_implicit_conversion() {
    let mut registry = OperatorRegistry::default();
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Implicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Implicit_Value",
        ),
    );

    match registry.resolve_conversion("Numbers::Input", "Numbers::Value", false) {
        ConversionResolution::Found(overload) => {
            assert_eq!(overload.function, "Numbers::Value::op_Implicit_Value");
        }
        other => panic!("expected implicit conversion, found {other:?}"),
    }
}

#[test]
fn prefers_implicit_conversion_when_explicit_also_exists() {
    let mut registry = OperatorRegistry::default();
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Implicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Implicit_Value",
        ),
    );
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Explicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Explicit_Value",
        ),
    );

    match registry.resolve_conversion("Numbers::Input", "Numbers::Value", true) {
        ConversionResolution::Found(overload) => {
            assert_eq!(overload.function, "Numbers::Value::op_Implicit_Value");
        }
        other => panic!("expected implicit conversion, found {other:?}"),
    }
}

#[test]
fn resolves_explicit_conversion_when_permitted() {
    let mut registry = OperatorRegistry::default();
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Explicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Explicit_Value",
        ),
    );

    match registry.resolve_conversion("Numbers::Input", "Numbers::Value", true) {
        ConversionResolution::Found(overload) => {
            assert_eq!(overload.function, "Numbers::Value::op_Explicit_Value");
        }
        other => panic!("expected explicit conversion, found {other:?}"),
    }
}

#[test]
fn reports_absent_implicit_conversion_with_explicit_candidates() {
    let mut registry = OperatorRegistry::default();
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Explicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Explicit_Value",
        ),
    );

    match registry.resolve_conversion("Numbers::Input", "Numbers::Value", false) {
        ConversionResolution::None {
            explicit_candidates,
        } => {
            assert_eq!(explicit_candidates.len(), 1);
            assert_eq!(
                explicit_candidates[0].function,
                "Numbers::Value::op_Explicit_Value"
            );
        }
        other => panic!("expected explicit candidate listing, found {other:?}"),
    }
}

#[test]
fn reports_ambiguous_implicit_conversions() {
    let mut registry = OperatorRegistry::default();
    registry.register(
        "Numbers::Value",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Implicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Value::op_Implicit_Value",
        ),
    );
    registry.register(
        "Numbers::Input",
        make_conversion(
            OperatorKind::Conversion(ConversionKind::Implicit),
            "Numbers::Input",
            "Numbers::Value",
            "Numbers::Input::op_Implicit_Value",
        ),
    );

    match registry.resolve_conversion("Numbers::Input", "Numbers::Value", false) {
        ConversionResolution::Ambiguous(candidates) => {
            assert_eq!(candidates.len(), 2);
            let names = candidates
                .iter()
                .map(|overload| overload.function.as_str())
                .collect::<Vec<_>>();
            assert!(
                names.contains(&"Numbers::Value::op_Implicit_Value")
                    && names.contains(&"Numbers::Input::op_Implicit_Value"),
                "expected both overloads, found {names:?}"
            );
        }
        other => panic!("expected ambiguous implicit conversions, found {other:?}"),
    }
}
