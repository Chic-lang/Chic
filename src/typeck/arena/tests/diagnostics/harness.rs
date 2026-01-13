use super::fixtures::{parse_and_check, parse_lower_and_check, result_contains};
use crate::frontend::ast::Module;
use crate::mir::TypeLayoutTable;
use crate::typeck::arena::{TypeCheckResult, TypeConstraint, check_module as run_typeck_module};

pub(crate) struct ArenaDiagnosticFixture;

impl ArenaDiagnosticFixture {
    pub fn parse_and_check(&self, source: &'static str) -> TypeCheckResult {
        parse_and_check(source).1
    }

    pub fn parse_lower_and_check(&self, source: &'static str) -> TypeCheckResult {
        parse_lower_and_check(source).1
    }

    pub fn check_module(
        &self,
        module: &Module,
        constraints: &[TypeConstraint],
        layouts: &TypeLayoutTable,
    ) -> TypeCheckResult {
        run_typeck_module(module, constraints, layouts)
    }
}

type CustomCase = fn(&ArenaDiagnosticFixture) -> TypeCheckResult;

pub(crate) struct ArenaDiagnosticCase {
    pub name: &'static str,
    scenario: Scenario,
    expect: Expectation,
}

impl ArenaDiagnosticCase {
    pub const fn parsed(name: &'static str, source: &'static str, expect: Expectation) -> Self {
        Self {
            name,
            scenario: Scenario::Parsed { source },
            expect,
        }
    }

    pub const fn lowered(name: &'static str, source: &'static str, expect: Expectation) -> Self {
        Self {
            name,
            scenario: Scenario::Lowered { source },
            expect,
        }
    }

    pub const fn custom(name: &'static str, runner: CustomCase, expect: Expectation) -> Self {
        Self {
            name,
            scenario: Scenario::Custom(runner),
            expect,
        }
    }
}

enum Scenario {
    Parsed { source: &'static str },
    Lowered { source: &'static str },
    Custom(CustomCase),
}

impl Scenario {
    fn execute(&self, fixture: &ArenaDiagnosticFixture) -> TypeCheckResult {
        match self {
            Scenario::Parsed { source } => fixture.parse_and_check(source),
            Scenario::Lowered { source } => fixture.parse_lower_and_check(source),
            Scenario::Custom(runner) => runner(fixture),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Expectation {
    present: &'static [&'static str],
    absent: &'static [&'static str],
    require_clean: bool,
}

impl Expectation {
    pub const fn contains(needles: &'static [&'static str]) -> Self {
        Self {
            present: needles,
            absent: &[],
            require_clean: false,
        }
    }

    pub const fn lacks(needles: &'static [&'static str]) -> Self {
        Self {
            present: &[],
            absent: needles,
            require_clean: false,
        }
    }

    pub const fn clean() -> Self {
        Self {
            present: &[],
            absent: &[],
            require_clean: true,
        }
    }

    pub const fn with(present: &'static [&'static str], absent: &'static [&'static str]) -> Self {
        Self {
            present,
            absent,
            require_clean: false,
        }
    }

    fn assert(&self, result: &TypeCheckResult, suite: &str, case: &str) {
        for needle in self.present {
            assert!(
                result_contains(result, needle),
                "[arena diagnostics::{suite}] case `{case}` missing `{needle}`; diagnostics: {:?}",
                result.diagnostics
            );
        }
        for needle in self.absent {
            assert!(
                !result_contains(result, needle),
                "[arena diagnostics::{suite}] case `{case}` unexpectedly contained `{needle}`; diagnostics: {:?}",
                result.diagnostics
            );
        }
        if self.require_clean {
            assert!(
                result.diagnostics.is_empty(),
                "[arena diagnostics::{suite}] case `{case}` expected no diagnostics; found {:?}",
                result.diagnostics
            );
        }
    }
}

pub(crate) fn run_cases(suite: &str, cases: &[ArenaDiagnosticCase]) {
    let fixture = ArenaDiagnosticFixture;
    for case in cases {
        if std::env::var_os("TRACE_DIAGNOSTIC_CASES").is_some() {
            eprintln!("running case {suite}::{name}", name = case.name);
        }
        let result = case.scenario.execute(&fixture);
        case.expect.assert(&result, suite, case.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Diagnostic;

    fn fake_result(messages: &[&str]) -> TypeCheckResult {
        let mut result = TypeCheckResult::default();
        result.diagnostics = messages
            .iter()
            .map(|msg| Diagnostic::error((*msg).to_string(), None))
            .collect();
        result
    }

    #[test]
    fn expectation_supports_mixed_constraints() {
        let result = fake_result(&["alpha", "[CODE] detail"]);
        Expectation::with(&["alpha"], &["beta"]).assert(&result, "test", "mixed");
        Expectation::contains(&["[CODE] detail"]).assert(&result, "test", "contains");
        Expectation::lacks(&["gamma"]).assert(&result, "test", "lacks");
    }

    #[test]
    fn run_cases_executes_all_scenarios() {
        fn custom_case(_: &ArenaDiagnosticFixture) -> TypeCheckResult {
            fake_result(&["custom diagnostics"])
        }
        const CASES: &[ArenaDiagnosticCase] = &[
            ArenaDiagnosticCase::parsed(
                "parameter_default_violation",
                r#"
namespace Demo;

public class Widget
{
    public void Update(ref int width = 5) { }
}
"#,
                Expectation::contains(&["[TCK045]"]),
            ),
            ArenaDiagnosticCase::lowered(
                "borrow_checker_violation",
                r#"
namespace Demo;

public class Borrow
{
    public string ReturnRef(ref string value)
    {
        return value;
    }
}
"#,
                Expectation::contains(&["[CL0031]"]),
            ),
            ArenaDiagnosticCase::custom(
                "custom_runner_invoked",
                custom_case,
                Expectation::contains(&["custom diagnostics"]),
            ),
        ];
        run_cases("harness_smoke", CASES);
    }

    #[test]
    fn expectation_clean_enforces_empty_diagnostics() {
        Expectation::clean().assert(&TypeCheckResult::default(), "test", "clean");
    }
}
