use super::{ArenaDiagnosticCase, ArenaDiagnosticFixture, Expectation, run_cases};
use crate::frontend::ast::Module;
use crate::mir::TypeLayoutTable;
use crate::typeck::ConstraintKind;
use crate::typeck::arena::{TypeCheckResult, TypeConstraint};

fn missing_random_effect(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let module = Module::new(Some("Random".into()));
    let constraints = vec![TypeConstraint::new(
        ConstraintKind::EffectEscape {
            function: "Random::Use".into(),
            effect: "random".into(),
        },
        None,
    )];
    let layouts = TypeLayoutTable::default();
    fixture.check_module(&module, &constraints, &layouts)
}

fn duplicated_rng_handle(fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
    let module = Module::new(Some("Random".into()));
    let constraints = vec![TypeConstraint::new(
        ConstraintKind::RandomDuplication {
            function: "Random::SplitWrong".into(),
        },
        None,
    )];
    let layouts = TypeLayoutTable::default();
    fixture.check_module(&module, &constraints, &layouts)
}

const CASES: &[ArenaDiagnosticCase] = &[
    ArenaDiagnosticCase::custom(
        "missing_effects_random_reports_rnd100",
        missing_random_effect,
        Expectation::contains(&["[RND100]"]),
    ),
    ArenaDiagnosticCase::custom(
        "rng_duplication_reports_rnd101",
        duplicated_rng_handle,
        Expectation::contains(&["[RND101]"]),
    ),
];

#[test]
fn random_diagnostics() {
    run_cases("random", CASES);
}
