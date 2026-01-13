use super::common::RequireExt;
use super::*;
use crate::mir::ConstOperand;
use crate::mir::ConstValue;
use crate::mir::data::Operand;

fn assert_const_symbol(operand: &Operand, expected: &str) {
    match operand {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Symbol(name) => assert_eq!(name, expected),
            other => panic!("expected symbol constant `{expected}`, found {other:?}"),
        },
        other => panic!("expected constant operand `{expected}`, found {other:?}"),
    }
}

fn assert_const_int(operand: &Operand, expected: i128) {
    match operand {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Int(value) => assert_eq!(*value, expected),
            ConstValue::UInt(value) => assert_eq!(*value, expected as u128),
            other => panic!("expected integer constant `{expected}`, found {other:?}"),
        },
        other => panic!("expected constant integer operand, found {other:?}"),
    }
}
use crate::mir::data::Terminator;

#[test]
fn reorders_named_arguments_for_free_function() {
    let source = r#"
namespace Sample;

public int Add(int left, int right)
{
    return left + right;
}

public int Use()
{
    return Add(right: 2, left: 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let Terminator::Call {
        func,
        args,
        arg_modes: _,
        ..
    } = use_func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator")
    else {
        unreachable!();
    };

    assert_const_symbol(&func, "Sample::Add");
    assert_eq!(args.len(), 2);
    assert_const_int(&args[0], 1);
    assert_const_int(&args[1], 2);
}

#[test]
fn supports_positional_then_named_arguments() {
    let source = r#"
namespace Sample;

public int Combine(int prefix, int value, int scale)
{
    return (prefix * scale) + value;
}

public int Use()
{
    return Combine(1, value: 2, scale: 3);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let Terminator::Call {
        func,
        args,
        arg_modes: _,
        ..
    } = use_func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator")
    else {
        unreachable!();
    };

    assert_const_symbol(&func, "Sample::Combine");
    assert_eq!(args.len(), 3);
    assert_const_int(&args[0], 1);
    assert_const_int(&args[1], 2);
    assert_const_int(&args[2], 3);
}

#[test]
fn reorders_named_arguments_for_instance_method() {
    let source = r#"
namespace Sample;

public class Calculator
{
    public int Combine(int x, int y)
    {
        return x + y;
    }
}

public int Use(Calculator calc)
{
    return calc.Combine(y: 3, x: 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let Terminator::Call {
        func,
        args,
        arg_modes: _,
        ..
    } = use_func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator")
    else {
        unreachable!();
    };

    assert_const_symbol(&func, "Sample::Calculator::Combine");
    assert_eq!(args.len(), 3);
    match &args[0] {
        Operand::Copy(place) => assert_eq!(place.local.0, 1, "receiver should be first argument"),
        other => panic!("expected receiver operand, found {other:?}"),
    }
    assert_const_int(&args[1], 1);
    assert_const_int(&args[2], 3);
}

#[test]
fn reorders_named_arguments_for_constructor_invocation() {
    let source = r#"
namespace Sample;

public class Point
{
    public int X;
    public int Y;

    public init(int x, int y)
    {
        self.X = x;
        self.Y = y;
    }
}

public Point Build()
{
    return Point(y: 2, x: 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let build_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Build"))
        .require("missing Build function");
    let Terminator::Call {
        func,
        args,
        arg_modes: _,
        ..
    } = build_func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator")
    else {
        unreachable!();
    };

    match func {
        Operand::Const(ConstOperand {
            value: ConstValue::Symbol(name),
            ..
        }) => {
            assert!(
                name.starts_with("Sample::Point::init"),
                "unexpected constructor symbol: {name}"
            );
        }
        other => panic!("expected constructor symbol operand, found {other:?}"),
    }
    assert_eq!(args.len(), 3);
    match &args[0] {
        Operand::Copy(place) | Operand::Move(place) => {
            assert!(
                place.projection.is_empty(),
                "constructor self operand should not include projections"
            );
        }
        Operand::Borrow(borrow) => {
            assert!(
                borrow.place.projection.is_empty(),
                "constructor self operand should not include projections"
            );
        }
        other => panic!("expected constructor self operand, found {other:?}"),
    }
    assert_const_int(&args[1], 1);
    assert_const_int(&args[2], 2);
}

#[test]
fn reorders_named_arguments_for_extern_function() {
    let source = r#"
namespace Sample;

@extern("C")
public static extern void native_write(int fd, str message);

public void Use()
{
    unsafe { native_write(message: "ok", fd: 1); }
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let use_func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Use"))
        .require("missing Use function");
    let Terminator::Call {
        func,
        args,
        arg_modes: _,
        ..
    } = use_func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator")
    else {
        unreachable!();
    };

    assert_const_symbol(&func, "Sample::native_write");
    assert_eq!(args.len(), 2);
    assert_const_int(&args[0], 1);
    match &args[1] {
        Operand::Const(constant) => match &constant.value {
            ConstValue::Str { value, .. } => assert_eq!(value, "ok"),
            other => panic!("expected string literal, got {other:?}"),
        },
        other => panic!("expected string operand, found {other:?}"),
    }
}

#[test]
fn reports_unknown_named_argument() {
    let source = r#"
namespace Sample;

public void Print(int number) { }

public void Use()
{
    Print(value: 1);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown named argument 'value'")),
        "expected unknown argument diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn reports_positional_after_named_argument() {
    let source = r#"
namespace Sample;

public int Add(int left, int right) { return left + right; }

public int Use()
{
    return Add(left: 1, 2);
}
"#;

    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("positional arguments cannot follow named arguments")),
        "expected positional-after-named diagnostic, found {:?}",
        lowering.diagnostics
    );
}
