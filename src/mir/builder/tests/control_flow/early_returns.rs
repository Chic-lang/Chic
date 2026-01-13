use super::prelude::*;

#[test]
fn result_propagation_uses_from_conversion_when_available() {
    let source = r"
namespace Demo;

public struct ErrorA { }

public class ErrorB
{
    public static extern ErrorB from(ErrorA value);
}

public enum IntErrorAResult
{
    Ok,
    Err { public ErrorA Error; }
}

public enum IntErrorBResult
{
    Ok,
    Err { public ErrorB Error; }
}

public IntErrorBResult Convert(IntErrorAResult input)
{
    input?;
    return IntErrorBResult.Ok;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Convert"))
        .require("missing Convert function");
    let body = &func.body;

    let call_block = body
        .blocks
        .iter()
        .find(|block| match &block.terminator {
            Some(Terminator::Call { func, .. }) => match func {
                Operand::Pending(pending) => pending.repr.contains("ErrorB::from"),
                _ => false,
            },
            _ => false,
        })
        .require("expected call to ErrorB::from in err path");

    let target = match &call_block.terminator {
        Some(Terminator::Call { target, .. }) => *target,
        _ => unreachable!(),
    };
    let continuation = &body.blocks[target.0];
    let converts_err = continuation.statements.iter().any(|stmt| {
        if let MirStatementKind::Assign {
            value:
                Rvalue::Aggregate {
                    kind: AggregateKind::Adt { .. },
                    fields,
                },
            ..
        } = &stmt.kind
        {
            fields.len() == 1
        } else {
            false
        }
    });
    assert!(
        converts_err,
        "expected converted err payload to be wrapped: {:?}",
        continuation.statements
    );
}

#[test]
fn result_propagation_requires_result_types() {
    let source = r"
public enum IntResult
{
    Ok { public int Value; },
    Err { public int Error; }
}

public int Demo(IntResult value)
{
    return value?;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("requires the enclosing function to return `Result")),
        "expected diagnostic about function return type: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn result_propagation_reports_missing_conversion() {
    let source = r"
namespace Demo;

public enum IntErrorAResult
{
    Ok,
    Err { public ErrorA Error; }
}

public enum IntErrorBResult
{
    Ok,
    Err { public ErrorB Error; }
}

public struct ErrorA { }

public struct ErrorB { }

public IntErrorBResult Convert(IntErrorAResult input)
{
    input?;
    return IntErrorBResult.Ok;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("cannot convert error type `Demo::ErrorA` to `Demo::ErrorB`")),
        "expected missing conversion diagnostic: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn closure_to_fn_ptr_non_capturing_returns_symbol() {
    let source = r"
namespace Demo;

public delegate int Transform(int value);

public Transform Make()
{
    let closure = (int value) => value + 1;
    return closure.to_fn_ptr();
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Make"))
        .require("missing Make function");

    assert!(
        func.body.blocks[0].statements.iter().any(|stmt| matches!(
            stmt.kind,
            MirStatementKind::Assign {
                value: Rvalue::Aggregate { .. },
                ..
            }
        )),
        "expected aggregate assignment for closure pointer"
    );
}

#[test]
fn closure_to_fn_ptr_capturing_invokes_runtime_adapter() {
    let source = r"
namespace Demo;

public delegate int Transform(int value);

public Transform Make()
{
    let delta = 2;
    let closure = (int value) => value + delta;
    return closure.to_fn_ptr();
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let function = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Make"))
        .require("missing Make function");

    let adapter_symbol = function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match &stmt.kind {
            StatementKind::Assign {
                value: Rvalue::Aggregate { fields, .. },
                ..
            } => fields.iter().find_map(|field| {
                if let Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                }) = field
                {
                    symbol.contains("to_fn_ptr#").then_some(symbol.clone())
                } else {
                    None
                }
            }),
            _ => None,
        })
        .expect("expected synthesized adapter reference");
    assert!(
        adapter_symbol.contains("to_fn_ptr#"),
        "expected synthesized adapter, got {adapter_symbol}"
    );
}

#[test]
#[ignore = "function pointer type syntax is not yet accepted by the parser"]
fn closure_to_fn_ptr_capturing_requires_explicit_conversion() {
    let source = r"
namespace Demo;

public delegate int Transform(int value);

public Transform Make()
{
    let delta = 2;
    Transform pointer = (int value) => value + delta;
    return pointer;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("call `.to_fn_ptr()` to convert it to a function pointer")),
        "expected diagnostic about missing `.to_fn_ptr()`, got {:?}",
        lowering.diagnostics
    );
}

#[test]
fn result_propagation_requires_result_operand() {
    let source = r"
namespace Demo;

public enum IntResult
{
    Ok,
    Err { public Error Error; }
}

public struct Error { }

public IntResult Demo(int value)
{
    return value?;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("`?` requires an enum `Result<T, E>` operand")),
        "expected diagnostic about operand type: {:?}",
        lowering.diagnostics
    );
}

#[test]
fn nested_try_catch_returns_are_preserved() {
    let source = r#"
namespace Control;

public class Exception { }

public int Handle(int value)
{
    try
    {
        try
        {
            return value;
        }
        catch (Exception inner)
        {
            return 1;
        }
    }
    catch (Exception outer)
    {
        return -1;
    }
}
"#;
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics in nested try/catch: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Handle"))
        .require("missing Handle function");
    let body = &func.body;
    assert_eq!(
        body.exception_regions.len(),
        2,
        "nested try/catch should introduce two exception regions"
    );
    let graph = GraphAssert::new(body);
    for region in &body.exception_regions {
        for catch in &region.catches {
            graph.expect_return(catch.body.0);
        }
    }
    let return_blocks = body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Some(Terminator::Return)))
        .count();
    assert!(
        return_blocks >= 3,
        "expected at least three return blocks for nested try/catch"
    );
}
