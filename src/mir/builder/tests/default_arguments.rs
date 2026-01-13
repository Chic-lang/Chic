use super::common::RequireExt;
use super::*;
use crate::mir::ConstOperand;
use crate::mir::ConstValue;
use crate::mir::DefaultArgumentKind;

fn lower_source(source: &str) -> LoweringResult {
    let parsed = parse_module(source).require("parse module");
    lower_module(&parsed.module)
}

fn assert_const_int(operand: &Operand, expected: i128) {
    match operand {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Int(value) => assert_eq!(*value, expected),
            ConstValue::UInt(value) => assert_eq!(*value, expected as u128),
            other => panic!("expected integer constant `{expected}`, found {other:?}"),
        },
        other => panic!("expected integer operand `{expected}`, found {other:?}"),
    }
}

fn assert_const_symbol_name(operand: &Operand, expected_prefix: &str) {
    match operand {
        Operand::Const(ConstOperand {
            value: ConstValue::Symbol(symbol),
            ..
        }) => {
            assert!(
                symbol.starts_with(expected_prefix),
                "expected symbol starting with `{expected_prefix}`, found `{symbol}`"
            );
        }
        other => panic!("expected const symbol operand, found {other:?}"),
    }
}

#[test]
fn records_const_default_arguments_in_metadata() {
    let source = r#"
namespace Demo;

public class Sample
{
    public int Compute(int value = 5)
    {
        return value;
    }
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    assert_eq!(
        lowering.module.default_arguments.len(),
        1,
        "expected single default argument, found {:?}",
        lowering.module.default_arguments
    );
    let defaults = &lowering.module.default_arguments;
    assert_eq!(defaults.len(), 1, "expected single default argument record");
    let record = &defaults[0];
    assert_eq!(record.function, "Demo::Sample::Compute");
    assert_eq!(record.param_name, "value");
    assert_eq!(record.param_index, 0);
    match &record.value {
        DefaultArgumentKind::Const(ConstValue::Int(val)) => assert_eq!(*val, 5),
        other => panic!("expected const default, found {other:?}"),
    }
}

#[test]
fn records_multiple_default_arguments() {
    let source = r#"
namespace Demo;

public class Calculator
{
    public int Compute(int start, int delta = 5, int scale = 3)
    {
        return (start + delta) * scale;
    }
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let defaults = &lowering.module.default_arguments;
    assert_eq!(defaults.len(), 2, "defaults: {defaults:?}");
    let names: Vec<_> = defaults
        .iter()
        .map(|entry| entry.param_name.as_str())
        .collect();
    assert!(names.contains(&"delta"));
    assert!(names.contains(&"scale"));
}

#[test]
fn thunk_default_arguments_record_thunk_symbol() {
    let source = r#"
namespace Demo;

public class Generator
{
    private static int Next() { return 7; }

    public int Provide(int seed = Next())
    {
        return seed;
    }
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    assert_eq!(
        lowering.module.default_arguments.len(),
        1,
        "expected default argument metadata, found {:?}",
        lowering.module.default_arguments
    );
    let record = lowering
        .module
        .default_arguments
        .iter()
        .find(|entry| entry.function == "Demo::Generator::Provide")
        .expect("expected metadata entry for default argument");
    match &record.value {
        DefaultArgumentKind::Const(ConstValue::Int(value)) => {
            // Pure default arguments are const-evaluated now that compile-time evaluation
            // accepts pure functions without an explicit `constexpr` marker.
            assert_eq!(*value, 7);
        }
        DefaultArgumentKind::Thunk {
            symbol,
            metadata_count,
        } => {
            let thunk = lowering
                .module
                .functions
                .iter()
                .find(|func| func.name == *symbol)
                .expect("expected synthesized default argument thunk");
            assert_eq!(symbol, &thunk.name);
            assert_eq!(*metadata_count, 0);
        }
        other => panic!("expected thunk metadata, found {other:?}"),
    }
}

#[test]
fn constructor_default_arguments_emit_metadata() {
    let source = r#"
namespace Demo;

public class Widget
{
    public int Width;

    public init(int width = 13)
    {
        Width = width;
    }
}
"#;

    let lowering = lower_source(source);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    let defaults = &lowering.module.default_arguments;
    assert_eq!(defaults.len(), 1, "expected constructor default recorded");
    let record = &defaults[0];
    assert_eq!(record.function, "Demo::Widget::init");
    assert_eq!(record.param_name, "width");
    assert_eq!(record.param_index, 0);
    match &record.value {
        DefaultArgumentKind::Const(ConstValue::Int(val)) => assert_eq!(*val, 13),
        other => panic!("expected const default arg, found {other:?}"),
    }
}

#[test]
fn positional_call_inserts_default_arguments() {
    let source = r#"
namespace Sample;

public int Combine(int value, int delta = 5, int scale = 2)
{
    return (value + delta) * scale;
}

public int Use()
{
    return Combine(7);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    assert!(
        !lowering.module.default_arguments.is_empty(),
        "expected default arguments to be recorded"
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .expect("missing Sample::Use function");
    let call = use_func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => Some((func, args)),
            _ => None,
        })
        .expect("expected call terminator");
    assert_const_symbol_name(call.0, "Sample::Combine");
    assert_eq!(call.1.len(), 3, "expected three call arguments");
    assert_const_int(&call.1[0], 7);
    assert_const_int(&call.1[1], 5);
    assert_const_int(&call.1[2], 2);
}

#[test]
fn named_call_skips_optional_parameters() {
    let source = r#"
namespace Sample;

public int Combine(int start, int delta = 5, int scale = 2)
{
    return (start + delta) * scale;
}

public int Use()
{
    return Combine(4, scale: 3);
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    assert!(
        !lowering.module.default_arguments.is_empty(),
        "expected default arguments to be recorded"
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Use"))
        .expect("missing Sample::Use function");
    let call = use_func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => Some((func, args)),
            _ => None,
        })
        .expect("expected call terminator");
    assert_const_symbol_name(call.0, "Sample::Combine");
    assert_eq!(call.1.len(), 3);
    assert_const_int(&call.1[0], 4);
    assert_const_int(&call.1[1], 5);
    assert_const_int(&call.1[2], 3);
}

#[test]
fn constructor_call_inserts_default_arguments() {
    let source = r#"
namespace Sample;

public class Widget
{
    public int Width;
    public int Height;

    public init(int width = 9, int height = 2)
    {
        Width = width;
        Height = height;
    }
}

public Widget Build()
{
    return new Widget();
}
"#;

    let parsed = parse_module(source).require("parse module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
    assert!(
        !lowering.module.default_arguments.is_empty(),
        "expected default arguments to be recorded"
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Build"))
        .expect("missing Build function");
    let call = build_func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, args, .. }) => match func {
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                }) if symbol.starts_with("Sample::Widget::init") => Some((func, args)),
                _ => None,
            },
            _ => None,
        })
        .expect("expected constructor call terminator");
    assert_const_symbol_name(call.0, "Sample::Widget::init");
    assert_eq!(call.1.len(), 3);
    match &call.1[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert!(place.projection.is_empty(), "self operand should be direct")
        }
        Operand::Borrow(borrow) => {
            assert!(
                borrow.place.projection.is_empty(),
                "self operand should be direct"
            );
        }
        other => panic!("expected constructor self operand, found {other:?}"),
    }
    assert_const_int(&call.1[1], 9);
    assert_const_int(&call.1[2], 2);
}
