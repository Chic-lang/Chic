use chic::frontend::parser::parse_module;
use chic::mir::{LoweringResult, TypeLayoutTable, lower_module};
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

fn contains_typeck_diagnostic(report: &CompilationReport, needle: &str) -> bool {
    report
        .typeck
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains(needle))
}

fn contains_lowering_diagnostic(report: &CompilationReport, needle: &str) -> bool {
    report
        .lowering
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains(needle))
}

#[test]
fn using_namespace_allows_simple_type_reference() {
    let report = run_check(
        r#"
import Utils.Collections;

namespace Utils
{
    namespace Collections
    {
        public struct Vec<T> { }
    }
}

namespace UsingNamespace
{
    public struct Holder
    {
        public Vec<int> Data;
    }
}
"#,
    );
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
fn using_alias_expands_type_references() {
    let report = run_check(
        r#"
import Col = Utils.Collections;

namespace Utils
{
    namespace Collections
    {
        public struct Vec<T> { }
    }
}

namespace UsingAlias
{
    public struct Holder
    {
        public Col.Vec<int> Data;
    }
}
"#,
    );
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
fn ambiguous_using_reports_diagnostic() {
    let report = run_check(
        r#"
import One;
import Two;

namespace One
{
    public struct Widget { }
}

namespace Two
{
    public struct Widget { }
}

namespace Ambiguous
{
    public struct Holder
    {
        public Widget Value;
    }
}
"#,
    );
    assert!(
        contains_typeck_diagnostic(&report, "resolves to multiple candidates"),
        "expected ambiguity diagnostic, got {:?}",
        report.typeck.diagnostics
    );
}

#[test]
fn global_using_namespace_visible_everywhere() {
    let report = run_check(
        r#"
global import Utils.Collections;

namespace Utils
{
    namespace Collections
    {
        public struct Vec<T> { }
    }
}

namespace Consumer
{
    public struct Holder
    {
        public Vec<int> Data;
    }
}
"#,
    );
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
fn conflicting_global_alias_reports_error() {
    let report = run_check(
        r#"
global import Alias = One;
import Alias = Two;

namespace One
{
    public struct Missing { }
}

namespace Two
{
    public struct Widget { }
}

namespace Consumer
{
    public struct Holder
    {
        public Alias.Widget Data;
    }
}
"#,
    );
    assert!(
        contains_typeck_diagnostic(&report, "conflicts with existing alias"),
        "expected alias conflict diagnostic, got {:?}",
        report.typeck.diagnostics
    );
    assert!(
        report.typeck.diagnostics.iter().any(|diag| diag
            .code
            .as_ref()
            .map(|code| code.code.as_str())
            == Some("E0G03")),
        "expected alias conflict diagnostic code E0G03"
    );
}

#[test]
fn global_using_static_available_everywhere() {
    let report = run_check(
        r#"
global import static Utilities.Numbers;

namespace Utilities
{
    public class Numbers
    {
        public static int Seed = 7;

        public static int Double(int value)
        {
            return value * 2;
        }
    }
}

namespace Consumers
{
    public struct Calculator
    {
        public int Compute()
        {
            return Double(Seed);
        }
    }
}
"#,
    );
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
fn global_and_local_using_static_ambiguity_reports_error() {
    let report = run_check(
        r#"
global import static First.Values;
import static Second.Values;

namespace First
{
    public class Values
    {
        public static int Item = 1;
    }
}

namespace Second
{
    public class Values
    {
        public static int Item = 2;
    }
}

namespace Test
{
    public struct Consumer
    {
        public int result;

        public void Init()
        {
            result = Item;
        }
    }
}
"#,
    );
    assert!(
        contains_lowering_diagnostic(&report, "ambiguous between"),
        "expected ambiguity diagnostic, got {:?}",
        report.lowering.diagnostics
    );
}

#[test]
fn using_static_imports_fields_and_methods() {
    let report = run_check(
        r#"
import static Utilities.Numbers;

namespace Utilities
{
    public class Numbers
    {
        public static int Seed = 5;

        public static int Increment(int value)
        {
            return value + 1;
        }
    }
}

namespace UsingStatic
{
    public struct Calculator
    {
        public int Compute()
        {
            return Increment(Seed);
        }
    }
}
"#,
    );
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
fn ambiguous_using_static_reports_error() {
    let report = run_check(
        r#"
import static First.Values;
import static Second.Values;

namespace First
{
    public class Values
    {
        public static int Item = 1;
    }
}

namespace Second
{
    public class Values
    {
        public static int Item = 2;
    }
}

namespace Test
{
    public struct Consumer
    {
        public int result;

        public void Init()
        {
            result = Item;
        }
    }
}
"#,
    );
    assert!(
        contains_lowering_diagnostic(&report, "ambiguous between"),
        "expected ambiguity diagnostic, got {:?}",
        report.lowering.diagnostics
    );
}
