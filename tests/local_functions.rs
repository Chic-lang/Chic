use chic::frontend::parser::parse_module;
use chic::mir::{LoweringResult, MirFunction, TypeLayoutTable, lower_module};
use chic::typeck::{TypeCheckResult, check_module};

struct CompilationReport {
    typeck: TypeCheckResult,
    lowering: LoweringResult,
}

fn run_check(source: &str) -> CompilationReport {
    let parse = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    assert!(
        parse.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parse.diagnostics
    );
    let module = parse.module;
    let typeck = check_module(&module, &[], &TypeLayoutTable::default());
    let lowering = lower_module(&module);
    CompilationReport { typeck, lowering }
}

fn find_function<'a>(report: &'a CompilationReport, name: &str) -> &'a MirFunction {
    report
        .lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

fn assert_no_diagnostics(report: &CompilationReport) {
    assert!(
        report.typeck.diagnostics.is_empty(),
        "unexpected typeck diagnostics: {:?}",
        report.typeck.diagnostics
    );
    assert!(
        report.lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        report.lowering.diagnostics
    );
}

#[test]
fn capturing_local_function_introduces_environment_param() {
    let report = run_check(
        r#"
namespace Samples;

public static class Program
{
    public static int Run(int seed)
    {
        function int Add(int delta)
        {
            return seed + delta;
        }

        return Add(12);
    }
}
"#,
    );
    assert_no_diagnostics(&report);

    let nested = find_function(&report, "Samples::Program::Run::local$0::Add");
    assert_eq!(
        nested.signature.params.len(),
        2,
        "expected capture plus user argument"
    );
    assert_eq!(
        nested.signature.params[0].canonical_name(),
        "int",
        "capture should lower to enclosing seed type"
    );
    assert_eq!(
        nested.signature.params[1].canonical_name(),
        "int",
        "user parameter should remain untouched"
    );
    assert_eq!(
        nested.signature.ret.canonical_name(),
        "int",
        "return type should match local function declaration"
    );
}

#[test]
fn local_function_inherits_parent_generics() {
    let report = run_check(
        r#"
namespace Samples;

public static class Program
{
    public static T Identity<T>(T value)
    {
        function T Local(T inner)
        {
            return inner;
        }

        return Local(value);
    }
}
"#,
    );
    assert_no_diagnostics(&report);

    let nested = find_function(&report, "Samples::Program::Identity::local$0::Local");
    assert_eq!(
        nested.signature.params.len(),
        1,
        "non-capturing local function should not receive hidden environment arguments"
    );
    assert_eq!(
        nested.signature.params[0].canonical_name(),
        "T",
        "local function parameter should reference parent generic parameter"
    );
    assert_eq!(
        nested.signature.ret.canonical_name(),
        "T",
        "return type should continue to reference parent generic parameter"
    );
}
