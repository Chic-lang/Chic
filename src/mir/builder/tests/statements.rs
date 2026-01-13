use super::common::RequireExt;
use super::*;

#[test]
fn lowers_default_literal_to_zero_init() {
    let source = r"
namespace Sample;

public int UseDefault()
{
    return default;
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
        .find(|f| f.name == "Sample::UseDefault")
        .expect("missing UseDefault function");
    let zero_init_present = func.body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::ZeroInit { .. }))
    });
    assert!(
        zero_init_present,
        "default literal should lower to ZeroInit statements"
    );
    verify_body(&func.body).require("body verification");
}

#[test]
fn lowers_default_of_type_to_zero_init() {
    let source = r"
namespace Sample;

public ReadOnlySpan<byte> Make()
{
    return default(ReadOnlySpan<byte>);
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
        .find(|f| f.name == "Sample::Make")
        .expect("missing Make function");
    let zero_init_present = func.body.blocks.iter().any(|block| {
        block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::ZeroInit { .. }))
    });
    assert!(zero_init_present, "default(T) should lower to ZeroInit");
    verify_body(&func.body).require("body verification");
}

#[test]
fn pin_attribute_marks_local_and_initializer_assignment() {
    let source = r"
namespace Sample;

public int Acquire()
{
    return 1;
}

public void Create()
{
    @pin var resource = Acquire();
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
        .find(|f| f.name == "Sample::Create")
        .expect("missing Create function");
    let resource_index = func
        .body
        .locals
        .iter()
        .position(|local| local.name.as_deref() == Some("resource"))
        .expect("resource local not declared");
    let resource_local = &func.body.locals[resource_index];
    assert!(
        resource_local.is_pinned,
        "resource should remain pinned after lowering"
    );

    let mut initializer_value = None;
    for block in &func.body.blocks {
        for statement in &block.statements {
            if let MirStatementKind::Assign { place, value } = &statement.kind {
                if place.local.0 == resource_index {
                    initializer_value = Some(value);
                }
            }
        }
    }
    let value = initializer_value.unwrap_or_else(|| {
        panic!(
            "expected assignment into resource local, blocks: {:#?}",
            func.body.blocks
        )
    });
    match value {
        Rvalue::Use(Operand::Copy(copy_place)) => {
            assert_ne!(
                copy_place.local.0, resource_index,
                "initializer should come from the call temp"
            );
        }
        other => panic!("expected use of call result, found {other:?}"),
    }

    verify_body(&func.body).require("body verification");
}

#[test]
fn call_expression_statement_emits_call_terminator() {
    let source = r"
namespace Sample;

public void DoWork()
{
}

public void Caller()
{
    DoWork();
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
        .find(|f| f.name == "Sample::Caller")
        .expect("missing Caller function");
    let entry_block = &func.body.blocks[0];
    let terminator = entry_block
        .terminator
        .as_ref()
        .expect("entry block should have call terminator");
    match terminator {
        Terminator::Call {
            func: operand,
            args,
            destination,
            target,
            unwind,
            arg_modes: _,
            dispatch: _,
        } => {
            assert!(
                destination.is_none(),
                "expression statement should not capture result"
            );
            assert_eq!(args.len(), 0, "DoWork takes no arguments");
            assert!(
                unwind.is_none(),
                "call lowering should not inject unwind edge for plain calls"
            );
            match operand {
                Operand::Pending(pending) => {
                    assert!(
                        pending.repr.contains("DoWork"),
                        "expected pending operand referencing DoWork, got {pending:?}"
                    );
                }
                Operand::Const(ConstOperand {
                    value: ConstValue::Symbol(symbol),
                    ..
                }) => {
                    assert!(
                        symbol.ends_with("DoWork"),
                        "expected resolved symbol for DoWork, got {symbol}"
                    );
                }
                other => panic!("expected pending or symbol operand, found {other:?}"),
            }

            let continue_block = func
                .body
                .blocks
                .iter()
                .find(|block| block.id == *target)
                .expect("missing continuation block");
            assert!(
                continue_block.terminator.is_some(),
                "continuation block should finish with return"
            );
        }
        other => panic!("expected call terminator, found {other:?}"),
    }
}

#[test]
fn missing_if_condition_creates_pending_statement() {
    let source = r"
namespace Sample;

public void Check()
{
    if (missing)
    {
        return;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown identifier `missing`")),
        "expected missing identifier diagnostic, found {0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let entry_block = &func.body.blocks[0];
    assert!(
        entry_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::Pending(_))),
        "if lowering should emit pending statement when condition fails"
    );
}

#[test]
fn unsafe_block_emits_enter_and_exit_markers() {
    let source = r"
namespace Sample;

public void Touch()
{
    unsafe
    {
        var value = 1;
    }
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );

    let func = &lowering.module.functions[0];
    let entry_block = &func.body.blocks[0];
    assert!(
        entry_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::EnterUnsafe)),
        "expected EnterUnsafe marker"
    );

    let exit_block = func
        .body
        .blocks
        .iter()
        .find(|block| {
            block
                .statements
                .iter()
                .any(|stmt| matches!(stmt.kind, MirStatementKind::ExitUnsafe))
        })
        .expect("missing block with ExitUnsafe marker");
    assert!(
        exit_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::ExitUnsafe)),
        "expected ExitUnsafe marker in terminal block"
    );

    verify_body(&func.body).require("body verification");
}

#[test]
fn static_member_assignment_remains_pending() {
    let source = r"
namespace Sample;

public void Set()
{
    Foo.Bar = 1;
}
";
    let parsed = parse_module(source).require("parse");
    let lowering = lower_module(&parsed.module);

    let func = &lowering.module.functions[0];
    let entry_block = &func.body.blocks[0];
    let eval = entry_block
        .statements
        .iter()
        .find_map(|stmt| {
            if let MirStatementKind::Eval(pending) = &stmt.kind {
                Some(pending.repr.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            panic!(
                "static-looking member assignment should stay pending, statements: {:#?}",
                entry_block.statements
            )
        });
    assert!(
        eval.contains("Foo.Bar"),
        "expected pending eval to mention Foo.Bar, found {eval}"
    );
    assert!(
        !entry_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. })),
        "no concrete assignment should be emitted for static-looking member"
    );
}
