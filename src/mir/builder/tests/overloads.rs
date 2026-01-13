use super::common::RequireExt;
use super::*;
use crate::mir::ConstValue;
use crate::mir::builder::SymbolIndex;
use crate::mir::data::{Operand, Terminator};

fn call_symbol<'a>(lowering: &'a LoweringResult, function_suffix: &str) -> &'a str {
    let function = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with(function_suffix))
        .require("missing target function");
    for block in &function.body.blocks {
        let Some(terminator) = &block.terminator else {
            continue;
        };
        let Terminator::Call { func, .. } = terminator else {
            continue;
        };
        let Operand::Const(constant) = func else {
            continue;
        };
        match &constant.value {
            ConstValue::Symbol(name) if name != "chic_rt_object_new" => {
                return name.as_str();
            }
            ConstValue::Symbol(_) => continue,
            other => panic!("expected symbol constant, found {other:?}"),
        }
    }
    panic!("expected call terminator in `{}`", function.name);
}

fn assert_call_symbol(lowering: &LoweringResult, function_suffix: &str, expected: &str) {
    let actual = call_symbol(lowering, function_suffix);
    assert_eq!(
        actual, expected,
        "expected `{expected}` for `{function_suffix}`, found `{actual}`"
    );
}

#[test]
fn reports_missing_matching_overload() {
    let source = r#"
namespace Demo;

public int Add(int x, int y) { return x + y; }

public int Use() { return Add(); }
"#;
    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    let has_overload = lowering.diagnostics.iter().any(|diag| {
        diag.message.contains("no overload of `Demo::Add` matches")
            || diag.message.contains("missing argument for parameter `x`")
    });
    assert!(
        has_overload,
        "expected overload resolution diagnostic, found: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn instance_methods_prefer_receiver_over_static_overload() {
    let source = r#"
namespace Demo;

public class Calculator
{
    public int Compute(int value) { return value; }
    public static int Compute(int value, int scale = 2) { return value * scale; }

    public int UseInstance() { return Compute(5); }
    public int UseStatic() { return Calculator.Compute(5); }
}
"#;

    let parsed = parse_module(source).require("parse");
    let index = SymbolIndex::build(&parsed.module);
    assert_eq!(
        index.function_count("Demo::Calculator::Compute"),
        Some(2),
        "expected two overloads registered for Demo::Calculator::Compute"
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let resolved = call_symbol(&lowering, "::UseInstance");
    assert!(
        resolved.starts_with("Demo::Calculator::Compute"),
        "expected `Demo::Calculator::Compute*` for `::UseInstance`, found `{resolved}`"
    );
    assert_call_symbol(&lowering, "::UseStatic", "Demo::Calculator::Compute#1");
}

#[test]
fn selects_overload_consuming_more_arguments() {
    let source = r#"
namespace Demo;

public class Math
{
    public static int Combine(int value) { return value; }
    public static int Combine(int value, int delta = 0) { return value + delta; }
    public static int Combine(int value, int delta, int scale = 1) { return value + (delta * scale); }
}

public class User
{
    public int UseOne() { return Math.Combine(1); }
    public int UseTwo() { return Math.Combine(1, 2); }
    public int UseThree() { return Math.Combine(1, 2, 3); }
}
"#;

    let parsed = parse_module(source).require("parse");
    let index = SymbolIndex::build(&parsed.module);
    assert_eq!(
        index.function_count("Demo::Math::Combine"),
        Some(3),
        "expected three overloads registered for Demo::Math::Combine"
    );
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    assert_call_symbol(&lowering, "::UseOne", "Demo::Math::Combine");
    assert_call_symbol(&lowering, "::UseTwo", "Demo::Math::Combine#1");
    assert_call_symbol(&lowering, "::UseThree", "Demo::Math::Combine#2");
}

#[test]
fn constructor_overloads_bind_to_matching_init() {
    let source = r#"
namespace Demo;

public class Widget
{
    public int Size;

    public init() { Size = 1; }
    public init(int size) { Size = size; }

    public static Widget MakeDefault() { return new Widget(); }
    public static Widget MakeSized() { return new Widget(8); }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    assert_call_symbol(&lowering, "::MakeDefault", "Demo::Widget::init#0");
    assert_call_symbol(&lowering, "::MakeSized", "Demo::Widget::init#1");
}
