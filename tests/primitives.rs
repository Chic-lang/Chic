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

#[test]
fn primitives_resolve_as_types() {
    let report = run_check(
        r#"
namespace PrimitiveTypes
{
    public struct Vec<T> { }
    public struct Option<T> { }
    public struct Span<T> { }

    public struct Holder
    {
        public int Number;
        public string Name;
        public Vec<int> Numbers;
        public Option<string> Maybe;
        public Span<byte> Buffer;
    }

    public class Demo
    {
        public int Identity()
        {
            let value = 1;
            return value;
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
fn primitive_spellings_can_be_identifiers() {
    let report = run_check(
        r#"
namespace PrimitiveIdentifiers
{
    public struct Names
    {
        public int int;
        public bool bool;
        public string string;
    }

    public struct Shadow
    {
        public int Echo(int int, bool bool)
        {
            let string = int;
            let decimal = 3;
            return decimal;
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
