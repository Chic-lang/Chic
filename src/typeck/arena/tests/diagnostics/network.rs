use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::Module;
use crate::mir::TypeLayoutTable;
use crate::typeck::ConstraintKind;
use crate::typeck::arena::{TypeCheckResult, TypeConstraint};

fn missing_network_effect(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let module = Module::new(Some("Network".into()));
    let constraints = vec![TypeConstraint::new(
        ConstraintKind::EffectEscape {
            function: "Network::Send".into(),
            effect: "network".into(),
        },
        None,
    )];
    let layouts = TypeLayoutTable::default();
    fixture.check_module(&module, &constraints, &layouts)
}

const CASES: &[ArenaDiagnosticCase] = &[ArenaDiagnosticCase::custom(
    "missing_effects_network_reports_net100",
    missing_network_effect,
    Expectation::contains(&["[NET100]"]),
)];

#[test]
fn network_diagnostics() {
    run_cases("network", CASES);
}
