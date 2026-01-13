use super::planner::LowerPlanner;
use super::{
    ModuleLowering, lower_module, lower_module_with_units, lower_module_with_units_and_hook,
};
use crate::frontend::conditional::ConditionalDefines;
use crate::frontend::parser::{parse_module, parse_module_with_defines};
use crate::primitives::{PrimitiveDescriptor, PrimitiveKind};

#[test]
fn lower_module_runs_planner_and_executor() {
    let parsed = parse_module("public void Run() { }").expect("parse module");

    let result = lower_module(&parsed.module);

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.module.functions.len(),
        1,
        "expected lowered function"
    );
    assert!(
        !result.pass_metrics.is_empty(),
        "expected pass metrics to be recorded by the pipeline"
    );
}

#[test]
fn lower_module_with_units_accepts_item_units() {
    let parsed = parse_module(
        r#"
public void A() { }
public void B() { }
"#,
    )
    .expect("parse module");

    let result = lower_module_with_units(&parsed.module, Some(&[0, 1]), None);

    assert_eq!(
        result.module.functions.len(),
        2,
        "expected both functions lowered"
    );
    // Even when unit slicing is disabled, this ensures the planner accepted the units list.
    assert!(
        result.unit_slices.is_empty() || result.unit_slices.len() == 2,
        "unit slices should be empty or reflect provided units"
    );
}

#[test]
fn planner_validates_unit_length_in_debug() {
    let parsed = parse_module("public void Only() { }").expect("parse module");
    let planner = LowerPlanner::new();
    let plan = planner.plan(&parsed.module, Some(&[0]));
    assert_eq!(plan.item_units.as_deref(), Some(&[0][..]));
}

#[test]
fn module_lowering_default_state_is_empty() {
    let lowering = ModuleLowering::default();
    assert!(lowering.diagnostics.is_empty());
    assert!(lowering.exports.is_empty());
}

#[test]
fn cfg_attributes_prune_lowered_functions() {
    let mut defines = ConditionalDefines::default();
    defines.set_bool("DEBUG", false);
    defines.set_bool("RELEASE", true);
    let parsed = parse_module_with_defines(
        r#"
@cfg(DEBUG)
public void DebugOnly() { }
@cfg(RELEASE)
public void ReleaseOnly() { }
"#,
        &defines,
    )
    .expect("parse module");
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|diag| !matches!(diag.severity, crate::frontend::diagnostics::Severity::Error)),
        "expected cfg filtering to keep diagnostics clean: {:?}",
        parsed.diagnostics
    );

    let result = lower_module(&parsed.module);
    assert_eq!(
        result.module.functions.len(),
        1,
        "only the active function should be lowered"
    );
    assert!(
        result
            .module
            .functions
            .iter()
            .any(|func| func.name.ends_with("ReleaseOnly")),
        "expected ReleaseOnly to be lowered"
    );
}

fn hook_int40() -> Vec<PrimitiveDescriptor> {
    vec![PrimitiveDescriptor {
        primitive_name: "int40".to_string(),
        aliases: vec!["i40".to_string()],
        kind: PrimitiveKind::Int {
            bits: 40,
            signed: true,
            pointer_sized: false,
        },
        c_type: Some("int40_t".to_string()),
        std_wrapper_type: None,
        span: None,
    }]
}

fn hook_conflicting_int() -> Vec<PrimitiveDescriptor> {
    vec![PrimitiveDescriptor {
        primitive_name: "int".to_string(),
        aliases: vec![],
        kind: PrimitiveKind::Int {
            bits: 64,
            signed: true,
            pointer_sized: false,
        },
        c_type: None,
        std_wrapper_type: None,
        span: None,
    }]
}

fn hook_invalid_bits() -> Vec<PrimitiveDescriptor> {
    vec![PrimitiveDescriptor {
        primitive_name: "int7".to_string(),
        aliases: vec![],
        kind: PrimitiveKind::Int {
            bits: 7,
            signed: true,
            pointer_sized: false,
        },
        c_type: None,
        std_wrapper_type: None,
        span: None,
    }]
}

#[test]
fn extra_primitive_hook_is_opt_in() {
    let parsed = parse_module("public void Run() { }").expect("parse module");

    let without_hook = lower_module_with_units_and_hook(&parsed.module, None, None, None);
    assert!(
        without_hook
            .module
            .primitive_registry
            .lookup_by_name("int40")
            .is_none(),
        "int40 should not be registered without a hook"
    );

    let with_hook = lower_module_with_units_and_hook(&parsed.module, None, None, Some(hook_int40));
    let id = with_hook.module.primitive_registry.lookup_by_name("int40");
    assert!(id.is_some(), "expected int40 from hook to be registered");
    let (size, align) = with_hook
        .module
        .primitive_registry
        .size_align_for_name("int40", 8, 8)
        .expect("size/align for int40");
    assert_eq!(size, 5);
    assert_eq!(align, 5);
}

#[test]
fn extra_primitive_hook_conflicts_with_builtin() {
    let parsed = parse_module("public void Run() { }").expect("parse module");

    let result =
        lower_module_with_units_and_hook(&parsed.module, None, None, Some(hook_conflicting_int));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("conflicts with built-in primitive")),
        "expected builtin conflict diagnostic, got {:?}",
        result.diagnostics
    );
    let kind = result
        .module
        .primitive_registry
        .kind_for_name("int")
        .expect("builtin int should remain registered");
    match kind {
        PrimitiveKind::Int { bits, signed, .. } => {
            assert_eq!(*bits, 32);
            assert!(*signed);
        }
        other => panic!("unexpected primitive kind for int: {other:?}"),
    }
}

#[test]
fn chic_primitive_overrides_hook_descriptor() {
    let parsed = parse_module(
        r#"
@primitive(
    primitive = "int40",
    kind = "int",
    bits = 48,
    signed = true,
    aliases = ["int40", "i40"]
)
public readonly struct Int40 { }
"#,
    )
    .expect("parse module");

    let result = lower_module_with_units_and_hook(&parsed.module, None, None, Some(hook_int40));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("extra primitive hook")),
        "expected conflict diagnostic when @primitive overrides hook: {:?}",
        result.diagnostics
    );
    let kind = result
        .module
        .primitive_registry
        .kind_for_name("int40")
        .expect("int40 should be registered from @primitive");
    match kind {
        PrimitiveKind::Int { bits, signed, .. } => {
            assert_eq!(*bits, 48);
            assert!(*signed);
        }
        other => panic!("unexpected kind for int40: {other:?}"),
    }
}

#[test]
fn hook_returning_invalid_descriptor_is_reported() {
    let parsed = parse_module("public void Run() { }").expect("parse module");
    let result =
        lower_module_with_units_and_hook(&parsed.module, None, None, Some(hook_invalid_bits));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("width")),
        "expected width diagnostic for invalid hook descriptor: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .module
            .primitive_registry
            .lookup_by_name("int7")
            .is_none(),
        "invalid hook descriptor should not be registered"
    );
}
