use super::*;
use crate::mir::builder::tests::common::RequireExt;
use crate::mir::data::{Operand, PendingOperand, Terminator};

#[test]
fn lower_property_generates_accessor_functions() {
    let source = r"
namespace Demo;

public class Counter
{
    public int Value { get; set; }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );

    let function_names: Vec<&str> = lowering
        .module
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    assert!(function_names.contains(&"Demo::Counter::get_Value"));
    assert!(function_names.contains(&"Demo::Counter::set_Value"));

    let layout = lowering
        .module
        .type_layouts
        .types
        .get("Demo::Counter")
        .and_then(|entry| match entry {
            TypeLayout::Class(class) => Some(class),
            _ => None,
        })
        .expect("missing class layout for Demo::Counter");
    assert!(
        layout
            .fields
            .iter()
            .any(|field| field.name == "__property_Value")
    );
}

#[test]
fn property_assignment_lowers_to_setter_call() {
    let source = r"
namespace Demo;

public class Counter
{
    public int Value { get; set; }

    public void Assign(ref this, int value)
    {
        this.Value = value;
    }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );

    let assign = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Counter::Assign")
        .expect("missing assign function");

    let call = assign
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                func,
                args,
                arg_modes: _,
                ..
            }) => Some((func, args)),
            _ => None,
        })
        .expect("expected property setter call terminator");

    match call.0 {
        Operand::Pending(PendingOperand { repr, .. }) => {
            assert_eq!(repr, "Demo::Counter::set_Value");
        }
        other => panic!("expected pending operand for setter call, found {other:?}"),
    }
    assert_eq!(call.1.len(), 2, "expected receiver and value arguments");
    assert!(matches!(call.1[0], Operand::Copy(_)));
}

#[test]
fn init_only_property_requires_constructor_context() {
    let source = r"
namespace Demo;

public class Sample
{
    public int Total { get; init; }

    public void Update(ref this, int value)
    {
        this.Total = value;
    }
}
";

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("init-only property")),
        "expected diagnostic complaining about init-only setter usage, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn ref_this_method_call_accepts_implicit_receiver() {
    let source = r"
namespace Demo;

public class Counter
{
    private int backing;
    public int Backing { get => backing; set => backing = value; }

    public int Run(ref this)
    {
        Backing = 5;
        return Backing;
    }
}

public int Main()
{
    var counter = new Counter();
    return counter.Run();
}
";

    let parsed = parse_module(source).require("parse ref-this module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );
}
